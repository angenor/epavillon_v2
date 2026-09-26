//! Le texte d'un avis, FR et EN. Le titre est la phrase entière, qui commence
//! par l'état (« Déplacée — ») ; le corps est l'agenda d'origine, « Sessions de
//! négociation » (FR-028) : le centre les lit comme « phrase » puis « origine ·
//! heure » (maquette 02 · 10), sans redire l'état. Jamais le nom de l'autrice
//! (SC-008). Les heures arrivent déjà dans le fuseau de la COP.

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

/// « heure d'Antalya », « heure de Belém » : même règle que `zoneElides()` du
/// site — voyelle, accentuée ou non ; le « h » ne compte pas, le nom seul ne dit
/// pas s'il est muet.
pub(crate) fn heure_de(ville: &str) -> String {
    let elide = ville
        .trim_start()
        .chars()
        .next()
        .and_then(|c| c.to_lowercase().next())
        .is_some_and(|c| "aeiouàâäáãéèêëíìîïóòôöõúùûü".contains(c));
    if elide {
        format!("heure d'{ville}")
    } else {
        format!("heure de {ville}")
    }
}

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
                format!("nouvelle heure {heure} ({})", heure_de(v)),
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
    phrase(
        format!("{} — {}, {precision_fr}{origine_fr}.", etat.fr(), sujet.fr),
        format!("{} — {}, {precision_en}{origine_en}.", etat.en(), sujet.en),
    )
}

fn phrase(fr: String, en: String) -> Texte {
    (
        texte(fr, en),
        texte(SIGNATURE_FR.to_owned(), SIGNATURE_EN.to_owned()),
    )
}

/// Pour l'autrice : son signalement est affiché.
pub fn publie(sujet: Sujet<'_>) -> Texte {
    phrase(
        format!(
            "Validé — {} : votre signalement est affiché, sans votre nom.",
            sujet.fr
        ),
        format!(
            "Validated — {}: your report is displayed, without your name.",
            sujet.en
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
    phrase(
        format!("Non retenu — {} : {fr}.", sujet.fr),
        format!("Not retained — {}: {en}.", sujet.en),
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
            assert!(titre.fr.starts_with(debut), "{}", titre.fr);
            assert_eq!(titre.fr.matches(debut).count(), 1, "{}", titre.fr);
            assert!(titre.fr.contains("signalé par le réseau"));
            assert_eq!(corps.fr, "Sessions de négociation");
            assert_eq!(corps.en, "Negotiation sessions");
        }
    }

    #[test]
    fn le_fuseau_elide_devant_une_voyelle() {
        assert_eq!(heure_de("Antalya"), "heure d'Antalya");
        assert_eq!(heure_de("Érevan"), "heure d'Érevan");
        assert_eq!(heure_de("Belém"), "heure de Belém");
        assert_eq!(heure_de("Hambourg"), "heure de Hambourg");
        let (titre, _) = changement(
            Etat::Deplacee,
            SUJET,
            Detail::Heure {
                heure: "11:30",
                ville: Some("Antalya"),
            },
            Origine::Source,
        );
        assert!(titre.fr.contains("(heure d'Antalya)"), "{}", titre.fr);
    }

    #[test]
    fn la_reunion_dit_son_jour_et_son_heure() {
        let (corps, _) = changement(
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
