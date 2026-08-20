//! La fusion de deux organisations.
//!
//! **C'est l'opération la plus dangereuse du module : rien ne l'annule d'un
//! clic.** La fiche absorbée survit, mais ses rattachements sont déplacés.
//!
//! ## L'ordre des écritures n'est pas celui qu'on croit
//!
//! ```text
//!   transaction ouverte par la porte d'écriture du noyau (acteur posé)
//!     │
//!     ├─ contrôles : permission globale, nom de confirmation, statuts
//!     │
//!     ├─ org.merge_organizations(source, cible, motif)
//!     │     └─ réaffecte, passe la source en `merged`, écrit le journal,
//!     │        marque la paire, ET ÉMET org.organization.merged
//!     │
//!     ├─ arbitrages de champ sur la fiche SURVIVANTE      ← APRÈS, jamais avant
//!     │
//!     ├─ relecture de rows_reassigned dans le journal
//!     └─ COMMIT
//! ```
//!
//! `docs/progression/api.md` et l'en-tête de
//! `frontend/app/types/admin-organizations.ts` disaient « avant l'appel, dans la
//! même transaction ». **La seconde moitié était juste, la première ne l'était
//! pas** : `ux_organizations_name_country` ne porte que sur les fiches
//! **vivantes**, et tant que la fiche absorbée l'est encore, la survivante ne
//! peut pas reprendre son nom légal — qui est le champ le plus souvent arbitré
//! (research.md § R5, écart n° 70).
//!
//! La garantie que cherchait l'obligation d'A11 est **conservée intacte** : si
//! un arbitrage échoue, la fusion est annulée avec lui. Seul l'ordre change.
//!
//! **Effet de bord voulu** : modifier le nom légal de la survivante fait entrer
//! l'ancien nom de la fiche absorbée dans ses dénominations, par le trigger du
//! modèle. Une recherche sur l'ancien nom continue donc de trouver la bonne
//! fiche, ce qui est la promesse de la fusion.

use kernel::auth::Scope;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::duplicates::DuplicateSide;
use crate::domain::ids::{OrganizationId, PersonId};
use crate::domain::merge::{
    MergeFieldComparison, MergeOutcome, MergePayload, MergePreview, MergeSide, MergeWarning,
    AVERT_CIBLE_NON_VERIFIEE, AVERT_PAYS_DIFFERENTS, AVERT_SOURCE_DOMAINE_VERIFIE,
    AVERT_SOURCE_PLUS_ACTIVE, AVERT_SOURCE_VERIFIEE, AVERT_TYPES_DIFFERENTS, CHAMP_NON_ARBITRABLE,
    MERGE_FIELDS,
};
use crate::domain::permissions::ORGANIZATION_MERGE;
use crate::jobs;
use crate::repo::{duplicates, merge, merge_counts};
use crate::state::OrgState;

/// L'aperçu, **pour un sens donné**.
///
/// Il est recalculé à l'inversion : le décompte n'est pas symétrique. Trois
/// adhésions transférées dans un sens peuvent en faire cinq dans l'autre, selon
/// ce que chaque fiche porte déjà.
pub async fn preview(
    state: &OrgState,
    acteur: PersonId,
    source: OrganizationId,
    cible: OrganizationId,
    pair_id: Option<Uuid>,
) -> Result<Option<MergePreview>> {
    exiger_la_portee_globale(state, acteur).await?;

    let mut conn = state.pool().acquire().await?;

    let cotes = duplicates::cotes_de(&mut conn, &[source.as_uuid(), cible.as_uuid()]).await?;
    let (Some(fiche_source), Some(fiche_cible)) = (
        cotes.iter().find(|c| c.organization_id == source).cloned(),
        cotes.iter().find(|c| c.organization_id == cible).cloned(),
    ) else {
        return Ok(None);
    };

    // Une fiche déjà absorbée ne se fusionne pas : le trigger l'interdirait de
    // toute façon, et l'écran n'a pas à proposer un geste voué au refus.
    if fiche_cible.status == "merged" {
        return Ok(None);
    }

    let comparisons = comparer(&mut conn, source, cible).await?;
    let transfers = merge_counts::chiffrer(&mut conn, source.as_uuid(), cible.as_uuid()).await?;
    let transferred_names = merge::denominations_apportees(&mut conn, source, cible).await?;
    let transferred_domains = merge::domaines_apportes(&mut conn, source, cible).await?;
    let warnings = avertir(&fiche_source, &fiche_cible, &transferred_domains);

    Ok(Some(MergePreview {
        source: Box::new(fiche_source),
        target: Box::new(fiche_cible),
        pair_id,
        comparisons,
        transfers,
        transferred_names,
        transferred_domains,
        warnings,
    }))
}

/// La fusion elle-même.
pub async fn merge(
    state: &OrgState,
    ctx: &RequestContext,
    acteur: PersonId,
    demande: MergePayload,
) -> Result<MergeOutcome> {
    exiger_la_portee_globale(state, acteur).await?;

    let source = OrganizationId(demande.source_id);
    let cible = OrganizationId(demande.target_id);

    if source == cible {
        return Err(ApiError::new(ErrorCode::OrgMergeSameOrganization).field("target_id"));
    }

    // **Le refus du champ non arbitrable vient avant tout le reste** : il ne
    // dépend d'aucune lecture, et le dire tôt évite d'ouvrir une transaction
    // pour rien.
    if demande.field_choices.get(CHAMP_NON_ARBITRABLE) == Some(&MergeSide::Source) {
        return Err(
            ApiError::new(ErrorCode::OrgMergeFieldNotArbitrable).field(CHAMP_NON_ARBITRABLE)
        );
    }

    let mut tx = state.db().write(ctx).await?;

    // 1. Contrôles.
    let Some(champs_source) = merge::champs_comparables(&mut tx, source).await? else {
        return Ok(MergeOutcome::NotFound);
    };
    if merge::champs_comparables(&mut tx, cible).await?.is_none() {
        return Ok(MergeOutcome::NotFound);
    }

    // 2. **Le nom de confirmation, revérifié.** Masquer un bouton n'a jamais
    //    empêché une requête.
    if !merge::nom_de_confirmation_valide(&mut tx, source, &demande.confirmation_name).await? {
        return Ok(MergeOutcome::ConfirmationMismatch);
    }

    // 3. L'appel de la fonction du modèle.
    //
    //    **Elle émet elle-même `org.organization.merged` et marque elle-même la
    //    paire de la file.** C'est le piège n° 1 du module `identity`, répété à
    //    l'identique : un service qui émettrait l'événement après l'appel en
    //    écrirait deux, SANS QU'AUCUNE ERREUR NE LE SIGNALE — l'outbox accepte
    //    les deux, et un consommateur idempotent traiterait la première ligne
    //    puis ignorerait la mauvaise.
    //
    //    ⚠️ C'EST ICI QU'ON SERAIT TENTÉ D'AJOUTER `events::emit(…)` ET
    //    `duplicates::trancher(…)`. NE PAS LE FAIRE. Lire
    //    `docs/database/040_organizations.sql` § 6 : la fonction fait les deux
    //    avant de rendre la main.
    let fusionnee = match merge::fusionner(&mut tx, source, cible, &demande.reason).await {
        Ok(id) => id,
        Err(e) => {
            // La transaction est abandonnée par le refus : on la rend avant de
            // relire quoi que ce soit.
            tx.rollback().await?;
            return traduire_le_refus_de_la_base(state, e, cible).await;
        }
    };

    // 4. **PUIS** les arbitrages de champ, sur la fiche survivante.
    //
    //    Inverser cet ordre fait échouer toute fusion arbitrant le nom légal :
    //    tant que la fiche absorbée est vivante, l'unicité du nom interdit à la
    //    survivante de reprendre le sien.
    let fields_applied = appliquer_les_arbitrages(&mut tx, cible, &demande, &champs_source).await?;

    // 5. Le décompte réel, relu dans le journal que l'étape 3 a écrit.
    let rows_reassigned = merge::decompte_reel(&mut tx, source, cible).await?;

    jobs::planifier_apres_ecriture(&mut tx, state.config(), cible).await?;
    tx.commit().await?;

    Ok(MergeOutcome::Merged {
        target: fusionnee,
        rows_reassigned,
        fields_applied,
    })
}

// -----------------------------------------------------------------------------

/// La fusion exige la permission **en portée globale**.
///
/// Il n'existe pas de fusion limitée à une édition : fusionner deux fiches
/// déplace des rattachements dans toutes les éditions, y compris celles qu'on
/// n'administre pas. Le code est distinct de `FORBIDDEN` parce que l'écran sait
/// dire **pourquoi**.
async fn exiger_la_portee_globale(state: &OrgState, acteur: PersonId) -> Result<()> {
    let autorise =
        kernel::auth::has_permission(state.pool(), acteur.0, ORGANIZATION_MERGE, Scope::Global)
            .await?;

    if autorise {
        Ok(())
    } else {
        Err(ApiError::new(ErrorCode::OrgMergeGlobalScopeRequired))
    }
}

async fn comparer(
    conn: &mut PgConnection,
    source: OrganizationId,
    cible: OrganizationId,
) -> Result<Vec<MergeFieldComparison>> {
    let (Some(a), Some(b)) = (
        merge::champs_comparables(conn, source).await?,
        merge::champs_comparables(conn, cible).await?,
    ) else {
        return Ok(Vec::new());
    };

    Ok(MERGE_FIELDS
        .iter()
        .map(|champ| {
            MergeFieldComparison::nouvelle(
                champ,
                a.get(*champ).cloned().unwrap_or(Value::Null),
                b.get(*champ).cloned().unwrap_or(Value::Null),
            )
        })
        .collect())
}

/// Les six avertissements. **Non bloquants** : l'écran ne décide pas à la place
/// de l'équipe.
fn avertir(
    source: &DuplicateSide,
    cible: &DuplicateSide,
    domaines: &[crate::domain::merge::TransferredDomain],
) -> Vec<MergeWarning> {
    let mut warnings = Vec::new();

    // **Le plus important, et donc le premier** : absorber une fiche portant le
    // sceau dans une fiche qui ne l'a pas fait perdre la vérification, et
    // personne ne s'en aperçoit avant qu'un public la cherche.
    if source.verified_at.is_some() && cible.verified_at.is_none() {
        warnings.push(MergeWarning::avec(
            AVERT_SOURCE_VERIFIEE,
            &[("source", source.legal_name.clone())],
        ));
    }

    let activite_source = source.proposal_count + source.session_count + source.member_count;
    let activite_cible = cible.proposal_count + cible.session_count + cible.member_count;
    if activite_source > activite_cible {
        warnings.push(MergeWarning::avec(
            AVERT_SOURCE_PLUS_ACTIVE,
            &[
                ("source", activite_source.to_string()),
                ("target", activite_cible.to_string()),
            ],
        ));
    }

    if domaines
        .iter()
        .any(|d| d.verified_at.is_some() && !d.already_present)
    {
        warnings.push(MergeWarning::simple(AVERT_SOURCE_DOMAINE_VERIFIE));
    }

    if cible.verified_at.is_none() {
        warnings.push(MergeWarning::simple(AVERT_CIBLE_NON_VERIFIEE));
    }

    if source.country_id != cible.country_id {
        warnings.push(MergeWarning::simple(AVERT_PAYS_DIFFERENTS));
    }

    if source.organization_type_code != cible.organization_type_code {
        warnings.push(MergeWarning::avec(
            AVERT_TYPES_DIFFERENTS,
            &[
                ("source", source.organization_type_code.clone()),
                ("target", cible.organization_type_code.clone()),
            ],
        ));
    }

    warnings
}

/// Applique les choix de l'opérateur sur la fiche survivante.
///
/// **Un champ absent du dictionnaire garde la valeur de la cible** : c'est elle
/// qui survit, et l'absence de choix ne doit rien écraser.
async fn appliquer_les_arbitrages(
    tx: &mut PgConnection,
    cible: OrganizationId,
    demande: &MergePayload,
    champs_source: &Value,
) -> Result<Vec<String>> {
    let mut appliques = Vec::new();

    for champ in MERGE_FIELDS {
        if demande.field_choices.get(*champ) != Some(&MergeSide::Source) {
            continue;
        }
        // Le refus a déjà été rendu en amont ; la garde reste, parce qu'un
        // second appelant ne le saurait pas.
        if *champ == CHAMP_NON_ARBITRABLE {
            continue;
        }

        let valeur = champs_source.get(*champ).cloned().unwrap_or(Value::Null);
        if merge::arbitrer(tx, cible, champ, &valeur).await? {
            appliques.push((*champ).to_owned());
        }
    }

    Ok(appliques)
}

/// Les trois exceptions que `040_organizations.sql` lève, traduites.
///
/// Leurs SQLSTATE ont été **relevés sur la base**, jamais recopiés d'un
/// document : `integrity_constraint_violation` vaut 23000,
/// `invalid_parameter_value` 22023, `no_data_found` P0002. B1 a payé une fois
/// d'avoir fait l'inverse, et une adresse mal écrite sortait en 500. Voir
/// `kernel::pg_error`.
///
/// **Le message de la base est rendu tel quel** dans les deux cas où il sort. Le
/// reformuler produirait un second libellé pour un même refus, et le second se
/// périmerait à la première évolution du modèle.
///
/// « Introuvable » et « déjà fusionnée » sortent du **même** refus de la
/// fonction : c'est ici qu'on les sépare, en relisant la cible. Le contrat les
/// distingue parce que l'écran n'a pas la même suite à proposer — sur une fiche
/// absorbée, il renvoie vers la fiche finale.
async fn traduire_le_refus_de_la_base(
    state: &OrgState,
    erreur: ApiError,
    cible: OrganizationId,
) -> Result<MergeOutcome> {
    match erreur.code {
        // `tg_forbid_merge_chains`, quand il est atteint : le message porte
        // « Cibler la fiche finale ».
        ErrorCode::Conflict => Ok(MergeOutcome::AlreadyMerged {
            target: fiche_finale(state, cible).await?,
            message: erreur.message,
        }),
        ErrorCode::NotFound => {
            // La fonction refuse « introuvable OU déjà fusionnée » d'un seul
            // message. Une cible absorbée existe : on la reconnaît en la
            // relisant, et on rend le renvoi que l'écran attend.
            match crate::repo::organizations::by_id(state.pool(), cible).await? {
                Some(fiche) if fiche.merged_into_id.is_some() => Ok(MergeOutcome::AlreadyMerged {
                    target: fiche.merged_into_id,
                    message: erreur.message,
                }),
                _ => Ok(MergeOutcome::NotFound),
            }
        }
        _ => Err(erreur),
    }
}

/// La fiche vivante derrière une fiche absorbée — celle que l'écran doit viser.
async fn fiche_finale(state: &OrgState, cible: OrganizationId) -> Result<Option<OrganizationId>> {
    Ok(crate::repo::organizations::by_id(state.pool(), cible)
        .await?
        .and_then(|f| f.merged_into_id))
}
