//! Les courriels d'une inscription à une journée (02/11).
//!
//! - **La confirmation** (`engagement.send_day_registration`), mise en file par
//!   le module Programmation dans la transaction de l'inscription. Sans compte,
//!   elle porte le jeton en clair dans le lien du direct : le travail le déclare
//!   (`carries_secret`), sa charge est vidée à la réussite comme à la mort.
//!   Zoom n'envoie pas sa propre confirmation : c'est ce courriel qui porte le
//!   lien du direct ([`courriel_direct`]).
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
use kernel::mail::{Extras, Mailer, OutgoingMail};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::courriel_direct::{self, CourrielDirect};
use crate::domain::gabarits::{images_integrees, Visuels};
use crate::domain::programmes::{self, CleEnvoi, Nature};
use crate::jobs::send_mailing::apparence;
use crate::repo::delivery;
use crate::repo::journees::{self, JourneePourCourriel};
use crate::repo::programmes as programmes_repo;
use crate::service::programmes::{composer, texte_ecrit, RappelProgramme};

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

/// Les activités suivies, en un paragraphe. Une journée de moins de deux
/// activités n'offre rien à choisir : le paragraphe est vide.
fn suivi(anglais: bool, n: i64, m: i64) -> String {
    let phrase = match (anglais, m < 2, n) {
        (_, true, _) => return String::new(),
        (false, _, 0) => "Vous n'avez gardé aucune activité en particulier : le direct reste ouvert toute la journée.".to_owned(),
        (false, _, 1) => format!("Vous suivez 1 activité sur {m} : vous recevrez un rappel avant son début."),
        (false, _, n) => format!("Vous suivez {n} activités sur {m} : vous recevrez un rappel avant le début de chacune."),
        (true, _, 0) => "You have not kept any activity in particular: the live stream stays open all day.".to_owned(),
        (true, _, 1) => format!("You are following 1 of {m} activities: you will get a reminder before it starts."),
        (true, _, n) => format!("You are following {n} of {m} activities: you will get a reminder before each of them starts."),
    };
    phrase
}

/// La confirmation, en français ou en anglais selon la langue d'inscription.
/// Sans lien de gestion (02/11) : le courriel ne mène qu'au direct.
pub fn confirmation(
    j: &JourneePourCourriel,
    jeton: Option<&str>,
    renvoi: bool,
    app: &str,
    message_id: &str,
    visuels: Option<&Visuels>,
) -> OutgoingMail {
    let anglais = j.locale.starts_with("en");
    let lien = match jeton {
        Some(t) => format!("{}?jeton={t}", page(app, j, Some(("direct", "live")))),
        None => page(app, j, None),
    };
    let quand = j
        .debut_local
        .as_deref()
        .map(|d| format!("{d} ({})", j.timezone))
        .unwrap_or_default();
    let suivi = suivi(anglais, j.gardees, j.activites);

    let c = if anglais {
        CourrielDirect {
            anglais,
            sujet: if renvoi {
                format!("Your link for {}", j.titre)
            } else {
                format!("Registration confirmed: {}", j.titre)
            },
            avant: [
                format!("Hello {},", j.first_name),
                format!(
                    "{} \"{}\" starts on {quand}. One link is all you need to follow the whole day live, from the welcome to the closing.",
                    if renvoi { "Here is your link again." } else { "Your registration is confirmed." },
                    j.titre
                ),
                suivi,
                "Join the live stream of the day, on the day:".to_owned(),
            ]
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect(),
            lien,
            apres: jeton
                .map(|_| "This link is personal: please do not forward it.".to_owned())
                .into_iter()
                .collect(),
        }
    } else {
        CourrielDirect {
            anglais,
            sujet: if renvoi {
                format!("Votre lien pour {}", j.titre)
            } else {
                format!("Inscription confirmée : {}", j.titre)
            },
            avant: [
                format!("Bonjour {},", j.first_name),
                format!(
                    "{} « {} » commence le {quand}. Un seul lien suffit pour suivre tout le direct de la journée, de l'accueil à la clôture.",
                    if renvoi { "Voici de nouveau votre lien." } else { "Votre inscription est confirmée." },
                    j.titre
                ),
                suivi,
                "Rejoignez le direct de la journée, le jour même :".to_owned(),
            ]
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect(),
            lien,
            apres: jeton
                .map(|_| "Ce lien vous est personnel : ne le transférez pas.".to_owned())
                .into_iter()
                .collect(),
        }
    };
    courriel_direct::composer(&c, visuels, &j.email, &j.locale, message_id)
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
        let habit = apparence(
            &self.db,
            j.event_id,
            courriel_direct::GABARIT,
            None,
            &self.app_public_url,
        )
        .await?;
        let mail = confirmation(
            &j,
            c.token.as_deref(),
            c.resent,
            &self.app_public_url,
            &job.id.to_string(),
            habit.visuels.as_ref(),
        );
        let extras = Extras {
            inline_images: images_integrees(&habit.gabarit, habit.visuels.as_ref()),
            ..Extras::default()
        };
        self.mailer
            .send_with_extras(&mail, &extras)
            .await
            .map_err(|_| {
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
        let debut = OffsetDateTime::from_unix_timestamp(c.starts_at)
            .map_err(|_| ApiError::internal("début de journée illisible"))?;
        let Some(cible) = programmes_repo::cible_journee(
            pool,
            None,
            Some(j.track_registration_id),
            debut,
            &j.locale,
        )
        .await?
        else {
            return Ok(());
        };
        let cle = CleEnvoi {
            nature: Nature::Journee,
            cible: cible.cible,
            offset_minutes: i32::try_from(c.offset_minutes)
                .map_err(|_| ApiError::internal("décalage de rappel illisible"))?,
            debut: Some(c.starts_at),
        };
        let texte = texte_ecrit(pool, j.event_id, &cle, programmes::langue(&j.locale))
            .await?
            .unwrap_or_else(|| {
                programmes::texte_par_defaut(Nature::Journee, programmes::langue(&j.locale) == "en")
            });
        let habit = apparence(
            &self.db,
            j.event_id,
            courriel_direct::GABARIT,
            None,
            &self.app_public_url,
        )
        .await?;
        let mail = composer(&RappelProgramme {
            nature: Nature::Journee,
            cible: &cible,
            prenom: &j.first_name,
            to: &j.email,
            locale: &j.locale,
            offset_minutes: c.offset_minutes,
            texte: &texte,
            site: &self.app_public_url,
            visuels: habit.visuels.as_ref(),
            message_id: &job.id.to_string(),
            surligner: false,
        })?;
        let extras = Extras {
            inline_images: images_integrees(&habit.gabarit, habit.visuels.as_ref()),
            ..Extras::default()
        };
        self.mailer
            .send_with_extras(&mail, &extras)
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
    fn un_invite_recoit_le_lien_du_direct_avec_le_jeton_et_rien_d_autre() {
        let m = confirmation(
            &journee("fr"),
            Some("abc"),
            false,
            "https://site.org/",
            "j",
            None,
        );
        assert!(m
            .text
            .contains("https://site.org/programmations/cdp31/journees/finance/direct?jeton=abc"));
        assert!(!m.text.contains("mon-inscription"));
        assert!(!m.text.contains("désinscrire"));
        assert!(m.text.contains("Vous suivez 6 activités sur 8"));
        assert!(m.text.contains("(Europe/Istanbul)"));
        assert!(m.html.unwrap().contains(">Rejoindre le direct</a>"));
    }

    #[test]
    fn une_journee_sans_choix_ne_parle_pas_d_activites() {
        let mut j = journee("fr");
        j.gardees = 0;
        j.activites = 0;
        let m = confirmation(&j, None, false, "https://site.org/", "j", None);
        assert!(!m.text.contains("activité"));
    }

    #[test]
    fn en_anglais_les_pages_sont_celles_du_site_anglais() {
        let m = confirmation(
            &journee("en"),
            Some("abc"),
            true,
            "https://site.org",
            "j",
            None,
        );
        assert!(m.subject.starts_with("Your link for"));
        assert!(m
            .text
            .contains("https://site.org/en/programmes/cdp31/days/finance/live?jeton=abc"));
    }

    #[test]
    fn avec_un_compte_le_lien_mene_a_la_page_de_la_journee() {
        let m = confirmation(&journee("fr"), None, false, "https://site.org", "j", None);
        assert!(!m.text.contains("jeton"));
        assert!(m
            .text
            .contains("https://site.org/programmations/cdp31/journees/finance"));
    }
}
