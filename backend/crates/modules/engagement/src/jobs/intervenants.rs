//! Les courriels des intervenants en ligne (08/11), `075_programme_sessions.sql`
//! § 2 bis.
//!
//! `engagement.send_speaker_zoom_link` : le lien Zoom personnel, qui ouvre
//! l'application, à l'intervenant. Mis en file par le module Live dès que la
//! place existe chez Zoom.
//!
//! `engagement.send_speaker_signup_link` : le lien d'inscription des
//! intervenants au porteur de l'activité, qui le transmet. Mis en file par la
//! régie ou le balayage de l'avant-veille. Un seul courriel : le premier
//! porteur en destinataire, les autres en copie.
//!
//! L'un et l'autre en copie aux adresses du réglage `engagement.speaker_links.cc`
//! (demande du commanditaire) ; hors de la copie automatique (`SANS_COPIE`).
//! `preview_to` : aperçu pour validation, à cette seule adresse, sans copie.

use async_trait::async_trait;
use contracts::engagement::{
    SpeakerSignupLinkEmail, SpeakerZoomLinkEmail, EMAIL_QUEUE, SEND_SPEAKER_SIGNUP_LINK,
    SEND_SPEAKER_ZOOM_LINK,
};
use kernel::config::Config;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use kernel::mail::{Extras, Mailer, OutgoingMail};
use serde_json::Value;
use std::sync::Arc;
use time::{PrimitiveDateTime, Weekday};

use crate::domain::courriel_direct::{self, Assemblage, Paragraphe};
use crate::domain::dossier::{date_longue, heure_de, nom_du_fuseau};
use crate::domain::gabarits::{images_integrees, Visuels};
use crate::jobs::send_mailing::apparence;
use crate::repo::intervenants::{self as lecture, ActiviteDuLien};

const LIEN_DEXEMPLE: &str = "https://us06web.zoom.us/w/0000000000?tk=exemple";

/// Le titre dans la langue du courriel, à défaut en français.
fn titre(t: &Value, anglais: bool) -> String {
    let langue = if anglais { "en" } else { "fr" };
    t.get(langue)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .or_else(|| t.get("fr").and_then(Value::as_str))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn hh_mm(t: PrimitiveDateTime) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}

fn date_anglaise(t: PrimitiveDateTime) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let jour = match t.weekday() {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    };
    format!(
        "{jour} {} {} {}",
        t.day(),
        MONTHS[usize::from(u8::from(t.month())) - 1],
        t.year()
    )
}

/// « le vendredi 13 novembre 2026, de 11:30 à 12:30, heure d'Antalya ».
pub fn quand(a: &ActiviteDuLien, anglais: bool) -> String {
    let zone = nom_du_fuseau(a.city.as_deref(), &a.timezone);
    if anglais {
        format!(
            "on {}, from {} to {}, {zone} time",
            date_anglaise(a.debut),
            hh_mm(a.debut),
            hh_mm(a.fin)
        )
    } else {
        format!(
            "le {}, de {} à {}, {}",
            date_longue(a.debut.date()),
            hh_mm(a.debut),
            hh_mm(a.fin),
            heure_de(&zone)
        )
    }
}

/// « Madame Diallo, », « Monsieur Mbala, », sinon « Madame, Monsieur, ».
pub fn appel(civilite: Option<&str>, nom: &str, anglais: bool) -> String {
    let nom = nom.trim();
    match (civilite, anglais, nom.is_empty()) {
        (Some("mme"), false, false) => format!("Madame {nom},"),
        (Some("m"), false, false) => format!("Monsieur {nom},"),
        (Some("mme"), true, false) => format!("Dear Ms {nom},"),
        (Some("m"), true, false) => format!("Dear Mr {nom},"),
        (Some("dr"), true, false) => format!("Dear Dr {nom},"),
        (Some("pr"), true, false) => format!("Dear Professor {nom},"),
        (_, true, _) => "Dear Sir or Madam,".to_owned(),
        _ => "Madame, Monsieur,".to_owned(),
    }
}

fn prefixe(a: &ActiviteDuLien) -> String {
    a.acronym
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("{s} : "))
        .unwrap_or_default()
}

fn p(textes: &[String]) -> Vec<Paragraphe> {
    textes.iter().map(|t| Paragraphe::brut(t)).collect()
}

pub struct CourrielIntervenant<'a> {
    pub activite: &'a ActiviteDuLien,
    pub appel: String,
    pub lien: &'a str,
    pub anglais: bool,
}

pub fn lien_zoom(
    c: &CourrielIntervenant<'_>,
    visuels: Option<&Visuels>,
    to: &str,
    message_id: &str,
) -> OutgoingMail {
    let a = c.activite;
    let t = titre(&a.title, c.anglais);
    let (sujet, avant, action, apres) = if c.anglais {
        (
            format!(
                "{}your Zoom link as a speaker, {} at {}",
                prefixe(a).replace(" : ", ": "),
                date_anglaise(a.debut),
                hh_mm(a.debut)
            ),
            vec![
                c.appel.clone(),
                format!("You are speaking in \"{t}\", {}.", quand(a, true)),
                "If you speak online, use the button below: it opens the Zoom app under your name. This link is personal, please do not share it. If you are on site, you can ignore this message.".to_owned(),
            ],
            "Join the Zoom meeting",
            vec!["Join a few minutes before the start: the technical team will give you the floor.".to_owned()],
        )
    } else {
        (
            format!(
                "{}votre lien Zoom d'intervention, le {} à {}",
                prefixe(a),
                date_longue(a.debut.date()),
                hh_mm(a.debut)
            ),
            vec![
                c.appel.clone(),
                format!("Vous intervenez dans « {t} », {}.", quand(a, false)),
                "Si vous intervenez en ligne, utilisez le bouton ci-dessous : il ouvre l'application Zoom à votre nom. Ce lien vous est personnel, merci de ne pas le partager. Si vous êtes sur place, vous pouvez ignorer ce message.".to_owned(),
            ],
            "Rejoindre la réunion Zoom",
            vec!["Connectez-vous quelques minutes avant le début : l'équipe technique vous donnera la parole.".to_owned()],
        )
    };
    courriel_direct::assembler(
        &Assemblage {
            anglais: c.anglais,
            sujet: &sujet,
            avant: &p(&avant),
            lien: c.lien,
            action,
            apres: &p(&apres),
        },
        visuels,
        to,
        if c.anglais { "en" } else { "fr" },
        message_id,
    )
}

pub fn lien_dinscription(
    a: &ActiviteDuLien,
    lien: &str,
    anglais: bool,
    visuels: Option<&Visuels>,
    to: &str,
    message_id: &str,
) -> OutgoingMail {
    let t = titre(&a.title, anglais);
    let (sujet, avant, action, apres) = if anglais {
        (
            format!("{}register your online speakers for \"{t}\"", prefixe(a).replace(" : ", ": ")),
            vec![
                "Dear Sir or Madam,".to_owned(),
                format!("Your activity \"{t}\" takes place {}. Speakers who will take part online must register through the link below: each of them immediately receives a personal Zoom link.", quand(a, true)),
                "Please forward it to each of them. Speakers present on site have nothing to do.".to_owned(),
            ],
            "Register an online speaker",
            vec!["This link works until the end of the activity.".to_owned()],
        )
    } else {
        (
            format!("{}inscrivez vos intervenants en ligne pour « {t} »", prefixe(a)),
            vec![
                "Madame, Monsieur,".to_owned(),
                format!("Votre activité « {t} » a lieu {}. Les intervenants qui la suivront en ligne doivent s'inscrire par le lien ci-dessous : chacun recevra aussitôt son lien Zoom personnel.", quand(a, false)),
                "Merci de le transmettre à chacun d'eux. Les intervenants présents sur place n'ont rien à faire.".to_owned(),
            ],
            "Inscrire un intervenant en ligne",
            vec!["Ce lien sert jusqu'à la fin de l'activité.".to_owned()],
        )
    };
    courriel_direct::assembler(
        &Assemblage {
            anglais,
            sujet: &sujet,
            avant: &p(&avant),
            lien,
            action,
            apres: &p(&apres),
        },
        visuels,
        to,
        if anglais { "en" } else { "fr" },
        message_id,
    )
}

fn apercu(mut mail: OutgoingMail, anglais: bool) -> OutgoingMail {
    mail.subject = format!(
        "{} {}",
        if anglais { "[Preview]" } else { "[Aperçu]" },
        mail.subject
    );
    mail
}

pub struct SendSpeakerZoomLink {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendSpeakerZoomLink {
    pub fn new(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            db,
            mailer,
            app_public_url: config.app_public_url.clone(),
        }
    }
}

#[async_trait]
impl JobHandler for SendSpeakerZoomLink {
    fn task(&self) -> &'static str {
        SEND_SPEAKER_ZOOM_LINK
    }

    fn queue(&self) -> &'static str {
        EMAIL_QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: SpeakerZoomLinkEmail =
            serde_json::from_value(job.payload.clone()).map_err(ApiError::internal)?;
        let pool = self.db.pool();
        let Some(a) = lecture::activite(pool, charge.session_id).await? else {
            return Ok(());
        };
        let habit = apparence(
            &self.db,
            a.event_id,
            courriel_direct::GABARIT,
            None,
            &self.app_public_url,
        )
        .await?;
        let message_id = job.id.to_string();
        let (mail, cc) = match (&charge.preview_to, charge.participant_id) {
            (Some(to), _) => {
                let c = CourrielIntervenant {
                    activite: &a,
                    appel: appel(Some("mme"), "Exemple", false),
                    lien: LIEN_DEXEMPLE,
                    anglais: false,
                };
                (
                    apercu(
                        lien_zoom(&c, habit.visuels.as_ref(), to, &message_id),
                        false,
                    ),
                    Vec::new(),
                )
            }
            (None, Some(participant)) => {
                let Some(place) = lecture::place(pool, participant).await? else {
                    return Ok(());
                };
                let anglais = place.locale.starts_with("en");
                let c = CourrielIntervenant {
                    activite: &a,
                    appel: appel(place.civility.as_deref(), &place.last_name, anglais),
                    lien: &place.join_url,
                    anglais,
                };
                (
                    lien_zoom(&c, habit.visuels.as_ref(), &place.email, &message_id),
                    lecture::copie(pool).await?,
                )
            }
            (None, None) => return Ok(()),
        };
        let extras = Extras {
            cc,
            inline_images: images_integrees(&habit.gabarit, habit.visuels.as_ref()),
            ..Extras::default()
        };
        self.mailer
            .send_with_extras(&mail, &extras)
            .await
            .map_err(|_| {
                ApiError::internal("envoi du lien Zoom d'intervenant refusé par le relais")
            })
    }
}

pub struct SendSpeakerSignupLink {
    db: Db,
    mailer: Arc<dyn Mailer>,
    app_public_url: String,
}

impl SendSpeakerSignupLink {
    pub fn new(db: Db, config: &Config, mailer: Arc<dyn Mailer>) -> Self {
        Self {
            db,
            mailer,
            app_public_url: config.app_public_url.clone(),
        }
    }
}

#[async_trait]
impl JobHandler for SendSpeakerSignupLink {
    fn task(&self) -> &'static str {
        SEND_SPEAKER_SIGNUP_LINK
    }

    fn queue(&self) -> &'static str {
        EMAIL_QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: SpeakerSignupLinkEmail =
            serde_json::from_value(job.payload.clone()).map_err(ApiError::internal)?;
        let pool = self.db.pool();
        let Some(a) = lecture::activite(pool, charge.session_id).await? else {
            return Ok(());
        };
        let habit = apparence(
            &self.db,
            a.event_id,
            courriel_direct::GABARIT,
            None,
            &self.app_public_url,
        )
        .await?;
        let message_id = job.id.to_string();
        let (to, mut cc, anglais) = match &charge.preview_to {
            Some(to) => (to.clone(), Vec::new(), false),
            None => {
                let porteurs = lecture::porteurs(pool, charge.session_id).await?;
                let Some((premier, autres)) = porteurs.split_first() else {
                    return Ok(());
                };
                let mut cc: Vec<String> = autres.iter().map(|d| d.email.clone()).collect();
                cc.extend(lecture::copie(pool).await?);
                (premier.email.clone(), cc, premier.locale.starts_with("en"))
            }
        };
        cc.retain(|c| !c.eq_ignore_ascii_case(&to));
        cc.dedup();
        let base = self.app_public_url.trim_end_matches('/');
        let lien = if anglais {
            format!("{base}/en/speaker?jeton={}", a.token)
        } else {
            format!("{base}/intervenant?jeton={}", a.token)
        };
        let mut mail =
            lien_dinscription(&a, &lien, anglais, habit.visuels.as_ref(), &to, &message_id);
        if charge.preview_to.is_some() {
            mail = apercu(mail, anglais);
        }
        let extras = Extras {
            cc,
            inline_images: images_integrees(&habit.gabarit, habit.visuels.as_ref()),
            ..Extras::default()
        };
        self.mailer
            .send_with_extras(&mail, &extras)
            .await
            .map_err(|_| {
                ApiError::internal(
                    "envoi du lien d'inscription des intervenants refusé par le relais",
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;
    use uuid::Uuid;

    fn activite() -> ActiviteDuLien {
        ActiviteDuLien {
            event_id: Uuid::nil(),
            acronym: Some("CdP31".into()),
            title: serde_json::json!({ "fr": "Autorité Nationale de Budgétisation Climat" }),
            debut: datetime!(2026-11-13 11:30),
            fin: datetime!(2026-11-13 12:30),
            timezone: "Europe/Istanbul".into(),
            city: Some("Antalya".into()),
            token: "abc".into(),
        }
    }

    #[test]
    fn la_date_porte_son_fuseau() {
        assert_eq!(
            quand(&activite(), false),
            "le vendredi 13 novembre 2026, de 11:30 à 12:30, heure d'Antalya"
        );
        assert_eq!(
            quand(&activite(), true),
            "on Friday 13 November 2026, from 11:30 to 12:30, Antalya time"
        );
    }

    #[test]
    fn le_lien_zoom_suit_le_bouton() {
        let a = activite();
        let c = CourrielIntervenant {
            activite: &a,
            appel: appel(Some("mme"), "Diallo", false),
            lien: "https://zoom.us/w/1?tk=x",
            anglais: false,
        };
        let m = lien_zoom(&c, None, "a@b.org", "j");
        assert!(m.subject.starts_with("CdP31 : votre lien Zoom"));
        assert!(m.text.contains("Madame Diallo,"));
        assert!(m.text.contains("https://zoom.us/w/1?tk=x"));
    }

    #[test]
    fn le_porteur_recoit_le_lien_dinscription() {
        let m = lien_dinscription(
            &activite(),
            "https://site/intervenant?jeton=abc",
            false,
            None,
            "p@o.org",
            "j",
        );
        assert!(m.subject.contains("inscrivez vos intervenants en ligne"));
        assert!(m.text.contains("https://site/intervenant?jeton=abc"));
        assert!(appel(None, "X", false) == "Madame, Monsieur,");
    }
}
