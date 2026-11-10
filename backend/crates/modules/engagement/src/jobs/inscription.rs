//! La confirmation d'une inscription à une activité seule (02/11).
//!
//! Zoom n'envoie plus la sienne : c'est ce courriel qui porte le lien du
//! direct, la salle du site (SDK), jamais le lien Zoom. Mis en file par le
//! consommateur des avis à la création de l'inscription ; une activité gardée
//! d'une inscription à la journée n'a pas la sienne, celle de la journée suffit.
//!
//! Sans compte, la salle ne reconnaît personne : le module Programmation met en
//! file une confirmation qui porte le jeton en clair, vers la page qui entre
//! directement dans Zoom. Celle du consommateur s'efface alors devant elle.

use async_trait::async_trait;
use contracts::engagement::{RegistrationEmail, EMAIL_QUEUE, SEND_REGISTRATION};
use kernel::config::Config;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::events::OutboxEvent;
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::mail::{Extras, Mailer, OutgoingMail};
use sqlx::postgres::PgConnection;
use std::sync::Arc;

use crate::domain::courriel_direct::{self, CourrielDirect};
use crate::domain::gabarits::{images_integrees, Visuels};
use crate::jobs::send_mailing::apparence;
use crate::repo::delivery;
use crate::repo::journees::{self, ActivitePourCourriel};

const TYPE_CONFIRMATION: &str = "programme.registration.confirmed";
/// La liste d'attente a sa place chez Zoom : le jour même, les premiers arrivés entrent.
const ETATS: [&str; 2] = ["registered", "waitlisted"];

/// Met en file la confirmation d'une inscription qui vient d'être créée.
pub async fn poser(conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
    let p = &event.payload;
    let de_journee = p.get("track_registration_id").is_some_and(|v| !v.is_null());
    let statut = p
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if de_journee || !ETATS.contains(&statut) {
        return Ok(());
    }
    let job = NewJob {
        queue: EMAIL_QUEUE,
        ..NewJob::new(
            SEND_REGISTRATION,
            serde_json::json!({ "registration_id": event.aggregate_id }),
        )
    }
    .idempotent(format!("{}:confirmation", event.aggregate_id));
    jobs::enqueue(conn, job).await?;
    Ok(())
}

/// Le lien du bouton : la salle de l'activité, qui entre sans second clic, ou
/// la page de sa journée, qui porte le direct.
fn lien(app: &str, a: &ActivitePourCourriel, jeton: Option<&str>) -> String {
    let base = app.trim_end_matches('/');
    let anglais = a.locale.starts_with("en");
    match (&a.track_slug, anglais) {
        (Some(t), false) => format!("{base}/programmations/{}/journees/{t}", a.event_slug),
        (Some(t), true) => format!("{base}/en/programmes/{}/days/{t}", a.event_slug),
        (None, false) if jeton.is_some() => format!(
            "{base}/programmations/{}/{}/direct?jeton={}",
            a.event_slug,
            a.session_slug,
            jeton.unwrap_or_default()
        ),
        (None, true) if jeton.is_some() => format!(
            "{base}/en/programmes/{}/{}/live?jeton={}",
            a.event_slug,
            a.session_slug,
            jeton.unwrap_or_default()
        ),
        (None, false) => format!(
            "{base}/programmations/{}/{}/salle?entrer=1",
            a.event_slug, a.session_slug
        ),
        (None, true) => format!(
            "{base}/en/programmes/{}/{}/room?entrer=1",
            a.event_slug, a.session_slug
        ),
    }
}

pub fn confirmation(
    a: &ActivitePourCourriel,
    jeton: Option<&str>,
    renvoi: bool,
    app: &str,
    message_id: &str,
    visuels: Option<&Visuels>,
) -> OutgoingMail {
    let anglais = a.locale.starts_with("en");
    let attente = a.status == "waitlisted";
    let quand = a
        .debut_local
        .as_deref()
        .map(|d| format!("{d} ({})", a.timezone))
        .unwrap_or_default();
    let mut avant = Vec::new();
    if anglais {
        avant.push(format!("Hello {},", a.first_name));
        avant.push(format!(
            "{} \"{}\" starts on {quand}.",
            if renvoi {
                "Here is your link again."
            } else {
                "Your registration is confirmed."
            },
            a.titre
        ));
        if attente {
            avant.push(
                "You are on the waiting list: on the day, the first to arrive get in.".to_owned(),
            );
        }
        avant.push("Join the live stream on the day:".to_owned());
    } else {
        avant.push(format!("Bonjour {},", a.first_name));
        avant.push(format!(
            "{} « {} » commence le {quand}.",
            if renvoi {
                "Voici de nouveau votre lien."
            } else {
                "Votre inscription est confirmée."
            },
            a.titre
        ));
        if attente {
            avant.push(
                "Vous êtes sur liste d'attente : le jour même, les premiers arrivés entrent."
                    .to_owned(),
            );
        }
        avant.push("Rejoignez le direct le jour même :".to_owned());
    }
    let c = CourrielDirect {
        anglais,
        sujet: match (anglais, renvoi) {
            (true, true) => format!("Your link for {}", a.titre),
            (true, false) => format!("Registration confirmed: {}", a.titre),
            (false, true) => format!("Votre lien pour {}", a.titre),
            (false, false) => format!("Inscription confirmée : {}", a.titre),
        },
        avant,
        lien: lien(app, a, jeton),
        apres: jeton
            .map(|_| {
                if anglais {
                    "This link is personal: please do not forward it.".to_owned()
                } else {
                    "Ce lien est personnel : merci de ne pas le transférer.".to_owned()
                }
            })
            .into_iter()
            .collect(),
    };
    courriel_direct::composer(&c, visuels, &a.email, &a.locale, message_id)
}

pub struct SendRegistration {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendRegistration {
    pub fn new(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            db,
            mailer,
            app_public_url: config.app_public_url.clone(),
        }
    }
}

#[async_trait]
impl JobHandler for SendRegistration {
    fn task(&self) -> &'static str {
        SEND_REGISTRATION
    }

    fn queue(&self) -> &'static str {
        EMAIL_QUEUE
    }

    fn carries_secret(&self) -> bool {
        true
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let c: RegistrationEmail = serde_json::from_value(job.payload.clone())
            .map_err(|_| ApiError::internal("charge sans registration_id"))?;
        let pool = self.db.pool();
        let Some(a) = journees::activite_pour_courriel(pool, c.registration_id).await? else {
            return Ok(());
        };
        if a.track_registration_id.is_some() || !ETATS.contains(&a.status.as_str()) {
            return Ok(());
        }
        if a.lien_personnel && c.token.is_none() {
            return Ok(());
        }
        if !delivery::canal_autorise(pool, a.person_id, TYPE_CONFIRMATION, "email").await? {
            return Ok(());
        }
        let habit = apparence(
            &self.db,
            a.event_id,
            courriel_direct::GABARIT,
            None,
            &self.app_public_url,
        )
        .await?;
        let mail = confirmation(
            &a,
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
                ApiError::internal("envoi de la confirmation d'inscription refusé par le relais")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn activite(locale: &str, journee: Option<&str>) -> ActivitePourCourriel {
        ActivitePourCourriel {
            status: "registered".to_owned(),
            person_id: Uuid::nil(),
            first_name: "Awa".to_owned(),
            email: "awa@example.org".to_owned(),
            locale: locale.to_owned(),
            event_id: Uuid::nil(),
            titre: "Finance climat".to_owned(),
            event_slug: "cdp31".to_owned(),
            session_slug: "finance-climat".to_owned(),
            timezone: "Europe/Istanbul".to_owned(),
            debut_local: Some("13/11/2026 à 08h00".to_owned()),
            track_registration_id: None,
            track_slug: journee.map(str::to_owned),
            lien_personnel: false,
        }
    }

    #[test]
    fn le_bouton_mene_a_la_salle_de_l_activite() {
        let m = confirmation(
            &activite("fr", None),
            None,
            false,
            "https://site.org/",
            "j",
            None,
        );
        assert!(m
            .text
            .contains("https://site.org/programmations/cdp31/finance-climat/salle?entrer=1"));
        assert!(m.text.contains("(Europe/Istanbul)"));
        let m = confirmation(
            &activite("en", None),
            None,
            false,
            "https://site.org",
            "j",
            None,
        );
        assert!(m
            .text
            .contains("https://site.org/en/programmes/cdp31/finance-climat/room?entrer=1"));
    }

    #[test]
    fn une_activite_de_journee_mene_a_la_journee() {
        let m = confirmation(
            &activite("fr", Some("jeunesse")),
            None,
            false,
            "https://site.org",
            "j",
            None,
        );
        assert!(m
            .text
            .contains("https://site.org/programmations/cdp31/journees/jeunesse\n"));
    }

    #[test]
    fn sans_compte_le_bouton_porte_le_jeton() {
        let m = confirmation(
            &activite("fr", None),
            Some("abc"),
            false,
            "https://site.org",
            "j",
            None,
        );
        assert!(m
            .text
            .contains("https://site.org/programmations/cdp31/finance-climat/direct?jeton=abc"));
        assert!(m.text.contains("personnel"));
        let m = confirmation(
            &activite("en", None),
            Some("abc"),
            true,
            "https://site.org",
            "j",
            None,
        );
        assert!(m
            .text
            .contains("https://site.org/en/programmes/cdp31/finance-climat/live?jeton=abc"));
        assert!(m.subject.starts_with("Your link"));
    }
}
