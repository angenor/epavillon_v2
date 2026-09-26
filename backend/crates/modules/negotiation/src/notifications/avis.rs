//! Le texte d'un avis, FR et EN. Il commence par l'état (« Déplacée — »), finit
//! par « Sessions de négociation » (FR-028), et ne nomme jamais l'autrice d'un
//! signalement (SC-008). Les heures arrivent déjà dans le fuseau de la COP.

use contracts::negotiation::{Notification, NotificationSubject, NotificationText};
use time::Date;
use uuid::Uuid;

use crate::domain::reports::ReportReason;

const SIGNATURE_FR: &str = "Sessions de négociation";
const SIGNATURE_EN: &str = "Negotiation sessions";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etat {
    Deplacee,
    Annulee,
    SalleChangee,
    NonAnnoncee,
    /// « Autre chose » : le réseau signale, sans valeur à afficher.
    Signalee,
}

impl Etat {
    pub fn du_motif(motif: ReportReason) -> Self {
        match motif {
            ReportReason::Time => Self::Deplacee,
            ReportReason::Cancelled => Self::Annulee,
            ReportReason::Venue => Self::SalleChangee,
            ReportReason::Unannounced => Self::NonAnnoncee,
            ReportReason::Other => Self::Signalee,
        }
    }

    pub fn fr(self) -> &'static str {
        match self {
            Self::Deplacee => "Déplacée",
            Self::Annulee => "Annulée",
            Self::SalleChangee => "Salle changée",
            Self::NonAnnoncee => "Non annoncée",
            Self::Signalee => "Signalée",
        }
    }

    pub fn en(self) -> &'static str {
        match self {
            Self::Deplacee => "Moved",
            Self::Annulee => "Cancelled",
            Self::SalleChangee => "Room changed",
            Self::NonAnnoncee => "Unannounced",
            Self::Signalee => "Reported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origine {
    /// L'import a lu le changement à la source officielle (phase 4).
    Source,
    /// Un signalement validé par l'IFDD.
    Reseau,
}

/// Le titre de la session ou de la réunion, dans chaque langue.
#[derive(Debug, Clone, Copy)]
pub struct Sujet<'a> {
    pub fr: &'a str,
    pub en: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum Detail<'a> {
    Aucun,
    /// `heure` en « HH:MM », dans le fuseau de la COP.
    Heure {
        heure: &'a str,
        ville: Option<&'a str>,
    },
    Salle(&'a str),
    Reunion {
        jour: Date,
        heure: Option<&'a str>,
        lieu: Option<&'a str>,
    },
}

pub type Texte = (NotificationText, NotificationText);

fn texte(fr: String, en: String) -> NotificationText {
    NotificationText { fr, en }
}

const MOIS_FR: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

pub(crate) fn jour_fr(jour: Date) -> String {
    format!(
        "{} {}",
        jour.day(),
        MOIS_FR[usize::from(u8::from(jour.month())) - 1]
    )
}

pub(crate) fn jour_en(jour: Date) -> String {
    format!("{} {}", jour.month(), jour.day())
}

fn detail(etat: Etat, d: Detail<'_>) -> (String, String) {
    match d {
        Detail::Heure { heure, ville } => match ville {
            Some(v) => (
                format!("nouvelle heure {heure} (heure de {v})"),
                format!("new time {heure} ({v} time)"),
            ),
            None => (
                format!("nouvelle heure {heure}"),
                format!("new time {heure}"),
            ),
        },
        Detail::Salle(salle) => (
            format!("nouvelle salle : {salle}"),
            format!("new room: {salle}"),
        ),
        Detail::Reunion { jour, heure, lieu } => {
            let mut fr = jour_fr(jour);
            let mut en = jour_en(jour);
            if let Some(h) = heure {
                fr.push_str(&format!(" à {h}"));
                en.push_str(&format!(" at {h}"));
            }
            if let Some(l) = lieu {
                fr.push_str(&format!(", {l}"));
                en.push_str(&format!(", {l}"));
            }
            (fr, en)
        }
        Detail::Aucun if etat == Etat::Annulee => (
            "la session n'aura pas lieu".into(),
            "the session will not take place".into(),
        ),
        Detail::Aucun => (
            "voir la fiche de la session".into(),
            "see the session page".into(),
        ),
    }
}

/// Un changement d'une session, ou une réunion non annoncée.
pub fn changement(etat: Etat, sujet: Sujet<'_>, d: Detail<'_>, origine: Origine) -> Texte {
    let (precision_fr, precision_en) = detail(etat, d);
    let (origine_fr, origine_en) = match origine {
        Origine::Reseau => (
            ", signalé par le réseau et validé par l'IFDD",
            ", reported by the network and validated by IFDD",
        ),
        Origine::Source => (", selon la source officielle", ", per the official source"),
    };
    (
        texte(
            format!("{} — {}", etat.fr(), sujet.fr),
            format!("{} — {}", etat.en(), sujet.en),
        ),
        texte(
            format!("{} — {precision_fr}{origine_fr}. {SIGNATURE_FR}", etat.fr()),
            format!("{} — {precision_en}{origine_en}. {SIGNATURE_EN}", etat.en()),
        ),
    )
}

/// Pour l'autrice : son signalement est affiché.
pub fn publie(sujet: Sujet<'_>) -> Texte {
    (
        texte(
            format!("Validé — {}", sujet.fr),
            format!("Validated — {}", sujet.en),
        ),
        texte(
            format!("Validé — votre signalement est affiché, sans votre nom. {SIGNATURE_FR}"),
            format!("Validated — your report is displayed, without your name. {SIGNATURE_EN}"),
        ),
    )
}

/// Pour l'autrice : son signalement n'est pas retenu, et pourquoi.
pub fn refuse(sujet: Sujet<'_>, motif: &str) -> Texte {
    let (fr, en) = match motif {
        "source_maintains" => (
            "la source officielle maintient l'information",
            "the official source maintains the information",
        ),
        "already_known" => (
            "le changement était déjà connu",
            "the change was already known",
        ),
        _ => (
            "le signalement n'est pas assez précis",
            "the report is not precise enough",
        ),
    };
    (
        texte(
            format!("Non retenu — {}", sujet.fr),
            format!("Not retained — {}", sujet.en),
        ),
        texte(
            format!("Non retenu — {fr}. {SIGNATURE_FR}"),
            format!("Not retained — {en}. {SIGNATURE_EN}"),
        ),
    )
}

/// `<type>:<id>:<jour>` : les avis d'une même cible et d'un même jour se
/// remplacent (research R8).
pub fn cle_du_jour(type_code: &str, id: Uuid, jour: Date) -> String {
    format!("{type_code}:{id}:{jour}")
}

pub struct Cible<'a> {
    pub table: &'a str,
    pub id: Uuid,
    pub lien: String,
}

pub fn notification(
    type_code: &str,
    destinataires: Vec<Uuid>,
    (title, body): Texte,
    cible: Cible<'_>,
    group_key: Option<String>,
    variables: serde_json::Value,
) -> Notification {
    Notification {
        type_code: type_code.to_owned(),
        recipients: destinataires,
        title,
        body,
        link_path: cible.lien,
        subject: NotificationSubject {
            schema: contracts::negotiation::AGGREGATE_SCHEMA.to_owned(),
            table: cible.table.to_owned(),
            id: cible.id,
        },
        replace: group_key.is_some(),
        group_key,
        variables,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    const SUJET: Sujet<'static> = Sujet {
        fr: "Objectif mondial d'adaptation",
        en: "Global goal on adaptation",
    };

    #[test]
    fn le_texte_commence_par_letat_et_finit_par_la_signature() {
        for (etat, d, debut) in [
            (
                Etat::Deplacee,
                Detail::Heure {
                    heure: "11:30",
                    ville: Some("Antalya"),
                },
                "Déplacée — ",
            ),
            (Etat::Annulee, Detail::Aucun, "Annulée — "),
            (
                Etat::SalleChangee,
                Detail::Salle("Salle 4"),
                "Salle changée — ",
            ),
            (
                Etat::NonAnnoncee,
                Detail::Reunion {
                    jour: date!(2026 - 11 - 10),
                    heure: Some("14:00"),
                    lieu: None,
                },
                "Non annoncée — ",
            ),
        ] {
            let (titre, corps) = changement(etat, SUJET, d, Origine::Reseau);
            assert!(titre.fr.starts_with(debut) && corps.fr.starts_with(debut));
            assert!(corps.fr.ends_with("Sessions de négociation"));
            assert!(corps.en.ends_with("Negotiation sessions"));
            assert!(corps.fr.contains("signalé par le réseau"));
        }
    }

    #[test]
    fn la_reunion_dit_son_jour_et_son_heure() {
        let (_, corps) = changement(
            Etat::NonAnnoncee,
            SUJET,
            Detail::Reunion {
                jour: date!(2026 - 11 - 10),
                heure: Some("14:00"),
                lieu: Some("Couloir B"),
            },
            Origine::Reseau,
        );
        assert!(
            corps.fr.contains("10 novembre à 14:00, Couloir B"),
            "{}",
            corps.fr
        );
        assert!(corps.en.contains("November 10 at 14:00"), "{}", corps.en);
    }

    #[test]
    fn une_cle_fait_remplacer_sans_cle_rien_ne_se_remplace() {
        let cible = || Cible {
            table: "meetings",
            id: Uuid::nil(),
            lien: "/x".into(),
        };
        let texte = publie(SUJET);
        let avec = notification(
            "t.a.b",
            vec![],
            texte.clone(),
            cible(),
            Some("k".into()),
            serde_json::json!({}),
        );
        let sans = notification("t.a.b", vec![], texte, cible(), None, serde_json::json!({}));
        assert!(avec.replace && !sans.replace);
    }
}
