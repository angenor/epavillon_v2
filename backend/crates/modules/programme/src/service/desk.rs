//! La fiche d'évaluation — **onze lectures, une transaction, une connexion**.
//!
//! # Cette lecture ÉCRIT, et c'est assumé
//!
//! Ouvrir la fiche pose l'accusé de lecture (`record_proposal_read`). B3
//! composait ses six onglets en transaction **lecture seule**, et c'était juste
//! — il ne s'y écrivait rien. Ici, une écriture a lieu, et le principe VII
//! exige que toute écriture pose l'acteur et l'identifiant de requête. La
//! composition passe donc par la **porte d'écriture** du noyau.
//!
//! La déguiser en deux appels — composer en lecture seule, puis poser l'accusé
//! ailleurs — coûterait deux allers-retours, retiendrait deux connexions du
//! pool (la leçon de B2), et laisserait un accusé manquer alors que la page
//! s'est affichée.
//!
//! **L'état « déjà ouvert » est lu AVANT l'appel qui le pose.** La fonction du
//! modèle insère ou incrémente sans distinguer : lue après, elle dirait
//! toujours « déjà vu », et l'écran ne pourrait plus signaler un dossier qu'on
//! découvre.
//!
//! # 🔴 Le voile n'est pas un filtre : ce qui est masqué N'EST PAS LU
//!
//! Quand le voile est baissé, `reviews::des_pairs()` **n'est pas appelée**. Le
//! décompte, lui, l'est — une requête d'agrégat qui ne rend aucun texte.
//!
//! Lire puis vider les champs sensibles laisse la donnée à portée d'un champ
//! oublié dans un type de sortie, d'une trace de débogage, d'un message
//! d'erreur enrichi. C'est le patron qui a produit, en v1, des notes internes
//! visibles dans une réponse JSON que l'écran n'affichait pas. **Ne pas lire
//! supprime la classe entière de défauts** — et c'est ce qui rend le test
//! possible : on inspecte la charge utile, pas l'écran.
//!
//! # Ce que la composition ne recalcule pas
//!
//! Le **rang** vient de la vue de pilotage, la **note pondérée maximale** de la
//! fonction de l'appel, les **thématiques** de `term_badges()`, l'**historique**
//! de la fonction du module. Chacune de ces valeurs existe déjà ; en refaire
//! une seule ici créerait deux définitions du même nombre, et elles finiraient
//! par diverger entre la liste et la fiche.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use std::collections::{BTreeMap, HashSet};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::blind::{self, Lecteur};
use crate::domain::desk::{
    self, AvancementDuComite, CritereAffiche, DroitsSurLaFiche, FicheDEvaluation,
    IntervenantDuDossier, MaRevue, OrganisationDuDossier, RevueDUnPair,
};
use crate::domain::ids::ProposalId;
use crate::domain::permissions::{CALL_MANAGE, PROPOSAL_DECIDE, REVIEW_WRITE};
use crate::repo::{
    assignments, comments, cross, dashboard, documents, organizations, proposals, reads, reviews,
    scores, speakers, transitions,
};
use crate::service::perimeter;
use crate::state::ProgrammeState;

/// Composer la fiche, **après contrôle du périmètre**.
///
/// L'ordre est celui de R13 : résoudre l'édition du dossier, vérifier le
/// périmètre, **puis** composer. Un dossier hors périmètre se refuse comme un
/// inexistant.
pub async fn ouvrir(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &kernel::auth::Perimeter,
    dossier: ProposalId,
) -> Result<FicheDEvaluation> {
    if perimetre.scope.is_empty() {
        return Err(ApiError::forbidden());
    }
    let lecteur = perimetre.person_id;
    let edition = perimeter::edition_dans_le_perimetre(
        state.pool(),
        perimetre,
        perimeter::Cible::Dossier(dossier),
    )
    .await?;

    let mut tx = state.db().write(ctx).await?;

    // ---- Le dossier, son édition, son appel ---------------------------------
    let proposal = proposals::fiche(&mut *tx, dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let fiche_edition = cross::fiche_edition(&mut *tx, edition)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let call = match proposal.call_id {
        Some(id) => cross::fiche_appel(&mut *tx, id).await?,
        None => None,
    };

    // ---- L'état d'AVANT la visite, puis l'accusé ----------------------------
    let deja_lu = reads::deja_lu(&mut *tx, dossier, lecteur).await?;
    reads::poser_accuse(&mut tx, dossier, lecteur).await?;
    let read_count = reads::compter_lecteurs(&mut *tx, dossier).await?;

    // ---- Le dossier, colonne de gauche --------------------------------------
    let liens = organizations::du_dossier(&mut *tx, dossier).await?;
    let intervenants = speakers::du_dossier(&mut *tx, dossier).await?;
    let documents = documents::du_dossier(&mut *tx, dossier).await?;
    let themes = cross::pastilles_du_dossier(&mut *tx, dossier).await?;
    let journal = transitions::journal(&mut *tx, dossier).await?;
    let history = cross::historique_du_dossier(&mut *tx, dossier).await?;

    let ids_organisations: Vec<Uuid> = liens.iter().map(|l| l.organization_id).collect();
    let fiches_organisations = cross::organisations_affichees(&mut *tx, &ids_organisations).await?;
    let anteriors = cross::anteriors_affiches(&mut *tx, &ids_organisations).await?;

    // ---- La grille, et l'affectation du lecteur -----------------------------
    let (criteria, max_weighted_score, required_reviews, blind_review) = match proposal.call_id {
        Some(id) => (
            cross::grille_de_lappel(&mut *tx, id)
                .await?
                .into_iter()
                .map(|c| CritereAffiche::depuis(c, id))
                .collect(),
            cross::note_pondere_maximale(&mut *tx, id).await?,
            call.as_ref().map(|c| c.required_reviews),
            call.as_ref().is_some_and(|c| c.blind_review),
        ),
        // Un dossier hors appel n'a ni grille ni aveugle : l'IFDD l'a créé
        // directement, et il n'y a personne à ancrer.
        None => (Vec::new(), 0.0, None, false),
    };

    let mon_affectation = assignments::affectation(&mut *tx, dossier, lecteur).await?;
    let ma_revue = reviews::mienne(&mut *tx, dossier, lecteur).await?;

    // ---- 🔴 Le voile, décidé AVANT toute lecture de revue --------------------
    let voile = blind::voile_baisse(Lecteur {
        appel_en_aveugle: blind_review,
        // Un membre déporté ne posera plus de note : il n'y a rien à ancrer.
        affecte: mon_affectation
            .as_ref()
            .is_some_and(|a| a.recused_at.is_none()),
        revue_deposee: ma_revue.as_ref().is_some_and(|r| r.submitted_at.is_some()),
    });

    let veiled_count = reviews::compter_deposees(&mut *tx, dossier, lecteur).await?;

    // **La requête n'est pas exécutée quand le voile est baissé.** C'est la
    // seule forme que peut prendre R4 : un `if` autour de l'appel, et non un
    // filtre sur son résultat.
    let revues_des_pairs = if voile {
        Vec::new()
    } else {
        reviews::des_pairs(&mut *tx, dossier, lecteur).await?
    };

    let ids_revues: Vec<Uuid> = revues_des_pairs.iter().map(|r| r.id).collect();
    let notes_des_pairs = scores::des_revues(&mut *tx, &ids_revues).await?;
    let mes_notes = match &ma_revue {
        Some(revue) => scores::de_la_revue(&mut *tx, revue.id).await?,
        None => Vec::new(),
    };

    // ---- L'avancement du comité, les échanges, les personnes ----------------
    let avancement = reviews::avancement_du_comite(&mut *tx, dossier).await?;
    let fil = comments::fil(&mut *tx, dossier, lecteur, comments::Cote::Comite).await?;
    let rank = dashboard::rang(&mut *tx, dossier).await?.unwrap_or(0);

    let ids_personnes = personnes_a_resoudre(&intervenants, &avancement, &fil, &revues_des_pairs);
    let personnes = cross::personnes_affichees(&mut *tx, &ids_personnes).await?;

    // ---- Les droits, et les actions offertes --------------------------------
    let permissions = droits(
        &mut tx,
        lecteur,
        edition.as_uuid(),
        mon_affectation.as_ref(),
    )
    .await?;
    let available_transitions = transitions::offertes(&mut *tx, dossier, lecteur).await?;

    tx.commit().await?;

    let maintenant = OffsetDateTime::now_utc();
    let par_personne: BTreeMap<Uuid, cross::PersonneAffichee> =
        personnes.iter().cloned().map(|p| (p.id, p)).collect();

    Ok(FicheDEvaluation {
        proposal,
        edition: fiche_edition,
        call,
        organizations: liens
            .into_iter()
            .map(|link| OrganisationDuDossier {
                organization: fiches_organisations
                    .iter()
                    .find(|o| o.id == link.organization_id)
                    .cloned(),
                track_record: anteriors
                    .iter()
                    .find(|a| a.organization_id == link.organization_id)
                    .cloned(),
                link,
            })
            .collect(),
        speakers: intervenants
            .into_iter()
            .map(|speaker| IntervenantDuDossier {
                person: par_personne.get(&speaker.person_id).cloned(),
                speaker,
            })
            .collect(),
        documents,
        themes,
        transitions: journal,
        history,
        criteria,
        max_weighted_score,
        required_reviews,
        blind_review,
        blind_veiled: voile,
        veiled_count,
        my_review: MaRevue {
            scores: mes_notes
                .iter()
                .map(|n| (n.criterion_id.to_string(), n.score))
                .collect(),
            comments: mes_notes
                .iter()
                .filter_map(|n| n.comment.clone().map(|c| (n.criterion_id.to_string(), c)))
                .collect(),
            assignment: mon_affectation,
            review: ma_revue,
        },
        peer_reviews: revues_des_pairs
            .into_iter()
            .map(|review| RevueDUnPair {
                scores: notes_des_pairs
                    .iter()
                    .filter(|n| n.review_id == review.id)
                    .cloned()
                    .collect(),
                reviewer: par_personne.get(&review.reviewer_id).cloned(),
                assignment: avancement
                    .iter()
                    .find(|a| a.assignment.reviewer_id == review.reviewer_id)
                    .map(|a| a.assignment.clone()),
                review,
            })
            .collect(),
        committee: avancement
            .into_iter()
            .map(|ligne| AvancementDuComite {
                state: desk::etat_davancement(
                    &ligne.assignment,
                    ligne.review_submitted_at,
                    ligne.review_existe,
                    maintenant,
                ),
                person: par_personne.get(&ligne.assignment.reviewer_id).cloned(),
                submitted_at: ligne.review_submitted_at,
                assignment: ligne.assignment,
            })
            .collect(),
        comments: fil,
        participants: personnes,
        permissions,
        rank,
        first_visit: !deja_lu,
        read_count,
        available_transitions,
    })
}

/// Les personnes à nommer, **dédoublonnées avant la requête**.
///
/// Un même membre du comité est souvent à la fois affecté, auteur d'un message
/// et signataire d'une revue : sans dédoublonnage, la requête reçoit trois fois
/// le même identifiant et la réponse porte trois fois la même fiche.
fn personnes_a_resoudre(
    intervenants: &[speakers::IntervenantLu],
    avancement: &[reviews::LigneDAvancement],
    fil: &[comments::Message],
    revues: &[reviews::Revue],
) -> Vec<Uuid> {
    let mut vus = HashSet::new();
    let mut ids = Vec::new();

    let sources = intervenants
        .iter()
        .map(|i| i.person_id)
        .chain(avancement.iter().map(|a| a.assignment.reviewer_id))
        .chain(fil.iter().map(|m| m.author_id))
        .chain(revues.iter().map(|r| r.reviewer_id));

    for id in sources {
        if vus.insert(id) {
            ids.push(id);
        }
    }

    ids
}

/// Les droits de ce lecteur **sur cette édition**, résolus une fois.
///
/// La portée est celle de l'édition du dossier, jamais la portée globale :
/// c'est ce qui fait qu'un responsable détaché sur un webinaire ne décide pas
/// sur la COP31.
async fn droits(
    conn: &mut sqlx::PgConnection,
    lecteur: Uuid,
    edition: Uuid,
    affectation: Option<&assignments::Affectation>,
) -> Result<DroitsSurLaFiche> {
    let portee = kernel::auth::Scope::Event(edition);
    let ligne = sqlx::query!(
        r#"SELECT identity.has_permission($1, $2, $4::text::identity.scope_type, $5) AS "noter!",
                  identity.has_permission($1, $3, $4::text::identity.scope_type, $5) AS "decider!",
                  identity.has_permission($1, $6, $4::text::identity.scope_type, $5) AS "affecter!""#,
        lecteur,
        REVIEW_WRITE,
        PROPOSAL_DECIDE,
        portee.scope_type().as_str(),
        portee.scope_id(),
        CALL_MANAGE,
    )
    .fetch_one(conn)
    .await?;

    Ok(DroitsSurLaFiche {
        can_review: ligne.noter,
        can_decide: ligne.decider,
        can_assign: ligne.affecter,
        is_assigned: affectation.is_some_and(|a| a.recused_at.is_none()),
        is_recused: affectation.is_some_and(|a| a.recused_at.is_some()),
    })
}
