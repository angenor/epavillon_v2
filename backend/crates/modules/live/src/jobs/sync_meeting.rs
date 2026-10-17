//! `live.sync_meeting` : créer la réunion chez Zoom, ou relire celle qu'on a
//! rattachée (et la recaler sur l'horaire de la séance), puis relancer les
//! inscrits qui l'attendaient.
//!
//! Deux protections contre la réunion en double. La ligne est réservée
//! (`in_progress`) avant tout appel : deux travaux ne créent pas en même temps.
//! Et la création est notée en base AVANT l'appel, la réunion portant la
//! référence de sa ligne dans son ordre du jour : un essai dont la réponse
//! s'est perdue (délai dépassé, worker tué) cherche d'abord chez Zoom la
//! réunion qu'il a peut-être déjà créée.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use serde_json::{json, Value};
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

use crate::repo::meetings::{self, FILE, SYNC_MEETING};
use crate::zoom::{Zoom, ZoomError};

pub struct SyncMeeting {
    db: Db,
    zoom: Arc<Zoom>,
}

impl SyncMeeting {
    pub fn new(db: Db, zoom: Arc<Zoom>) -> Self {
        Self { db, zoom }
    }
}

/// Les champs de Zoom qui valent authentification : jamais dans `provider_payload`.
const SECRETS: [&str; 5] = [
    "password",
    "h323_password",
    "pstn_password",
    "encrypted_password",
    "start_url",
];

pub fn sans_secrets(reponse: &Value) -> Value {
    let mut copie = reponse.clone();
    if let Some(objet) = copie.as_object_mut() {
        for cle in SECRETS {
            objet.remove(cle);
        }
        if let Some(lien) = objet.get("join_url").and_then(Value::as_str) {
            let lien = sans_pwd(lien);
            objet.insert("join_url".to_owned(), Value::from(lien));
        }
    }
    copie
}

/// Le lien générique sans son `pwd=` : avec, il fait entrer sans code d'accès,
/// et `live.meetings_public` le dit diffusable.
pub fn sans_pwd(lien: &str) -> String {
    let Some((base, requete)) = lien.split_once('?') else {
        return lien.to_owned();
    };
    let gardes: Vec<&str> = requete
        .split('&')
        .filter(|p| !p.is_empty() && p.split('=').next() != Some("pwd"))
        .collect();
    if gardes.is_empty() {
        base.to_owned()
    } else {
        format!("{base}?{}", gardes.join("&"))
    }
}

/// La référence de la ligne, posée dans l'ordre du jour de la réunion Zoom.
pub fn reference(meeting_id: Uuid) -> String {
    format!("Réf. ePavillon {meeting_id}")
}

fn instant_zoom(d: OffsetDateTime) -> Option<String> {
    d.to_offset(UtcOffset::UTC)
        .replace_nanosecond(0)
        .ok()?
        .format(&Rfc3339)
        .ok()
}

/// Inscription obligatoire, approbation automatique : chaque inscrit reçoit
/// tout de suite son lien personnel, et Zoom ferme la porte à la jauge.
pub fn corps_de_creation(
    topic: &str,
    agenda: Option<&str>,
    debut: Option<OffsetDateTime>,
    duree: Option<i32>,
    fuseau: &str,
) -> Value {
    json!({
        "type": 2,
        "topic": topic,
        "agenda": agenda.map(|a| a.chars().take(2000).collect::<String>()),
        "start_time": debut.and_then(instant_zoom),
        "duration": duree,
        "timezone": fuseau,
        "settings": {
            "approval_type": 0,
            "registration_type": 1,
            "registrants_email_notification": true,
            "registrants_confirmation_email": true,
            "join_before_host": false,
            "waiting_room": false,
            "meeting_authentication": false,
        },
    })
}

struct AReserver {
    external_id: Option<String>,
    topic: Option<String>,
    agenda: Option<String>,
    scheduled_start_at: Option<OffsetDateTime>,
    duration_minutes: Option<i32>,
    timezone: String,
    creation_envoyee: bool,
}

#[async_trait]
impl JobHandler for SyncMeeting {
    fn task(&self) -> &'static str {
        SYNC_MEETING
    }

    fn queue(&self) -> &'static str {
        FILE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let meeting_id: Uuid = serde_json::from_value(job.payload["meeting_id"].clone())
            .map_err(ApiError::internal)?;

        let mut tx = self.db.write(&job.context()).await?;
        let reservee = sqlx::query_as!(
            AReserver,
            r#"UPDATE live.meetings
                  SET sync_status = 'in_progress', sync_attempts = sync_attempts + 1
                WHERE id = $1
                  AND (sync_status IN ('pending', 'failed')
                       OR (sync_status = 'in_progress' AND updated_at < now() - interval '10 minutes'))
            RETURNING external_id, topic, agenda, scheduled_start_at, duration_minutes,
                      timezone::text AS "timezone!",
                      (provider_payload ? 'creation_envoyee') AS "creation_envoyee!""#,
            meeting_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;

        // Déjà synchronisée, ou prise par un autre worker.
        let Some(r) = reservee else {
            return Ok(());
        };

        match self.appeler_zoom(job, meeting_id, &r).await {
            Ok(reunion) => self.enregistrer(job, meeting_id, &reunion).await,
            Err(e) => {
                let dernier = job.attempts >= job.max_attempts || !e.est_passagere();
                let statut = if dernier { "abandoned" } else { "failed" };
                let mut tx = self.db.write(&job.context()).await?;
                sqlx::query!(
                    "UPDATE live.meetings
                        SET sync_status = $2::text::live.sync_status, last_sync_error = $3
                      WHERE id = $1",
                    meeting_id,
                    statut,
                    e.to_string()
                )
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                if e.est_passagere() {
                    Err(e.into())
                } else {
                    tracing::warn!(reunion = %meeting_id, erreur = %e, "réunion refusée par Zoom");
                    Ok(())
                }
            }
        }
    }
}

impl SyncMeeting {
    async fn appeler_zoom(
        &self,
        job: &ClaimedJob,
        meeting_id: Uuid,
        r: &AReserver,
    ) -> std::result::Result<Value, ZoomError> {
        let client = &self.zoom.client;
        let marque = reference(meeting_id);

        let numero = match &r.external_id {
            Some(n) => n.clone(),
            None => {
                if r.creation_envoyee {
                    if let Some(n) = self.retrouver(&marque).await? {
                        return self.relire(&n, r, true).await;
                    }
                }
                self.noter_lenvoi(job, meeting_id)
                    .await
                    .map_err(|e| ZoomError::Passagere(e.to_string()))?;
                let topic = r.topic.as_deref().unwrap_or("Pavillon de la Francophonie");
                let agenda = match r.agenda.as_deref() {
                    Some(a) if !a.trim().is_empty() => format!("{marque}\n\n{a}"),
                    _ => marque,
                };
                return client
                    .creer_reunion(&corps_de_creation(
                        topic,
                        Some(&agenda),
                        r.scheduled_start_at,
                        r.duration_minutes,
                        &r.timezone,
                    ))
                    .await;
            }
        };
        self.relire(&numero, r, false).await
    }

    /// Une réunion de l'hôte dont l'ordre du jour porte la référence de la ligne.
    async fn retrouver(&self, marque: &str) -> std::result::Result<Option<String>, ZoomError> {
        let reunions = self.zoom.client.reunions_a_venir().await?;
        Ok(reunions
            .iter()
            .find(|m| m["agenda"].as_str().is_some_and(|a| a.contains(marque)))
            .and_then(|m| match &m["id"] {
                Value::Number(n) => Some(n.to_string()),
                Value::String(s) => Some(s.clone()),
                _ => None,
            }))
    }

    async fn noter_lenvoi(&self, job: &ClaimedJob, meeting_id: Uuid) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;
        sqlx::query!(
            "UPDATE live.meetings
                SET provider_payload = jsonb_build_object('creation_envoyee', now())
              WHERE id = $1",
            meeting_id
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// Relit une réunion existante : vérifie l'hôte, impose l'approbation
    /// automatique, la recale sur l'horaire de la séance.
    async fn relire(
        &self,
        numero: &str,
        r: &AReserver,
        creee_par_nous: bool,
    ) -> std::result::Result<Value, ZoomError> {
        let client = &self.zoom.client;
        let reunion = client.lire_reunion(numero).await?;
        if !creee_par_nous {
            verifier_lhote(&reunion, &self.zoom.config.host_user)?;
        }

        let mut correction = serde_json::Map::new();
        // Réunion créée à la main sans inscription, ou à approbation manuelle :
        // sans approbation automatique, pas de lien personnel.
        if reunion["settings"]["approval_type"].as_i64() != Some(0) {
            correction.insert("settings".into(), json!({ "approval_type": 0 }));
        }
        if let (Some(debut), Some(duree)) = (
            r.scheduled_start_at.and_then(instant_zoom),
            r.duration_minutes,
        ) {
            let debut_zoom = reunion["start_time"]
                .as_str()
                .and_then(|t| OffsetDateTime::parse(t, &Rfc3339).ok())
                .and_then(instant_zoom);
            let duree_zoom = reunion["duration"].as_i64();
            if debut_zoom.as_deref() != Some(debut.as_str()) || duree_zoom != Some(i64::from(duree))
            {
                correction.insert("start_time".into(), Value::from(debut));
                correction.insert("duration".into(), Value::from(duree));
                correction.insert("timezone".into(), Value::from(r.timezone.clone()));
            }
        }
        if correction.is_empty() {
            return Ok(reunion);
        }
        client
            .modifier_reunion(numero, &Value::Object(correction))
            .await?;
        client.lire_reunion(numero).await
    }

    async fn enregistrer(&self, job: &ClaimedJob, meeting_id: Uuid, reunion: &Value) -> Result<()> {
        let numero = match &reunion["id"] {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            _ => return Err(ApiError::internal("réponse de Zoom sans numéro de réunion")),
        };
        let coffre = &self.zoom.coffre;
        let passcode = reunion["password"]
            .as_str()
            .filter(|p| !p.is_empty())
            .map(|p| coffre.chiffrer(p))
            .transpose()?;
        let lien_hote = reunion["start_url"]
            .as_str()
            .filter(|p| !p.is_empty())
            .map(|p| coffre.chiffrer(p))
            .transpose()?;
        let url = |cle: &str| {
            reunion[cle]
                .as_str()
                .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
                .map(sans_pwd)
        };

        let mut tx = self.db.write(&job.context()).await?;
        sqlx::query!(
            "UPDATE live.meetings
                SET external_id = $2, external_uuid = $3, provider_account = $4,
                    provider_payload = $5, join_url = $6::text::platform.url,
                    registration_url = $7::text::platform.url,
                    passcode_encrypted = $8, start_url_encrypted = $9,
                    host_email = $10::text::platform.email,
                    status = CASE WHEN status = 'draft' THEN 'scheduled' ELSE status END,
                    sync_status = 'synced', last_synced_at = now(), last_sync_error = NULL
              WHERE id = $1",
            meeting_id,
            numero,
            reunion["uuid"].as_str(),
            reunion["host_id"].as_str(),
            sans_secrets(reunion),
            url("join_url"),
            url("registration_url"),
            passcode,
            lien_hote,
            reunion["host_email"].as_str().filter(|e| e.contains('@')),
        )
        .execute(&mut *tx)
        .await?;
        meetings::requeue_failed(&mut tx, meeting_id).await?;
        tx.commit().await
    }
}

/// Une réunion rattachée doit être celle du compte hôte configuré : sans quoi
/// un administrateur ferait lire le code d'accès de n'importe quelle réunion du
/// compte. `me` ne se compare à rien : le contrôle est alors sauté.
pub fn verifier_lhote(reunion: &Value, hote: &str) -> std::result::Result<(), ZoomError> {
    if hote.eq_ignore_ascii_case("me") {
        return Ok(());
    }
    let correspond = [&reunion["host_email"], &reunion["host_id"]]
        .iter()
        .filter_map(|v| v.as_str())
        .any(|v| v.eq_ignore_ascii_case(hote));
    if correspond {
        Ok(())
    } else {
        Err(ZoomError::Refusee {
            statut: 403,
            code: None,
            message:
                "cette réunion Zoom n'appartient pas au compte hôte configuré (ZOOM_HOST_USER)"
                    .to_owned(),
        })
    }
}
