//! La liste du comité — **tout l'écran en une réponse**, et les deux actions
//! groupées qui partent de la sélection.
//!
//! # Une composition, pas six lectures (R16)
//!
//! Les lignes sont lues **une fois**, et les sept facettes se comptent dessus.
//! Le contrat du front en donne la raison : « demandées à part, elles seraient
//! mesurées à un autre instant, et le "Retenu (17)" du filtre finirait par ne
//! plus correspondre aux lignes affichées ».
//!
//! **Une seule exception** : les dossiers non lus viennent de
//! `programme.unread_proposals_for()`, qui prend le lecteur en paramètre. Ce
//! n'est pas une facette mais une relation entre un dossier et une personne.
//!
//! # Le périmètre est vérifié DEUX fois, et les deux comptent
//!
//! L'extracteur de route refuse un périmètre **vide** ; ce service le refuse
//! aussi, parce que ses tests l'appellent sans passer par une route et qu'une
//! garde qui ne vit que dans la couche HTTP n'en est pas une. **Un périmètre
//! vide est un refus explicite, jamais une liste vide** : les confondre
//! afficherait « rien à traiter » à qui n'a aucun droit (principe V).
//!
//! Puis l'édition demandée est bornée. **Une édition hors périmètre et une
//! édition inexistante rendent le même refus** : sans cela, une URL forgée
//! dirait à qui la forge quelles éditions existent.
//!
//! # Ni pagination, ni tri, ni filtre côté serveur (R17)
//!
//! Le contrat le dit : le filtrage et le tri restent à l'écran, et « ces
//! paramètres deviendront ceux de la requête » au raccordement (B7). Les
//! livrer maintenant produirait une surface que personne n'appelle et deux
//! implémentations du tri à réconcilier.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self, DomainEvent};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::facets::{self, Compteur, ProposalFacet, ProposalFacets};
use crate::domain::ids::{EventId, ProposalId};
use crate::domain::permissions::CALL_MANAGE;
use crate::repo::dashboard::LigneDePilotage;
use crate::repo::{assignments, cross, dashboard, proposals, reads};
use crate::service::transition::{Ecart, RaisonDEcart, ResultatGroupe};
use crate::state::ProgrammeState;

/// `ProposalListScreen` — les lignes, les facettes, les non-lus, et le fuseau
/// dans lequel toutes les dates de cet écran se lisent.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EcranDeListe {
    pub event_id: Uuid,
    /// `event.events.timezone` : **jamais celui du navigateur**. Une échéance
    /// de revue annoncée à 23 h 59 l'est à Belém, pas à Québec.
    pub timezone: String,
    /// Le nom de ville **nomme** le fuseau à l'écran — « heure de Belém ».
    pub city: Option<String>,
    /// `event.effective_deadline()`, **prolongation comprise**. Nulle quand
    /// l'édition n'ouvre aucun appel.
    #[serde(with = "time::serde::rfc3339::option")]
    pub deadline: Option<OffsetDateTime>,
    /// Le dénominateur du « 2/3 ». Nul hors appel.
    pub required_reviews: Option<i16>,
    pub rows: Vec<LigneDePilotage>,
    pub facets: ProposalFacets,
    pub unread_ids: Vec<Uuid>,
}

/// Tout l'écran, en une réponse.
pub async fn ecran(
    state: &ProgrammeState,
    perimetre: &kernel::auth::Perimeter,
    edition: EventId,
) -> Result<EcranDeListe> {
    let contexte = edition_bornee(state, perimetre, edition).await?;

    let rows = dashboard::lignes(state.pool(), edition).await?;
    let unread_ids = reads::non_lus(state.pool(), perimetre.person_id, edition).await?;

    let appel = cross::appel_de_ledition(state.pool(), edition).await?;
    let (deadline, required_reviews) = match appel {
        Some(call_id) => (
            cross::echeance_effective(state.pool(), call_id).await?,
            cross::regles_de_lappel(state.pool(), call_id)
                .await?
                .map(|r| r.required_reviews),
        ),
        None => (None, None),
    };

    let facets = facettes(&rows, &unread_ids);

    Ok(EcranDeListe {
        event_id: contexte.event_id,
        timezone: contexte.timezone,
        city: contexte.city,
        deadline,
        required_reviews,
        rows,
        facets,
        unread_ids,
    })
}

/// Les lignes seules — `GET /proposals/dashboard`.
///
/// Elle existe parce qu'un écran qui n'affiche que le tableau n'a pas besoin
/// des facettes ni des non-lus, et que la vue est déjà composée.
pub async fn lignes(
    state: &ProgrammeState,
    perimetre: &kernel::auth::Perimeter,
    edition: EventId,
) -> Result<Vec<LigneDePilotage>> {
    edition_bornee(state, perimetre, edition).await?;
    dashboard::lignes(state.pool(), edition).await
}

/// La composition du comité de l'appel, **chacun avec sa charge** — rendue
/// sous la forme d'une facette, comme le contrat l'attend : la valeur est la
/// personne, le libellé son nom, le décompte sa charge.
///
/// Une édition sans appel rend une liste vide, et non un refus : il n'y a
/// alors aucun comité, ce qui est un fait et non une erreur.
pub async fn comite(
    state: &ProgrammeState,
    perimetre: &kernel::auth::Perimeter,
    edition: EventId,
) -> Result<Vec<ProposalFacet>> {
    edition_bornee(state, perimetre, edition).await?;

    let Some(call_id) = cross::appel_de_ledition(state.pool(), edition).await? else {
        return Ok(Vec::new());
    };

    Ok(cross::charges_du_comite(state.pool(), call_id)
        .await?
        .into_iter()
        .map(|membre| ProposalFacet {
            value: membre.person_id.to_string(),
            label: Some(serde_json::Value::String(membre.display_name)),
            count: membre.charge,
            color: None,
        })
        .collect())
}

/// **Résoudre l'édition, périmètre d'abord.**
///
/// Trois refus, et deux d'entre eux se ressemblent volontairement : périmètre
/// vide → 403 ; édition hors périmètre → 404 ; édition inexistante → 404.
async fn edition_bornee(
    state: &ProgrammeState,
    perimetre: &kernel::auth::Perimeter,
    edition: EventId,
) -> Result<cross::ContexteEdition> {
    if perimetre.scope.is_empty() {
        return Err(ApiError::forbidden());
    }
    perimetre.ensure(edition.as_uuid())?;

    cross::contexte_edition(state.pool(), edition)
        .await?
        .ok_or_else(ApiError::not_found)
}

// -----------------------------------------------------------------------------
// Les sept facettes, comptées sur les lignes déjà lues
// -----------------------------------------------------------------------------

/// Ce que porte un libellé : la **base** quand la valeur y est nommée —
/// thématique, pays, organisation, personne —, rien quand c'est un code
/// d'énumération que l'écran traduit lui-même.
fn facettes(lignes: &[LigneDePilotage], non_lus: &[Uuid]) -> ProposalFacets {
    let mut statuts = Compteur::new();
    let mut formats = Compteur::new();
    let mut themes = Compteur::new();
    let mut pays = Compteur::new();
    let mut organisations = Compteur::new();
    let mut noteurs = Compteur::new();

    let (mut non_evalues, mut en_retard, mut jamais_ouverts) = (0, 0, 0);

    for ligne in lignes {
        statuts.ajouter_code(&ligne.status);
        formats.ajouter_code(&ligne.format);

        for code in &ligne.theme_codes {
            let (label, couleur) = pastille(&ligne.themes, code);
            themes.ajouter(code, label, couleur);
        }

        if let Some(code) = &ligne.organization_country_code {
            pays.ajouter(code, ligne.organization_country.clone(), None);
        }

        organisations.ajouter(
            &ligne.organization_id.to_string(),
            Some(serde_json::Value::String(ligne.organization_name.clone())),
            None,
        );

        for membre in &ligne.reviewer_ids {
            noteurs.ajouter(
                &membre.to_string(),
                nom_du_noteur(&ligne.reviewers, membre),
                None,
            );
        }

        // « Non évaluée » ne veut pas dire « sans membre du comité » : un
        // dossier confié à trois personnes dont aucune n'a rendu sa note est
        // non évalué. Le brouillon en est exclu — il n'a jamais été soumis.
        if ligne.review_count == 0 && ligne.status != "draft" {
            non_evalues += 1;
        }
        if ligne.overdue_reviews > 0 {
            en_retard += 1;
        }
        if non_lus.contains(&ligne.id) {
            jamais_ouverts += 1;
        }
    }

    ProposalFacets {
        statuses: facets::selon(statuts.rendre(), &facets::ORDRE_DES_STATUTS),
        themes: facets::par_compte_decroissant(themes.rendre()),
        formats: facets::selon(formats.rendre(), &facets::ORDRE_DES_FORMATS),
        countries: facets::par_compte_decroissant(pays.rendre()),
        organizations: facets::par_compte_decroissant(organisations.rendre()),
        reviewers: facets::par_compte_decroissant(noteurs.rendre()),
        // Les trois signaux sont **toujours rendus**, fût-ce à zéro : leur
        // absence laisserait croire que le filtre n'existe pas.
        flags: vec![
            drapeau(facets::FLAG_UNREVIEWED, non_evalues),
            drapeau(facets::FLAG_LATE, en_retard),
            drapeau(facets::FLAG_UNREAD, jamais_ouverts),
        ],
    }
}

fn drapeau(code: &str, compte: i64) -> ProposalFacet {
    ProposalFacet {
        value: code.to_owned(),
        label: None,
        count: compte,
        color: None,
    }
}

/// Le libellé et la couleur d'une thématique, **pris sur la ligne**.
///
/// La vue les porte déjà résolus par `reference.term_badges()`. Recharger la
/// taxonomie serait refaire ce que la base a fait, avec un risque de
/// divergence — et c'est ainsi que les libellés se sont retrouvés figés dans
/// le frontend de la v1.
fn pastille(
    pastilles: &serde_json::Value,
    code: &str,
) -> (Option<serde_json::Value>, Option<String>) {
    let Some(pastille) = pastilles.as_array().and_then(|badges| {
        badges
            .iter()
            .find(|b| b.get("code").and_then(|c| c.as_str()) == Some(code))
    }) else {
        return (None, None);
    };

    (
        pastille.get("label").cloned(),
        pastille
            .get("color")
            .and_then(|c| c.as_str())
            .map(str::to_owned),
    )
}

/// Le nom d'un membre du comité, pris sur la ligne : un « 2/3 » ne dit pas de
/// qui on attend la troisième revue.
fn nom_du_noteur(noteurs: &serde_json::Value, membre: &Uuid) -> Option<serde_json::Value> {
    let attendu = membre.to_string();
    noteurs
        .as_array()?
        .iter()
        .find(|n| n.get("person_id").and_then(|p| p.as_str()) == Some(attendu.as_str()))
        .and_then(|n| n.get("name").cloned())
}

// -----------------------------------------------------------------------------
// L'affectation groupée
// -----------------------------------------------------------------------------

/// `AssignReviewerPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AssignReviewerPayload {
    pub proposal_ids: Vec<Uuid>,
    pub reviewer_id: Uuid,
    /// Échéance commune, dans le fuseau de l'édition. **Nullable** : un comité
    /// peut confier sans dater.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub due_at: Option<OffsetDateTime>,
}

/// Confier une sélection à un membre du comité.
///
/// # L'autorisation est évaluée DOSSIER PAR DOSSIER
///
/// `event.call.manage` sur **l'édition du dossier**, et le périmètre sur la
/// même édition. Une sélection de douze peut traverser deux éditions :
/// vérifier une fois pour le lot accorderait sur l'une ce qui n'est permis que
/// sur l'autre.
///
/// **La permission garde l'affectation, et c'est une permission d'un autre
/// module** : composer le comité et répartir sa charge sont le même geste,
/// celui de qui tient la campagne (écart n° 48). Une permission est une chaîne
/// lue en base, pas un symbole d'un autre crate.
///
/// # Trois écarts, chacun nommé
///
/// Déjà confié, déporté, introuvable. **Un dossier hors périmètre rend le même
/// écart qu'un dossier inexistant** : le refus ne dit pas à qui forge une
/// sélection que le dossier existe ailleurs.
///
/// # Un événement PAR DOSSIER
///
/// Douze dossiers confiés émettent douze `programme.review.assigned`, jamais
/// un pour le lot : un consommateur qui reçoit un lot doit le déplier
/// lui-même, et son échec porte alors sur douze effets au lieu d'un. La garde
/// de rejeu est par événement.
///
/// **Et c'est le seul événement que ce geste émet** : aucun état ne change
/// ici, donc le déclencheur d'état ne s'éveille pas.
pub async fn confier_en_groupe(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &kernel::auth::Perimeter,
    payload: AssignReviewerPayload,
) -> Result<ResultatGroupe> {
    if perimetre.scope.is_empty() {
        return Err(ApiError::forbidden());
    }

    // **La personne visée est vérifiée une fois, pas douze.** La laisser au
    // hasard de la clé étrangère ferait échouer le premier dossier après en
    // avoir confié aucun, avec un refus qui ne nomme pas le champ fautif.
    if cross::fiches_personnes(state.pool(), &[payload.reviewer_id])
        .await?
        .is_empty()
    {
        return Err(ApiError::with_message(
            kernel::error::ErrorCode::ProposalUnknownReference,
            "Cette personne est inconnue.",
        )
        .field("reviewer_id"));
    }

    let mut resultat = ResultatGroupe::default();

    for id in payload.proposal_ids {
        let dossier = ProposalId(id);

        let Some(etat) = proposals::etat(state.pool(), dossier).await? else {
            resultat.skipped.push(introuvable(id));
            continue;
        };

        if !perimetre.allows(etat.event_id) {
            resultat.skipped.push(introuvable(id));
            continue;
        }

        let autorise = kernel::auth::has_permission(
            state.pool(),
            perimetre.person_id,
            CALL_MANAGE,
            kernel::auth::Scope::Event(etat.event_id),
        )
        .await?;
        if !autorise {
            resultat.skipped.push(introuvable(id));
            continue;
        }

        // Le déport n'est pas une suppression : réattribuer le dossier
        // effacerait une déclaration d'impartialité.
        if let Some(deja) =
            assignments::affectation(state.pool(), dossier, payload.reviewer_id).await?
        {
            resultat.skipped.push(Ecart {
                proposal_id: id,
                reference_code: etat.reference_code,
                reason: if deja.recused_at.is_some() {
                    RaisonDEcart::Recused
                } else {
                    RaisonDEcart::AlreadyAssigned
                },
            });
            continue;
        }

        let mut tx = state.db().write(ctx).await?;
        let confiee = assignments::confier(
            &mut tx,
            dossier,
            payload.reviewer_id,
            perimetre.person_id,
            payload.due_at,
        )
        .await?;

        let Some(confiee) = confiee else {
            // Deux actions groupées se sont croisées entre la lecture et
            // l'écriture. La contrainte a tranché ; l'écart le dit.
            tx.rollback().await?;
            resultat.skipped.push(Ecart {
                proposal_id: id,
                reference_code: etat.reference_code,
                reason: RaisonDEcart::AlreadyAssigned,
            });
            continue;
        };

        events::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: contracts::programme::AGGREGATE_SCHEMA,
                aggregate_type: contracts::programme::AGGREGATE_PROPOSAL,
                aggregate_id: id,
                event_type: contracts::programme::REVIEW_ASSIGNED,
                payload: serde_json::to_value(contracts::programme::ReviewAssigned {
                    proposal_id: id,
                    reference_code: etat.reference_code.clone(),
                    event_id: etat.event_id,
                    reviewer_id: confiee.reviewer_id,
                    due_at: confiee.due_at,
                })
                .map_err(ApiError::internal)?,
            },
        )
        .await?;

        tx.commit().await?;
        resultat.applied.push(id);
    }

    Ok(resultat)
}

/// Sans numéro de dossier à rendre — il n'existe pas, ou pas pour ce lecteur —,
/// on rend l'identifiant demandé : c'est ce que l'écran a en main.
fn introuvable(id: Uuid) -> Ecart {
    Ecart {
        proposal_id: id,
        reference_code: String::new(),
        reason: RaisonDEcart::NotFound,
    }
}
