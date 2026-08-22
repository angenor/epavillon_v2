//! **Chacun choisit ce qu'il reçoit** — et ce qui est critique part quand même.
//!
//! # La liste et le compte dans la même réponse
//!
//! Deux appels donneraient deux chiffres mesurés à deux instants, et un badge
//! qui contredit la liste qu'il coiffe. C'est le défaut que B4 avait nommé sur
//! les facettes d'une liste, et il se reproduirait ici.
//!
//! # Une préférence sur un type critique est ENREGISTRÉE, et sans effet
//!
//! L'API ne refuse pas : refuser laisserait l'écran sans réponse à donner, et
//! l'interrupteur reviendrait à sa position sans explication. C'est la
//! **lecture** qui dit que l'arbitrage n'oppose rien, par `is_overridable`
//! (FR-095).
//!
//! # Un type inconnu vaut refus, jamais un envoi par défaut
//!
//! `is_channel_enabled()` est **totale** et rend faux pour un type qu'elle ne
//! connaît pas : *« on n'invente pas d'envoi »*. L'écriture de préférence, elle,
//! refuse explicitement — sans quoi une faute de frappe poserait une ligne
//! orpheline que personne ne relirait jamais.

use kernel::auth::{has_permission, Scope};
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::mail::OutgoingMail;
use kernel::RequestContext;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::notification::{
    NotificationFeed, NotificationPreferencePayload, NotificationPreferenceRow,
};
use crate::domain::reminder::NotificationChannel;
use crate::repo::{cross, delivery, notifications, preferences};
use crate::state::EngagementState;

/// Le type servi par la diffusion d'annonce. Il est **semé** par le modèle, avec
/// ses deux canaux et sa criticité basse : une annonce de plateforme se coupe.
pub const TYPE_ANNONCE: &str = "engagement.announcement.general";

/// La permission de diffuser, sur la portée globale : une annonce s'adresse à
/// la plateforme, pas à une édition.
const PERMISSION_DIFFUSION: &str = "engagement.notification.broadcast";

/// Le plafond d'une page de fil. Écrit ici plutôt que laissé au client : une
/// limite reçue sans borne laisse un appel demander la table entière.
const LIMITE_MAX: i64 = 100;
const LIMITE_DEFAUT: i64 = 30;

// -----------------------------------------------------------------------------
// Le fil
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct FilQuery {
    #[serde(default)]
    pub unread_only: bool,
    pub limit: Option<i64>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub before: Option<OffsetDateTime>,
}

pub async fn fil(
    state: &EngagementState,
    acteur: Uuid,
    requete: &FilQuery,
) -> Result<NotificationFeed> {
    let limite = requete.limit.unwrap_or(LIMITE_DEFAUT).clamp(1, LIMITE_MAX);
    let fil = notifications::fil(
        state.pool(),
        acteur,
        requete.unread_only,
        limite,
        requete.before,
    )
    .await?;

    Ok(NotificationFeed {
        items: fil.items,
        unread_count: fil.unread_count,
    })
}

/// Ce qu'un marquage vise. Sans `ids` : tout.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MarquagePayload {
    pub ids: Option<Vec<Uuid>>,
}

pub async fn marquer_lues(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: &MarquagePayload,
) -> Result<u64> {
    let mut tx = state.db().write(ctx).await?;
    let marquees = notifications::marquer_lues(&mut tx, acteur, payload.ids.as_deref()).await?;
    tx.commit().await?;
    Ok(marquees)
}

/// Ce qu'un archivage vise. La liste est **exigée** : « tout archiver » n'est
/// pas un geste qu'on fait par mégarde.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ArchivagePayload {
    pub ids: Vec<Uuid>,
}

pub async fn archiver(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: &ArchivagePayload,
) -> Result<u64> {
    let mut tx = state.db().write(ctx).await?;
    let archivees = notifications::archiver(&mut tx, acteur, &payload.ids).await?;
    tx.commit().await?;
    Ok(archivees)
}

// -----------------------------------------------------------------------------
// Les préférences
// -----------------------------------------------------------------------------

pub async fn preferences(
    state: &EngagementState,
    acteur: Uuid,
    locale: &str,
) -> Result<Vec<NotificationPreferenceRow>> {
    preferences::lister(state.pool(), acteur, locale).await
}

/// Écrit un lot d'arbitrages, et **rend la liste entière** : l'écran affiche
/// l'état après écriture sans second appel, et une préférence sans effet se voit
/// immédiatement.
pub async fn ecrire_les_preferences(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    locale: &str,
    lot: &[NotificationPreferencePayload],
) -> Result<Vec<NotificationPreferenceRow>> {
    for arbitrage in lot {
        if NotificationChannel::from_db(&arbitrage.channel).is_none() {
            return Err(ApiError::new(ErrorCode::ValidationFailed)
                .field("channel")
                .detail(format!("canal inconnu : « {} »", arbitrage.channel)));
        }
    }

    let mut tx = state.db().write(ctx).await?;
    for arbitrage in lot {
        // **Le type inconnu se refuse à l'écriture**, alors que l'envoi le
        // refuse en silence : une ligne orpheline ne serait jamais relue, et la
        // personne croirait avoir coupé quelque chose.
        if let Err(erreur) = preferences::ecrire(
            &mut tx,
            acteur,
            &arbitrage.type_code,
            &arbitrage.channel,
            arbitrage.is_enabled,
        )
        .await
        {
            tx.rollback().await?;
            return Err(traduire(erreur, &arbitrage.type_code));
        }
    }
    tx.commit().await?;

    preferences::lister(state.pool(), acteur, locale).await
}

fn traduire(erreur: sqlx::Error, type_code: &str) -> ApiError {
    match kernel::pg_error::constraint(&erreur) {
        Some(nom) if nom.contains("type_code") => {
            ApiError::new(ErrorCode::EngagementNotificationTypeUnknown)
                .field("type_code")
                .detail(format!("« {type_code} » n'est pas au catalogue"))
        }
        _ => ApiError::from(erreur),
    }
}

// -----------------------------------------------------------------------------
// La diffusion d'une annonce
// -----------------------------------------------------------------------------

/// À qui l'annonce s'adresse.
///
/// **Deux périmètres et pas un de plus** : toute la plateforme, ou les inscrits
/// d'une édition. Un troisième — « les référents d'organisation », « les
/// négociateurs » — demanderait une définition que rien ne porte aujourd'hui, et
/// l'inventer produirait une liste que personne n'aurait validée.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Audience {
    /// Les comptes actifs de la plateforme.
    All,
    /// Les inscrits des séances d'une édition, hors annulés.
    Event { event_id: Uuid },
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BroadcastPayload {
    /// Multilingue, comme tout texte que l'écran affiche.
    pub title: serde_json::Value,
    pub body: Option<serde_json::Value>,
    /// **Chemin relatif**, jamais une adresse absolue.
    pub link_path: Option<String>,
    pub audience: Audience,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BroadcastResult {
    pub recipients: i64,
    /// Ceux qui l'ont reçue **par courriel** — les autres l'ont coupé, ou leur
    /// adresse est hors circuit.
    pub emailed: i64,
}

/// Diffuse une annonce.
///
/// **L'expédition est faite dans la requête**, sans travail différé : aucune des
/// cinq tâches du jalon n'en prévoit un, et en ajouter une sixième pour un geste
/// que l'IFDD fait quelques fois par an coûterait plus qu'elle ne rapporte. Le
/// périmètre est borné par la nature des deux audiences ; le jour où une
/// annonce vise dix mille comptes, elle passera par la file, et ce sera un
/// chantier assumé.
pub async fn diffuser(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: &BroadcastPayload,
) -> Result<BroadcastResult> {
    if !has_permission(state.pool(), acteur, PERMISSION_DIFFUSION, Scope::Global).await? {
        return Err(ApiError::forbidden());
    }
    if let Some(chemin) = payload.link_path.as_deref() {
        if !chemin.starts_with('/') {
            return Err(ApiError::new(ErrorCode::ValidationFailed)
                .field("link_path")
                .detail(
                    "un chemin relatif est attendu : un nom d'hôte ne doit pas entrer en base",
                ));
        }
    }

    let destinataires = match payload.audience {
        Audience::All => cross::comptes_actifs(state.pool()).await?,
        Audience::Event { event_id } => cross::inscrits_de_ledition(state.pool(), event_id).await?,
    };

    // Une clé de regroupement par diffusion : deux annonces successives ne se
    // confondent pas, et une même annonce relayée deux fois n'écrit qu'une
    // ligne par personne.
    let cle = format!("{TYPE_ANNONCE}:{}", ctx.request_id);
    let mut touches = 0_i64;
    let mut par_courriel = 0_i64;

    for destinataire in &destinataires {
        if delivery::canal_autorise(
            state.pool(),
            destinataire.person_id,
            TYPE_ANNONCE,
            NotificationChannel::InApp.as_str(),
        )
        .await?
        {
            let mut tx = state.db().write(ctx).await?;
            notifications::ecrire(
                &mut tx,
                &notifications::NouvelleNotification {
                    person_id: destinataire.person_id,
                    type_code: TYPE_ANNONCE,
                    title: payload.title.clone(),
                    body: payload.body.clone(),
                    variables: serde_json::json!({}),
                    link_path: payload.link_path.clone(),
                    subject_schema: None,
                    subject_table: None,
                    subject_id: None,
                    group_key: Some(cle.clone()),
                },
            )
            .await?;
            tx.commit().await?;
            touches += 1;
        }

        if delivery::canal_autorise(
            state.pool(),
            destinataire.person_id,
            TYPE_ANNONCE,
            NotificationChannel::Email.as_str(),
        )
        .await?
        {
            // La garde d'envoi écarte les adresses hors circuit et écrit la
            // trace : ce service ne consulte pas la liste lui-même.
            let langue = &destinataire.locale;
            let mail = OutgoingMail {
                message_id: Uuid::now_v7().to_string(),
                to: destinataire.email.clone(),
                locale: langue.clone(),
                subject: textuel(&payload.title, langue),
                text: payload
                    .body
                    .as_ref()
                    .map(|corps| textuel(corps, langue))
                    .unwrap_or_else(|| textuel(&payload.title, langue)),
                html: None,
            };
            if let Err(erreur) = state.mailer().send(&mail).await {
                // Une annonce n'est pas un courriel de sécurité : l'échec d'un
                // destinataire ne doit pas interrompre les autres.
                tracing::warn!(erreur = %erreur, "annonce non remise à un destinataire");
                continue;
            }
            par_courriel += 1;
        }
    }

    Ok(BroadcastResult {
        recipients: touches,
        emailed: par_courriel,
    })
}

/// Repli sur le français, comme `platform.t()`.
fn textuel(valeur: &serde_json::Value, locale: &str) -> String {
    valeur
        .get(locale)
        .and_then(serde_json::Value::as_str)
        .filter(|texte| !texte.is_empty())
        .or_else(|| valeur.get("fr").and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .to_owned()
}
