//! Signaler un changement sur une session officielle, ou une réunion non
//! annoncée. Le signalement ne touche jamais la session (ADR-010).

use kernel::error::{ApiError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

pub const DETAIL_MAX: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportReason {
    Cancelled,
    Time,
    Venue,
    Other,
    Unannounced,
}

impl ReportReason {
    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "cancelled" => Some(Self::Cancelled),
            "time" => Some(Self::Time),
            "venue" => Some(Self::Venue),
            "other" => Some(Self::Other),
            "unannounced" => Some(Self::Unannounced),
            _ => None,
        }
    }

    pub fn as_db(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Time => "time",
            Self::Venue => "venue",
            Self::Other => "other",
            Self::Unannounced => "unannounced",
        }
    }
}

/// `ReportPayload` — ce que le téléphone envoie. Les dates restent du texte
/// jusqu'à la validation, pour que le refus nomme le champ.
#[derive(Debug, Clone, Deserialize)]
pub struct ReportPayload {
    pub client_ref: Uuid,
    pub edition: String,
    pub reason: String,
    #[serde(default)]
    pub session_id: Option<Uuid>,
    #[serde(default)]
    pub proposed_start: Option<String>,
    #[serde(default)]
    pub proposed_venue: Option<String>,
    #[serde(default)]
    pub what: Option<String>,
    #[serde(default)]
    pub day: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

/// Le signalement tel qu'il s'écrit : un champ étranger au motif est ignoré.
#[derive(Debug, Clone, PartialEq)]
pub struct NouveauSignalement {
    pub reason: ReportReason,
    pub session_id: Option<Uuid>,
    pub proposed_start: Option<OffsetDateTime>,
    pub proposed_venue: Option<String>,
    pub what: Option<String>,
    pub proposed_day: Option<Date>,
    pub theme: Option<String>,
    pub detail: Option<String>,
}

fn invalide(champ: &str, message: &str) -> ApiError {
    ApiError::with_message(ErrorCode::NegotiationReportInvalid, message).field(champ)
}

fn texte(valeur: &Option<String>) -> Option<String> {
    valeur
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}

fn instant(valeur: &Option<String>) -> Result<Option<OffsetDateTime>> {
    texte(valeur)
        .map(|v| {
            OffsetDateTime::parse(&v, &Rfc3339).map_err(|_| {
                invalide(
                    "proposed_start",
                    "L'heure proposée doit être un instant ISO 8601.",
                )
            })
        })
        .transpose()
}

impl ReportPayload {
    pub fn valider(&self) -> Result<NouveauSignalement> {
        let reason = ReportReason::from_db(self.reason.trim())
            .ok_or_else(|| invalide("reason", "Ce motif de signalement n'existe pas."))?;
        let detail = texte(&self.detail);
        if detail
            .as_deref()
            .is_some_and(|d| d.chars().count() > DETAIL_MAX)
        {
            return Err(invalide(
                "detail",
                "La précision ne dépasse pas 600 caractères.",
            ));
        }

        if reason == ReportReason::Unannounced {
            let what = texte(&self.what)
                .ok_or_else(|| invalide("what", "Dites de quelle réunion il s'agit (« Quoi »)."))?;
            let jour = texte(&self.day)
                .ok_or_else(|| invalide("day", "Choisissez le jour de la réunion."))?;
            let proposed_day = Date::parse(&jour, format_description!("[year]-[month]-[day]"))
                .map_err(|_| invalide("day", "Le jour doit s'écrire AAAA-MM-JJ."))?;
            return Ok(NouveauSignalement {
                reason,
                session_id: None,
                proposed_start: instant(&self.proposed_start)?,
                proposed_venue: texte(&self.proposed_venue),
                what: Some(what),
                proposed_day: Some(proposed_day),
                theme: texte(&self.theme),
                detail,
            });
        }

        let session_id = self
            .session_id
            .ok_or_else(|| invalide("session_id", "Choisissez la session signalée."))?;
        Ok(NouveauSignalement {
            reason,
            session_id: Some(session_id),
            proposed_start: match reason {
                ReportReason::Time => instant(&self.proposed_start)?,
                _ => None,
            },
            proposed_venue: match reason {
                ReportReason::Venue => texte(&self.proposed_venue),
                _ => None,
            },
            what: None,
            proposed_day: None,
            theme: None,
            detail,
        })
    }
}

/// « Validé » ne se montre qu'une fois publié : avant, l'annulation peut encore
/// tout défaire (R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MyReportStatus {
    Submitted,
    Validated,
    Rejected,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportedSession {
    pub id: Uuid,
    pub title_en: String,
    pub title_fr: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub start_at: OffsetDateTime,
    pub venue: Option<String>,
}

/// `MyReport` — un signalement, vu par son autrice.
#[derive(Debug, Clone, Serialize)]
pub struct MyReport {
    pub id: Uuid,
    pub client_ref: Uuid,
    pub reason: ReportReason,
    pub session: Option<ReportedSession>,
    pub what: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub proposed_start: Option<OffsetDateTime>,
    pub proposed_venue: Option<String>,
    pub day: Option<Date>,
    /// Code `negotiation_theme` d'une réunion non annoncée.
    pub theme: Option<String>,
    /// La réunion née du signalement, une fois publiée et tant qu'elle n'est pas retirée.
    pub network_meeting_id: Option<Uuid>,
    pub detail: Option<String>,
    pub status: MyReportStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub submitted_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub decided_at: Option<OffsetDateTime>,
    pub reject_reason: Option<String>,
    pub reject_detail: Option<String>,
}

/// `MyReports` — plus récent d'abord.
#[derive(Debug, Clone, Serialize)]
pub struct MyReports {
    pub reports: Vec<MyReport>,
}

impl MyReports {
    pub fn empreinte(&self) -> String {
        kernel::empreinte::de(&serde_json::to_string(&self.reports).unwrap_or_default())
    }
}

/// Ce que dit la ligne, ramené à ce que l'autrice a le droit de voir.
pub fn statut_vu(statut: &str, publie: bool) -> MyReportStatus {
    match statut {
        "rejected" => MyReportStatus::Rejected,
        "validated" if publie => MyReportStatus::Validated,
        _ => MyReportStatus::Submitted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn charge(reason: &str) -> ReportPayload {
        ReportPayload {
            client_ref: Uuid::nil(),
            edition: "cop31".into(),
            reason: reason.into(),
            session_id: Some(Uuid::nil()),
            proposed_start: Some("2026-11-10T09:00:00Z".into()),
            proposed_venue: Some(" Salle 4 ".into()),
            what: Some("Réunion".into()),
            day: Some("2026-11-10".into()),
            theme: Some("finance".into()),
            detail: None,
        }
    }

    #[test]
    fn un_champ_etranger_au_motif_est_ignore() {
        let s = charge("venue").valider().expect("valide");
        assert_eq!(s.proposed_venue.as_deref(), Some("Salle 4"));
        assert!(s.proposed_start.is_none() && s.what.is_none() && s.theme.is_none());
    }

    #[test]
    fn le_refus_nomme_le_champ() {
        let mut c = charge("unannounced");
        c.what = Some("  ".into());
        assert_eq!(c.valider().unwrap_err().field.as_deref(), Some("what"));
        let mut c = charge("time");
        c.session_id = None;
        assert_eq!(
            c.valider().unwrap_err().field.as_deref(),
            Some("session_id")
        );
        let mut c = charge("time");
        c.detail = Some("é".repeat(601));
        assert_eq!(c.valider().unwrap_err().field.as_deref(), Some("detail"));
        c.detail = Some("é".repeat(600));
        assert!(c.valider().is_ok());
        assert_eq!(
            charge("moved").valider().unwrap_err().field.as_deref(),
            Some("reason")
        );
    }

    #[test]
    fn valide_ne_se_dit_quune_fois_publie() {
        assert_eq!(statut_vu("validated", false), MyReportStatus::Submitted);
        assert_eq!(statut_vu("validated", true), MyReportStatus::Validated);
        assert_eq!(statut_vu("rejected", false), MyReportStatus::Rejected);
    }
}
