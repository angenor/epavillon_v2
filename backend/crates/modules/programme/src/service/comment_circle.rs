//! Les fils adressés à des personnes nommées, et le lu / non lu par message
//! (demande du commanditaire, 27/10).
//!
//! # Qui forme le comité d'un dossier
//!
//! Les personnes qui ouvrent le dossier côté back-office : la permission
//! `programme.proposal.read_all` sur l'édition (attribution globale comprise),
//! c'est-à-dire exactement ce que `perimeter::acces_au_dossier` exige, moins
//! celles que la voie de l'organisation capte d'abord. C'est la liste où l'on
//! choisit des destinataires, et le « sur M » des accusés.
//!
//! # Le cercle est tenu en base
//!
//! `programme.comment_in_committee_circle()` dit qui lit quoi ; le fil, le
//! marquage et le compteur de la liste l'appellent tous. Aucune copie ici.
//!
//! # Les avis
//!
//! `programme.comment.addressed` part dans la transaction du message, charge
//! `notification` déjà résolue et courriel demandé : à l'ouverture d'un fil
//! adressé, à chaque réponse du comité dans ce fil (le cercle sauf l'auteur),
//! à l'ajout de personnes (les ajoutées). Un message à tout le comité n'en
//! émet aucun, comme avant.

use std::collections::{HashMap, HashSet};

use contracts::negotiation::{
    Notification, NotificationSubject, NotificationText, WithNotification,
};
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self, DomainEvent};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::{CommentId, EventId, ProposalId};
use crate::domain::permissions::PROPOSAL_READ_ALL;
use crate::repo::comments::{self, CommentPerson, CommentReader, Cote, Message};
use crate::repo::cross;
use crate::service::comments::{cote_de, VISIBILITE_COMITE};
use crate::service::perimeter::{self, Acces};
use crate::state::ProgrammeState;

/// `CommentReceipts`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CommentReceipts {
    /// Personnes du comité qui peuvent lire ce message, auteur exclu.
    pub audience_count: i64,
    pub readers: Vec<CommentReader>,
}

/// `ProposalCommentView` : un `ProposalComment` et ce qu'il est pour son
/// lecteur. Se lit comme le message qu'il enveloppe.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProposalCommentView {
    #[serde(flatten)]
    pub message: Message,
    /// Non vide seulement sur un message qui ouvre un fil adressé.
    pub recipients: Vec<CommentPerson>,
    /// Toujours vrai pour ses propres messages.
    pub is_read: bool,
    /// Seulement pour les messages du lecteur, côté comité.
    pub receipts: Option<CommentReceipts>,
}

impl std::ops::Deref for ProposalCommentView {
    type Target = Message;

    fn deref(&self) -> &Message {
        &self.message
    }
}

/// `AddCommentRecipientsPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddCommentRecipientsPayload {
    pub person_ids: Vec<Uuid>,
}

/// `MarkCommentsReadPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MarkCommentsReadPayload {
    pub comment_ids: Vec<Uuid>,
}

// -----------------------------------------------------------------------------
// Le comité du dossier
// -----------------------------------------------------------------------------

async fn audience_du_dossier(
    state: &ProgrammeState,
    dossier: ProposalId,
    edition: EventId,
) -> Result<Vec<CommentPerson>> {
    let detenteurs = kernel::auth::holders(
        state.pool(),
        PROPOSAL_READ_ALL,
        kernel::auth::Scope::Event(edition.as_uuid()),
    )
    .await?;
    cross::comite_parmi(state.pool(), dossier, &detenteurs).await
}

/// `GET /proposals/{id}/comment-audience`. L'organisation ne lit pas les noms
/// du comité : pour elle, le dossier n'a pas d'audience à montrer.
pub async fn audience(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
) -> Result<Vec<CommentPerson>> {
    let (edition, acces) = perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;
    if acces != Acces::BackOffice {
        return Err(ApiError::not_found());
    }
    audience_du_dossier(state, dossier, edition).await
}

/// Les personnes choisies, vérifiées contre le comité du dossier, sans
/// doublon et sans `exclues` (l'auteur, déjà du cercle).
pub(crate) async fn choisir_dans_le_comite(
    state: &ProgrammeState,
    dossier: ProposalId,
    edition: EventId,
    choisies: &[Uuid],
    exclues: &[Uuid],
) -> Result<Vec<CommentPerson>> {
    let comite = audience_du_dossier(state, dossier, edition).await?;
    let connus: HashSet<Uuid> = comite.iter().map(|p| p.person_id).collect();

    if choisies.iter().any(|id| !connus.contains(id)) {
        return Err(ApiError::validation(
            "Une personne choisie ne fait pas partie du comité de ce dossier.",
            "recipient_ids",
        ));
    }

    let voulues: HashSet<Uuid> = choisies
        .iter()
        .copied()
        .filter(|id| !exclues.contains(id))
        .collect();
    let retenues: Vec<CommentPerson> = comite
        .into_iter()
        .filter(|p| voulues.contains(&p.person_id))
        .collect();

    if retenues.is_empty() {
        return Err(ApiError::validation(
            "Choisissez au moins une autre personne que vous-même.",
            "recipient_ids",
        ));
    }
    Ok(retenues)
}

/// Le fil de ce message s'il est adressé : l'auteur de sa racine et ses
/// destinataires. `None` pour un fil ouvert à tout le comité.
pub(crate) async fn fil_adresse(
    state: &ProgrammeState,
    message: Uuid,
) -> Result<Option<(Uuid, Vec<Uuid>)>> {
    let Some(racine) = comments::racine(state.pool(), message).await? else {
        return Ok(None);
    };
    let destinataires = comments::destinataires(state.pool(), racine).await?;
    if destinataires.is_empty() {
        return Ok(None);
    }
    let auteur = comments::par_id(state.pool(), CommentId(racine))
        .await?
        .map(|m| m.author_id)
        .ok_or_else(ApiError::not_found)?;
    Ok(Some((auteur, destinataires)))
}

// -----------------------------------------------------------------------------
// La vue d'un fil
// -----------------------------------------------------------------------------

/// Destinataires, lu / non lu et accusés, pour des messages déjà filtrés.
pub(crate) async fn composer(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
    edition: EventId,
    cote: Cote,
    messages: Vec<Message>,
) -> Result<Vec<ProposalCommentView>> {
    let pool = state.pool();
    let comite = cote == Cote::Comite;

    let mut destinataires: HashMap<Uuid, Vec<CommentPerson>> = HashMap::new();
    if comite {
        for (racine, personne) in cross::destinataires_du_dossier(pool, dossier).await? {
            destinataires.entry(racine).or_default().push(personne);
        }
    }

    let lus: HashSet<Uuid> = comments::lus_par(pool, dossier, lecteur)
        .await?
        .into_iter()
        .collect();

    // Les accusés ne sortent pas du comité : ils nomment ses membres.
    let miens: Vec<Uuid> = messages
        .iter()
        .filter(|m| comite && m.author_id == lecteur)
        .map(|m| m.id)
        .collect();
    let mut tailles: HashMap<Uuid, i64> = HashMap::new();
    let mut lecteurs: HashMap<Uuid, Vec<CommentReader>> = HashMap::new();
    if !miens.is_empty() {
        let audience: Vec<Uuid> = audience_du_dossier(state, dossier, edition)
            .await?
            .into_iter()
            .map(|p| p.person_id)
            .collect();
        tailles = comments::tailles_daudience(pool, &miens, &audience)
            .await?
            .into_iter()
            .collect();
        for (message, lecture) in cross::lecteurs(pool, &miens).await? {
            lecteurs.entry(message).or_default().push(lecture);
        }
    }

    Ok(messages
        .into_iter()
        .map(|m| {
            let a_moi = m.author_id == lecteur;
            let receipts = (comite && a_moi).then(|| CommentReceipts {
                audience_count: tailles.get(&m.id).copied().unwrap_or(0),
                readers: lecteurs.remove(&m.id).unwrap_or_default(),
            });
            ProposalCommentView {
                recipients: destinataires.remove(&m.id).unwrap_or_default(),
                is_read: a_moi || lus.contains(&m.id),
                receipts,
                message: m,
            }
        })
        .collect())
}

// -----------------------------------------------------------------------------
// Élargir un fil adressé
// -----------------------------------------------------------------------------

/// `POST /proposal-comments/{id}/recipients`. Ouvert à qui lit déjà le fil ;
/// seulement sur la racine d'un fil déjà adressé. Idempotent.
pub async fn ajouter_destinataires(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    racine: CommentId,
    payload: AddCommentRecipientsPayload,
) -> Result<()> {
    let message = comments::par_id(state.pool(), racine)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let dossier = ProposalId(message.proposal_id);
    let (edition, acces) = perimeter::acces_au_dossier(state.pool(), acteur, dossier).await?;
    if acces != Acces::BackOffice
        || !comments::lisible(state.pool(), message.id, acteur, Cote::Comite).await?
    {
        return Err(ApiError::not_found());
    }

    if message.parent_id.is_some() || message.visibility != VISIBILITE_COMITE {
        return Err(ApiError::validation(
            "Seul le message qui ouvre un fil du comité reçoit des destinataires.",
            "comment_id",
        ));
    }
    if comments::destinataires(state.pool(), message.id)
        .await?
        .is_empty()
    {
        return Err(ApiError::validation(
            "Ce fil s'adresse déjà à tout le comité : personne n'est à ajouter.",
            "comment_id",
        ));
    }
    // L'auteur du fil y est déjà : le nommer ne change rien.
    let voulues: Vec<Uuid> = payload
        .person_ids
        .iter()
        .copied()
        .filter(|p| *p != message.author_id)
        .collect();
    if voulues.is_empty() {
        return Ok(());
    }
    let choisies = choisir_dans_le_comite(state, dossier, edition, &voulues, &[]).await?;
    let ids: Vec<Uuid> = choisies.iter().map(|p| p.person_id).collect();

    let mut tx = state.db().write(ctx).await?;
    let ajoutes = comments::ajouter_destinataires(&mut tx, message.id, &ids, acteur).await?;
    let a_prevenir: Vec<Uuid> = ajoutes.into_iter().filter(|p| *p != acteur).collect();
    annoncer(
        &mut tx,
        dossier,
        message.id,
        acteur,
        a_prevenir,
        Annonce::Ajout,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// Lu / non lu
// -----------------------------------------------------------------------------

/// `POST /proposals/{id}/comments/read`. Ses propres messages et ceux qu'on ne
/// lit pas sont ignorés sans erreur : l'écran envoie ce qu'il a affiché.
pub async fn marquer_lus(
    state: &ProgrammeState,
    ctx: &RequestContext,
    lecteur: Uuid,
    dossier: ProposalId,
    payload: MarkCommentsReadPayload,
) -> Result<()> {
    let (_, acces) = perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;
    if payload.comment_ids.is_empty() {
        return Ok(());
    }
    let mut tx = state.db().write(ctx).await?;
    comments::marquer_lus(
        &mut tx,
        dossier,
        lecteur,
        &payload.comment_ids,
        cote_de(acces),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// `DELETE /proposal-comments/{id}/read`.
pub async fn marquer_non_lu(
    state: &ProgrammeState,
    ctx: &RequestContext,
    lecteur: Uuid,
    message: CommentId,
) -> Result<()> {
    let ligne = comments::par_id(state.pool(), message)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let (_, acces) =
        perimeter::acces_au_dossier(state.pool(), lecteur, ProposalId(ligne.proposal_id)).await?;
    if !comments::lisible(state.pool(), ligne.id, lecteur, cote_de(acces)).await? {
        return Err(ApiError::not_found());
    }
    let mut tx = state.db().write(ctx).await?;
    comments::marquer_non_lu(&mut tx, ligne.id, lecteur).await?;
    tx.commit().await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// L'avis
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(crate) enum Annonce {
    /// Un fil ouvert à des personnes nommées.
    Adresse,
    /// Une réponse dans ce fil.
    Reponse,
    /// Des personnes ajoutées au fil.
    Ajout,
}

/// `programme.comment.addressed`, dans la transaction du message. Rien quand
/// personne n'est à prévenir.
pub(crate) async fn annoncer(
    conn: &mut PgConnection,
    dossier: ProposalId,
    message: Uuid,
    auteur: Uuid,
    destinataires: Vec<Uuid>,
    annonce: Annonce,
) -> Result<()> {
    if destinataires.is_empty() {
        return Ok(());
    }
    let contexte = cross::contexte_davis(&mut *conn, dossier, auteur)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let (fr, en) = match annonce {
        Annonce::Adresse => (
            format!(
                "{} vous a adressé un message sur « {} ».",
                contexte.auteur, contexte.titre_fr
            ),
            format!(
                "{} sent you a message about \"{}\".",
                contexte.auteur, contexte.titre_en
            ),
        ),
        Annonce::Reponse => (
            format!(
                "{} a répondu dans un fil qui vous est adressé, sur « {} ».",
                contexte.auteur, contexte.titre_fr
            ),
            format!(
                "{} replied in a thread addressed to you, about \"{}\".",
                contexte.auteur, contexte.titre_en
            ),
        ),
        Annonce::Ajout => (
            format!(
                "{} vous a ajouté à un fil du comité sur « {} ».",
                contexte.auteur, contexte.titre_fr
            ),
            format!(
                "{} added you to a committee thread about \"{}\".",
                contexte.auteur, contexte.titre_en
            ),
        ),
    };

    let notification = Notification {
        type_code: contracts::programme::COMMENT_ADDRESSED.to_owned(),
        recipients: destinataires,
        title: NotificationText {
            fr: format!("Message du comité : {}", contexte.titre_fr),
            en: format!("Committee message: {}", contexte.titre_en),
        },
        body: NotificationText { fr, en },
        link_path: format!("/admin/propositions/{dossier}"),
        subject: NotificationSubject {
            schema: "programme".to_owned(),
            table: "proposal_comments".to_owned(),
            id: message,
        },
        // Un avis par dossier tant qu'il n'est pas lu, portant le dernier
        // texte et le nombre de messages.
        group_key: Some(format!(
            "{}:{dossier}",
            contracts::programme::COMMENT_ADDRESSED
        )),
        replace: true,
        variables: serde_json::json!({
            "titre_activite": contexte.titre_fr,
            "auteur": contexte.auteur,
        }),
    };

    events::emit(
        conn,
        DomainEvent {
            aggregate_schema: contracts::programme::AGGREGATE_SCHEMA,
            aggregate_type: contracts::programme::AGGREGATE_COMMENT,
            aggregate_id: message,
            event_type: contracts::programme::COMMENT_ADDRESSED,
            payload: serde_json::to_value(WithNotification {
                notification,
                email: true,
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;
    Ok(())
}
