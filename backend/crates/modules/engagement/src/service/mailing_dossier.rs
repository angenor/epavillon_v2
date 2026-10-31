//! **Écrire au déposant d'un dossier** (01/11), par la rubrique Courriels.
//!
//! Un brouillon rattaché à un dossier est un envoi comme un autre : même
//! fenêtre de rédaction, mêmes gardes (`engagement.mailing.send` sur l'édition
//! du dossier, périmètre d'administration), même historique. Le rattachement
//! ajoute les variables du dossier et range le courriel sur sa fiche.
//!
//! Le dossier se LIT en SQL (`repo/cross.rs`), comme l'audience des déposants
//! le fait déjà : un crate de module n'appelle jamais `programme`.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::dossier::{self, MailingProposal};
use crate::domain::mailing::{
    CreateProposalMailingPayload, Mailing, MailingListScreen, MailingPayload,
};
use crate::domain::render;
use crate::repo::{cross, mailings as repo_mailings};
use crate::service::mailings;
use crate::state::EngagementState;

/// Le dossier d'un envoi et les valeurs de ses variables ; `None` hors dossier
/// (ou dossier supprimé depuis).
pub(crate) async fn dossier_de(
    pool: &PgPool,
    proposal_id: Option<Uuid>,
) -> Result<Option<MailingProposal>> {
    let Some(id) = proposal_id else {
        return Ok(None);
    };
    Ok(cross::dossier_pour_courriel(pool, id).await?.map(|d| {
        let (values, missing) = dossier::valeurs(&d.titre, d.seance.as_ref());
        MailingProposal {
            id: d.id,
            reference_code: d.reference_code,
            title: d.titre,
            values,
            missing,
        }
    }))
}

/// Les valeurs du dossier à figer sur chaque destinataire (objet vide hors
/// dossier). Refuse, en le disant, une variable du dossier citée sans valeur.
/// L'essai du worker passe aussi par ici.
pub(crate) async fn exiger_valeurs(
    pool: &PgPool,
    envoi: &repo_mailings::LigneEnvoi,
) -> Result<serde_json::Value> {
    let mut citees = render::variables_citees(&envoi.subject);
    citees.extend(render::variables_citees(&envoi.body_html));
    let d = dossier_de(pool, envoi.proposal_id).await?;
    dossier::exiger(&citees, d.as_ref()).map_err(|m| ApiError::validation(m, "body_html"))?;
    Ok(match d {
        Some(d) => serde_json::to_value(d.values).map_err(ApiError::internal)?,
        None => serde_json::json!({}),
    })
}

/// Le brouillon d'un dossier : au déposant, en copie la personne à contacter
/// (si c'en est une autre) avec la copie de l'équipe, objet = titre de
/// l'activité. Hors périmètre, le dossier n'existe pas (404).
pub async fn creer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: &CreateProposalMailingPayload,
) -> Result<Mailing> {
    let Some(d) = cross::dossier_pour_courriel(state.pool(), payload.proposal_id).await? else {
        return Err(ApiError::not_found());
    };
    mailings::garder_edition(state, acteur, d.event_id).await?;
    let draft = MailingPayload {
        subject: Some(d.titre.chars().take(250).collect()),
        extra_addresses: Some(vec![d.deposant]),
        ..MailingPayload::default()
    };
    let contact: Vec<String> = d.contact.into_iter().collect();
    mailings::creer_garde(state, ctx, acteur, d.event_id, &draft, &contact, Some(d.id)).await
}

/// L'historique des courriels d'un dossier, gardé sur son édition. Une
/// édition nommée qui n'est pas la sienne : il n'y est pas (404).
pub async fn ecran(
    state: &EngagementState,
    acteur: Uuid,
    proposal_id: Uuid,
    event_id: Option<Uuid>,
) -> Result<MailingListScreen> {
    let Some(d) = cross::dossier_pour_courriel(state.pool(), proposal_id).await? else {
        return Err(ApiError::not_found());
    };
    if event_id.is_some_and(|e| e != d.event_id) {
        return Err(ApiError::not_found());
    }
    mailings::garder_edition(state, acteur, d.event_id).await?;
    mailings::ecran_garde(state, d.event_id, Some(d.id)).await
}
