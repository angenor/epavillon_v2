//! Les courriels d'une inscription à une journée (02/11).
//!
//! - **La confirmation** (`engagement.send_day_registration`), mise en file par
//!   le module Programmation dans la transaction de l'inscription. Sans compte,
//!   elle porte le jeton en clair du lien de gestion : le travail le déclare
//!   (`carries_secret`), sa charge est vidée à la réussite comme à la mort.
//!   Zoom n'envoie pas sa propre confirmation pour une réunion de journée :
//!   c'est ce courriel qui porte le lien du direct.
//! - **Les rappels de la journée** (`engagement.send_day_reminder`), un par
//!   décalage d'un jour ou plus de la règle de l'édition (2 jours, 1 jour avec la
//!   règle par défaut). Ceux de moins d'un jour partent par activité gardée.
//!   Un travail porte l'heure de début qu'il vise : si la journée a bougé, il
//!   ne part pas, celui de la nouvelle heure a été mis en file à sa place.
//!
//! Un rappel ne porte pas le jeton (la base ne le garde que haché) : il mène à
//! la page de la journée, et le courriel de confirmation reste la clé de
//! l'invité.

use async_trait::async_trait;
use contracts::engagement::{DayRegistrationEmail, EMAIL_QUEUE, SEND_DAY_REGISTRATION};
use kernel::config::Config;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::{Mailer, OutgoingMail};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::repo::delivery;
use crate::repo::journees::{self, JourneePourCourriel};

pub const SEND_DAY_REMINDER: &str = "engagement.send_day_reminder";
const TYPE_CONFIRMATION: &str = "programme.track_registration.confirmed";
const TYPE_RAPPEL: &str = "programme.session.reminder";

/// Les pages du site (`frontend/app/pages/track/[edition]/[track]/`).
fn page(app: &str, j: &JourneePourCourriel, suite: Option<(&str, &str)>) -> String {
    let base = app.trim_end_matches('/');
    let anglais = j.locale.starts_with("en");
    let journee = if anglais {
        format!(
            "{base}/en/programmes/{}/days/{}",
            j.event_slug, j.track_slug
        )
    } else {
        format!(
            "{base}/programmations/{}/journees/{}",
            j.event_slug, j.track_slug
        )
    };
    match suite {
        None => journee,
        Some((fr, en)) => format!("{journee}/{}", if anglais { en } else { fr }),
    }
}

/// La confirmation, en français ou en anglais selon la langue d'inscription.
pub fn confirmation(
    j: &JourneePourCourriel,
    jeton: Option<&str>,
    renvoi: bool,
    app: &str,
    message_id: &str,
) -> OutgoingMail {
    let anglais = j.locale.starts_with("en");
    let (direct, gestion) = match jeton {
        Some(t) => (
            format!("{}?jeton={t}", page(app, j, Some(("direct", "live")))),
            format!(
                "{}?jeton={t}",
                page(app, j, Some(("mon-inscription", "my-registration")))
            ),
        ),
        None => (page(app, j, None), page(app, j, None)),
    };
    let quand = j
        .debut_local
        .as_deref()
        .map(|d| format!("{d} ({})", j.timezone))
        .unwrap_or_default();

    let (sujet, texte) = if anglais {
        let sujet = if renvoi {
            format!("Your link for {}", j.titre)
        } else {
            format!("Registration confirmed: {}", j.titre)
        };
        let texte = format!(
            "Hello {prenom},\n\n\
             {intro} \"{titre}\" starts on {quand}. One link is all you need to follow the whole day live, from the welcome to the closing.\n\n\
             You are following {n} of {m} activities: you will get a reminder before each of them starts.\n\n\
             Join the live stream of the day, on the day:\n{direct}\n\n\
             Change your activities or cancel your registration:\n{gestion}\n\n\
             {perso}",
            prenom = j.first_name,
            intro = if renvoi { "Here is your link again." } else { "Your registration is confirmed." },
            titre = j.titre,
            n = j.gardees,
            m = j.activites,
            perso = if jeton.is_some() { "These links are personal: please do not forward them." } else { "" },
        );
        (sujet, texte)
    } else {
        let sujet = if renvoi {
            format!("Votre lien pour {}", j.titre)
        } else {
            format!("Inscription confirmée : {}", j.titre)
        };
        let texte = format!(
            "Bonjour {prenom},\n\n\
             {intro} « {titre} » commence le {quand}. Un seul lien suffit pour suivre tout le direct de la journée, de l'accueil à la clôture.\n\n\
             Vous suivez {n} activité(s) sur {m} : vous recevrez un rappel avant le début de chacune.\n\n\
             Rejoindre le direct de la journée, le jour même :\n{direct}\n\n\
             Changer vos activités ou vous désinscrire :\n{gestion}\n\n\
             {perso}",
            prenom = j.first_name,
            intro = if renvoi { "Voici de nouveau votre lien." } else { "Votre inscription est confirmée." },
            titre = j.titre,
            n = j.gardees,
            m = j.activites,
            perso = if jeton.is_some() { "Ces liens vous sont personnels : ne les transférez pas." } else { "" },
        );
        (sujet, texte)
    };
    OutgoingMail {
        message_id: message_id.to_owned(),
        to: j.email.clone(),
        locale: j.locale.clone(),
        subject: sujet,
        text: texte.trim_end().to_owned() + "\n",
        html: None,
    }
}

/// Le rappel de la journée, `minutes` avant son début.
pub fn rappel(j: &JourneePourCourriel, minutes: i64, app: &str, message_id: &str) -> OutgoingMail {
    let anglais = j.locale.starts_with("en");
    let jours = (minutes / 1440).max(1);
    let lien = page(app, j, None);
    let quand = j
        .debut_local
        .as_deref()
        .map(|d| format!("{d} ({})", j.timezone))
        .unwrap_or_default();
    let (sujet, texte) = if anglais {
        let delai = if jours == 1 {
            "tomorrow".to_owned()
        } else {
            format!("in {jours} days")
        };
        (
            format!("{} starts {delai}", j.titre),
            format!(
                "Hello {},\n\n\"{}\" starts {delai}, on {quand}. Follow the live stream with the link in your confirmation email, or from the page of the day:\n{lien}\n",
                j.first_name, j.titre
            ),
        )
    } else {
        let delai = if jours == 1 {
            "demain".to_owned()
        } else {
            format!("dans {jours} jours")
        };
        (
            format!("{} commence {delai}", j.titre),
            format!(
                "Bonjour {},\n\n« {} » commence {delai}, le {quand}. Suivez le direct avec le lien de votre courriel de confirmation, ou depuis la page de la journée :\n{lien}\n",
                j.first_name, j.titre
            ),
        )
    };
    OutgoingMail {
        message_id: message_id.to_owned(),
        to: j.email.clone(),
        locale: j.locale.clone(),
        subject: sujet,
        text: texte,
        html: None,
    }
}

// ---------------------------------------------------------------------------
// La confirmation
// ---------------------------------------------------------------------------

pub struct SendDayRegistration {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendDayRegistration {
    pub fn new(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            db,
            mailer,
            app_public_url: config.app_public_url.clone(),
        }
    }
}

#[async_trait]
impl JobHandler for SendDayRegistration {
    fn task(&self) -> &'static str {
        SEND_DAY_REGISTRATION
    }

    fn queue(&self) -> &'static str {
        EMAIL_QUEUE
    }

    fn carries_secret(&self) -> bool {
        true
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let c: DayRegistrationEmail = serde_json::from_value(job.payload.clone())
            .map_err(|_| ApiError::internal("charge de confirmation de journée illisible"))?;
        let pool = self.db.pool();
        let Some(j) = journees::journee_pour_courriel(pool, c.track_registration_id).await? else {
            return Ok(());
        };
        if j.status == "cancelled" {
            return Ok(());
        }
        // Un invité n'a de lien que par ce courriel : on ne le coupe pas.
        if c.token.is_none()
            && !delivery::canal_autorise(pool, j.person_id, TYPE_CONFIRMATION, "email").await?
        {
            return Ok(());
        }
        let mail = confirmation(
            &j,
            c.token.as_deref(),
            c.resent,
            &self.app_public_url,
            &job.id.to_string(),
        );
        self.mailer.send(&mail).await.map_err(|_| {
            ApiError::internal("envoi de la confirmation de journée refusé par le relais")
        })
    }
}

// ---------------------------------------------------------------------------
// Les rappels de la journée
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct ChargeRappel {
    track_registration_id: Uuid,
    offset_minutes: i64,
    /// Début visé, en secondes Unix : un travail d'une heure dépassée ne part pas.
    starts_at: i64,
}

/// Met en file les rappels de la journée d'une inscription, pour l'heure de
/// début actuelle. Rejouable : la clé porte l'heure visée, une journée
/// déplacée donne de nouveaux travaux et les anciens ne partent pas.
pub async fn poser_rappels(
    pool: &sqlx::PgPool,
    conn: &mut PgConnection,
    track_registration_id: Uuid,
) -> Result<usize> {
    let Some(j) = journees::journee_pour_courriel(pool, track_registration_id).await? else {
        return Ok(0);
    };
    let Some(debut) = j.debut else {
        return Ok(0);
    };
    if !matches!(j.status.as_str(), "registered" | "attended") {
        return Ok(0);
    }
    let maintenant = OffsetDateTime::now_utc();
    let mut poses = 0;
    for minutes in journees::decalages_de_journee(pool, j.event_id).await? {
        let quand = debut - time::Duration::minutes(minutes);
        if quand <= maintenant {
            continue;
        }
        let charge = ChargeRappel {
            track_registration_id,
            offset_minutes: minutes,
            starts_at: debut.unix_timestamp(),
        };
        let job = NewJob {
            queue: EMAIL_QUEUE,
            ..NewJob::new(
                SEND_DAY_REMINDER,
                serde_json::to_value(&charge).map_err(ApiError::internal)?,
            )
        }
        .idempotent(format!(
            "{track_registration_id}:{minutes}:{}",
            debut.unix_timestamp()
        ))
        .at(quand);
        if jobs::enqueue(conn, job).await?.is_some() {
            poses += 1;
        }
    }
    Ok(poses)
}

pub struct SendDayReminder {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendDayReminder {
    pub fn new(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            db,
            mailer,
            app_public_url: config.app_public_url.clone(),
        }
    }
}

#[async_trait]
impl JobHandler for SendDayReminder {
    fn task(&self) -> &'static str {
        SEND_DAY_REMINDER
    }

    fn queue(&self) -> &'static str {
        EMAIL_QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let c: ChargeRappel = serde_json::from_value(job.payload.clone())
            .map_err(|_| ApiError::internal("charge de rappel de journée illisible"))?;
        let pool = self.db.pool();
        let Some(j) = journees::journee_pour_courriel(pool, c.track_registration_id).await? else {
            return Ok(());
        };
        if !matches!(j.status.as_str(), "registered" | "attended") {
            return Ok(());
        }
        // La journée a bougé : le travail de la nouvelle heure s'en charge.
        if j.debut.map(|d| d.unix_timestamp()) != Some(c.starts_at) {
            return Ok(());
        }
        if !delivery::canal_autorise(pool, j.person_id, TYPE_RAPPEL, "email").await? {
            return Ok(());
        }
        let mail = rappel(
            &j,
            c.offset_minutes,
            &self.app_public_url,
            &job.id.to_string(),
        );
        self.mailer
            .send(&mail)
            .await
            .map_err(|_| ApiError::internal("envoi du rappel de journée refusé par le relais"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journee(locale: &str) -> JourneePourCourriel {
        JourneePourCourriel {
            track_registration_id: Uuid::nil(),
            status: "registered".to_owned(),
            person_id: Uuid::nil(),
            first_name: "Awa".to_owned(),
            email: "awa@example.org".to_owned(),
            locale: locale.to_owned(),
            event_id: Uuid::nil(),
            titre: "Journée finance durable".to_owned(),
            event_slug: "cdp31".to_owned(),
            track_slug: "finance".to_owned(),
            timezone: "Europe/Istanbul".to_owned(),
            debut: None,
            debut_local: Some("13/11/2026 à 08h00".to_owned()),
            gardees: 6,
            activites: 8,
        }
    }

    #[test]
    fn un_invite_recoit_ses_deux_liens_avec_le_jeton() {
        let m = confirmation(&journee("fr"), Some("abc"), false, "https://site.org/", "j");
        assert!(m
            .text
            .contains("https://site.org/programmations/cdp31/journees/finance/direct?jeton=abc"));
        assert!(m.text.contains(
            "https://site.org/programmations/cdp31/journees/finance/mon-inscription?jeton=abc"
        ));
        assert!(m.text.contains("6 activité(s) sur 8"));
        assert!(m.text.contains("(Europe/Istanbul)"));
        assert!(!m.text.contains('\u{2014}'));
    }

    #[test]
    fn en_anglais_les_pages_sont_celles_du_site_anglais() {
        let m = confirmation(&journee("en"), Some("abc"), true, "https://site.org", "j");
        assert!(m.subject.starts_with("Your link for"));
        assert!(m
            .text
            .contains("https://site.org/en/programmes/cdp31/days/finance/live?jeton=abc"));
    }

    #[test]
    fn avec_un_compte_le_lien_mene_a_la_page_de_la_journee() {
        let m = confirmation(&journee("fr"), None, false, "https://site.org", "j");
        assert!(!m.text.contains("jeton"));
        assert!(m
            .text
            .contains("https://site.org/programmations/cdp31/journees/finance"));
    }

    #[test]
    fn le_rappel_dit_demain_pour_un_jour() {
        let m = rappel(&journee("fr"), 1440, "https://site.org", "j");
        assert_eq!(m.subject, "Journée finance durable commence demain");
        let m = rappel(&journee("en"), 2880, "https://site.org", "j");
        assert_eq!(m.subject, "Journée finance durable starts in 2 days");
    }
}
