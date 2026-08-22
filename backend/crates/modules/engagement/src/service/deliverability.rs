//! **La plateforme cesse d'écrire à qui ne veut plus la lire.**
//!
//! # C'est la réputation du domaine qui est en jeu, pas le confort
//!
//! Une adresse en rebond dur qui continue de recevoir des invitations fait
//! monter le taux de rebond du domaine expéditeur — et le domaine finit en
//! dossier indésirable **pour tout le monde**, confirmations d'inscription
//! comprises. C'est le défaut que la v1 portait, et la liste de suppression est
//! la seule réponse honnête à « arrêtez de m'écrire ».
//!
//! # Une suppression échue se lève sans intervention
//!
//! `is_email_suppressed()` compare déjà `expires_at` à maintenant. **Aucun
//! travail récurrent ne la lève**, et c'est délibéré : une purge programmée
//! serait un second dispositif à tenir d'accord avec la fonction du modèle, et
//! le premier écart entre les deux serait silencieux (FR-098).
//!
//! # L'adresse voyage HACHÉE dans l'annonce
//!
//! L'outbox est durable, indexée par agrégat, relayée et faite pour être relue.
//! Une adresse électronique est une donnée personnelle : qui la détient déjà
//! peut vérifier qu'elle est concernée, personne ne peut la lire.

use kernel::auth::{has_permission, Scope};
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::DomainEvent;
use kernel::RequestContext;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::template::EmailSuppression;
use crate::mail::empreinte_adresse;
use crate::repo::suppressions;
use crate::state::EngagementState;

/// La permission qui gouverne la liste. La même que les modèles : celui qui
/// écrit les courriels de la plateforme est celui qui répond de leur
/// délivrabilité.
const PERMISSION: &str = "engagement.template.manage";

/// Les motifs de `engagement.suppression_reason`, en toutes lettres. L'énuméré
/// est fermé en base : un motif inconnu sortirait en erreur de transtypage, sur
/// un message qui ne dirait pas quel champ est en cause.
const MOTIFS: [&str; 5] = [
    "hard_bounce",
    "complaint",
    "unsubscribe",
    "invalid_address",
    "manual",
];

// -----------------------------------------------------------------------------
// La liste
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RechercheQuery {
    pub q: Option<String>,
}

pub async fn lister(
    state: &EngagementState,
    acteur: Uuid,
    recherche: Option<&str>,
) -> Result<Vec<EmailSuppression>> {
    exiger_le_droit(state, acteur).await?;
    suppressions::lister(state.pool(), recherche).await
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SuppressionPayload {
    pub email: String,
    /// `hard_bounce`, `complaint`, `unsubscribe`, `invalid_address`, `manual`.
    pub reason: String,
    pub detail: Option<String>,
    /// Nulle : définitive. Une valeur lève la suppression toute seule le moment
    /// venu — une boîte pleine n'est pas une adresse morte.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
}

pub async fn poser(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: &SuppressionPayload,
) -> Result<EmailSuppression> {
    exiger_le_droit(state, acteur).await?;
    if !MOTIFS.contains(&payload.reason.as_str()) {
        return Err(ApiError::new(ErrorCode::ValidationFailed)
            .field("reason")
            .detail(format!("motif inconnu : « {} »", payload.reason)));
    }

    let mut tx = state.db().write(ctx).await?;
    let posee = suppressions::poser(
        &mut tx,
        &payload.email,
        &payload.reason,
        payload.detail.as_deref(),
        payload.expires_at,
        Some(acteur),
    )
    .await;

    let posee = match posee {
        Ok(posee) => posee,
        Err(erreur) => {
            tx.rollback().await?;
            return Err(ApiError::from(erreur));
        }
    };

    annoncer(&mut tx, &payload.email, &payload.reason).await?;
    tx.commit().await?;

    Ok(posee)
}

pub async fn retirer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    email: &str,
) -> Result<bool> {
    exiger_le_droit(state, acteur).await?;
    let mut tx = state.db().write(ctx).await?;
    let retiree = suppressions::retirer(&mut tx, email).await?;
    tx.commit().await?;
    Ok(retiree)
}

/// **L'un des deux seuls événements que ce module émet**, et l'adresse n'y est
/// pas en clair.
///
/// L'agrégat est identifié par les seize premiers octets de l'empreinte : une
/// valeur **stable** — deux annonces sur la même adresse se rangent ensemble —
/// et qui ne laisse pas relire l'adresse. Un identifiant tiré au hasard perdrait
/// la corrélation, et l'adresse en clair perdrait la garantie.
async fn annoncer(
    conn: &mut sqlx::postgres::PgConnection,
    email: &str,
    reason: &str,
) -> Result<()> {
    let empreinte = empreinte_adresse(email);
    let digest = Sha256::digest(email.trim().to_lowercase().as_bytes());
    let aggregate_id = Uuid::from_slice(&digest[..16]).expect("seize octets");

    let charge = contracts::engagement::EmailSuppressed {
        email_hash: empreinte,
        reason: reason.to_owned(),
    };

    kernel::events::emit(
        conn,
        DomainEvent {
            aggregate_schema: contracts::engagement::AGGREGATE_SCHEMA,
            aggregate_type: contracts::engagement::AGGREGATE_EMAIL_SUPPRESSION,
            aggregate_id,
            event_type: contracts::engagement::EMAIL_SUPPRESSED,
            payload: serde_json::to_value(charge).map_err(ApiError::internal)?,
        },
    )
    .await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// L'ingestion des retours du fournisseur
// -----------------------------------------------------------------------------

/// Ce que le site remonte de ce que le fournisseur a dit d'un courriel.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MailEvent {
    /// **L'identifiant que l'API a remis au site avec le message** — c'est lui
    /// qui relie l'annonce à sa trace. Le contrat d'envoi du noyau ne rapporte
    /// aucun identifiant de fournisseur : s'y reposer laisserait toute annonce
    /// sans trace à mettre à jour.
    pub message_id: String,
    /// Celui du fournisseur, conservé pour corréler ses propres journaux.
    pub provider_message_id: Option<String>,
    /// `delivered`, `bounced`, `complained`, `failed`.
    pub status: String,
    /// `hard`, `soft`, `block` — seulement pour un rebond.
    pub bounce_kind: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Ingestion {
    pub applied: i64,
    /// Une annonce déjà vue, ou dont la trace est introuvable. **Ignorée n'est
    /// pas perdue** : le fournisseur rejoue volontiers ses annonces, et rendre
    /// une erreur le ferait recommencer sans fin.
    pub ignored: i64,
}

/// Les états que le modèle connaît, et le sous-ensemble qu'une annonce peut
/// poser. `queued` et `sent` appartiennent à l'expéditeur, pas au fournisseur.
const ETATS_ANNONCABLES: [&str; 4] = ["delivered", "bounced", "complained", "failed"];

/// **Un rebond dur ou une plainte inscrivent l'adresse sur la liste** : c'est le
/// seul geste qui protège la réputation du domaine sans intervention humaine
/// (FR-097).
fn motif_de_suppression(annonce: &MailEvent) -> Option<&'static str> {
    match (annonce.status.as_str(), annonce.bounce_kind.as_deref()) {
        ("bounced", Some("hard")) => Some("hard_bounce"),
        ("complained", _) => Some("complaint"),
        _ => None,
    }
}

pub async fn ingerer(
    state: &EngagementState,
    ctx: &RequestContext,
    annonces: &[MailEvent],
) -> Result<Ingestion> {
    let mut applied = 0;
    let mut ignored = 0;

    for annonce in annonces {
        if !ETATS_ANNONCABLES.contains(&annonce.status.as_str()) {
            ignored += 1;
            continue;
        }

        let Ok(message_id) = annonce.message_id.parse::<Uuid>() else {
            ignored += 1;
            continue;
        };

        let mut tx = state.db().write(ctx).await?;
        let Some(trace) =
            suppressions::trace_du_message(&mut tx, message_id, &annonce.status).await?
        else {
            tx.rollback().await?;
            ignored += 1;
            continue;
        };

        // **Rejouée, elle est ignorée** — jamais dupliquée. Le fournisseur
        // rejoue volontiers, et une seconde application avancerait l'instant de
        // remise sans rien apprendre.
        if trace.deja_a_cet_etat {
            tx.rollback().await?;
            ignored += 1;
            continue;
        }

        suppressions::appliquer(
            &mut tx,
            &trace,
            &suppressions::AnnonceRecue {
                statut: &annonce.status,
                bounce_kind: annonce.bounce_kind.as_deref(),
                detail: annonce.detail.as_deref(),
                provider_message_id: annonce.provider_message_id.as_deref(),
            },
        )
        .await?;

        if let Some(motif) = motif_de_suppression(annonce) {
            suppressions::poser(
                &mut tx,
                &trace.email,
                motif,
                annonce.detail.as_deref(),
                None,
                None,
            )
            .await
            .map_err(ApiError::from)?;
            annoncer(&mut tx, &trace.email, motif).await?;
        }

        tx.commit().await?;
        applied += 1;
    }

    Ok(Ingestion { applied, ignored })
}

async fn exiger_le_droit(state: &EngagementState, acteur: Uuid) -> Result<()> {
    if has_permission(state.pool(), acteur, PERMISSION, Scope::Global).await? {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}
