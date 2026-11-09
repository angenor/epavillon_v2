//! Les intervenants en ligne (08/11), `075_programme_sessions.sql` § 2 bis.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// En-tête du jeton du lien d'intervenant : jamais dans l'adresse de l'API,
/// qui finit dans les journaux.
pub const EN_TETE_LIEN: &str = "x-speaker-link";

pub const SOURCE_REGIE: &str = "regie";
pub const SOURCE_LIEN: &str = "lien";

const CIVILITES: [&str; 5] = ["mme", "m", "dr", "pr", "other"];

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OnlineSpeakerInput {
    pub civility: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub job_title: String,
    pub organization: String,
    pub email: String,
    pub country_iso2: Option<String>,
    pub bio: Option<String>,
    #[serde(default)]
    pub consent: bool,
}

/// La saisie, nettoyée et vérifiée.
#[derive(Debug, Clone)]
pub struct IntervenantSaisi {
    pub civility: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub job_title: String,
    pub organization: String,
    pub email: String,
    pub country_iso2: Option<String>,
    pub bio: Option<String>,
}

fn requis(valeur: &str, champ: &str, libelle: &str, max: usize) -> kernel::error::Result<String> {
    let v = valeur.trim();
    if v.is_empty() {
        return Err(kernel::error::ApiError::validation(
            format!("{libelle} est obligatoire."),
            champ,
        ));
    }
    if v.chars().count() > max {
        return Err(kernel::error::ApiError::validation(
            format!("{libelle} tient en {max} caractères."),
            champ,
        ));
    }
    Ok(v.to_owned())
}

impl OnlineSpeakerInput {
    pub fn verifier(&self) -> kernel::error::Result<IntervenantSaisi> {
        let email = requis(&self.email, "email", "L'adresse courriel", 254)?.to_lowercase();
        let (avant, apres) = email.split_once('@').unwrap_or(("", ""));
        if avant.is_empty() || !apres.contains('.') || email.contains(char::is_whitespace) {
            return Err(kernel::error::ApiError::validation(
                "Cette adresse courriel n'est pas valable.",
                "email",
            ));
        }
        let civility = self
            .civility
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_owned);
        if civility.as_deref().is_some_and(|c| !CIVILITES.contains(&c)) {
            return Err(kernel::error::ApiError::validation(
                "Civilité inconnue.",
                "civility",
            ));
        }
        let bio = self
            .bio
            .as_deref()
            .map(str::trim)
            .filter(|b| !b.is_empty())
            .map(str::to_owned);
        if bio.as_deref().is_some_and(|b| b.chars().count() > 2000) {
            return Err(kernel::error::ApiError::validation(
                "La présentation tient en deux mille caractères.",
                "bio",
            ));
        }
        Ok(IntervenantSaisi {
            civility,
            first_name: requis(&self.first_name, "first_name", "Le prénom", 100)?,
            last_name: requis(&self.last_name, "last_name", "Le nom", 100)?,
            job_title: requis(&self.job_title, "job_title", "La fonction", 200)?,
            organization: requis(&self.organization, "organization", "L'organisation", 200)?,
            email,
            country_iso2: self
                .country_iso2
                .as_deref()
                .map(str::trim)
                .filter(|c| c.len() == 2)
                .map(str::to_uppercase),
            bio,
        })
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OnlineSpeakerCreated {
    pub speaker_id: Uuid,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SpeakerLinkInvite {
    pub session_id: Uuid,
    pub title: Value,
    pub event_title: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub timezone: String,
    pub city: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieSpeaker {
    pub speaker_id: Uuid,
    pub display_name: String,
    pub job_title: Option<String>,
    pub organization: Option<String>,
    pub email: Option<String>,
    /// `dossier`, `regie` ou `lien`.
    pub source: String,
    pub is_listed: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub link_sent_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegieSpeakers {
    pub session_id: Uuid,
    pub speakers: Vec<RegieSpeaker>,
    pub signup_url: String,
    pub owner_emails: Vec<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub owner_link_sent_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SpeakerLinksSent {
    pub recipients: i64,
}

/// Le résumé de la carte de la régie.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct RegieSpeakerSummary {
    /// Intervenants dont le lien Zoom a été envoyé.
    pub online: i64,
    /// Intervenants qui ont une adresse.
    pub reachable: i64,
    /// Inscrits par le lien, pas encore affichés sur la page.
    pub to_list: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub links_sent_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub owner_link_sent_at: Option<OffsetDateTime>,
    pub has_owner: bool,
}

/// Le lien d'inscription, tel que le porteur le reçoit.
pub fn lien_dinscription(app_public_url: &str, jeton: &str) -> String {
    format!(
        "{}/intervenant?jeton={jeton}",
        app_public_url.trim_end_matches('/')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saisie() -> OnlineSpeakerInput {
        OnlineSpeakerInput {
            civility: Some("dr".into()),
            first_name: " Koffi ".into(),
            last_name: "Mensah".into(),
            job_title: "Chercheur".into(),
            organization: "Université".into(),
            email: " K.Mensah@Exemple.org ".into(),
            country_iso2: Some("gh".into()),
            bio: Some("  ".into()),
            consent: true,
        }
    }

    #[test]
    fn la_saisie_se_nettoie() {
        let s = saisie().verifier().unwrap();
        assert_eq!(s.first_name, "Koffi");
        assert_eq!(s.email, "k.mensah@exemple.org");
        assert_eq!(s.country_iso2.as_deref(), Some("GH"));
        assert!(s.bio.is_none());
    }

    #[test]
    fn fonction_et_adresse_sont_exigees() {
        let mut s = saisie();
        s.job_title = " ".into();
        assert!(s.verifier().is_err());
        let mut s = saisie();
        s.email = "pas-une-adresse".into();
        assert!(s.verifier().is_err());
    }
}
