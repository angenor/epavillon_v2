//! **Les variables d'un courriel écrit depuis un dossier** (01/11), pures.
//!
//! `{{activite}}`, `{{date}}`, `{{heures}}`, `{{salle}}` : lues sur le dossier
//! et sa séance retenue, en français (un courriel du back-office part dans une
//! langue). L'heure se lit dans le fuseau de la séance et le dit : « 10:00 à
//! 11:30, heure d'Antalya ».
//!
//! Une variable sans valeur n'est jamais remplacée par du vide : l'envoi est
//! refusé en la nommant. Un « Date : , » parti chez un partenaire ne se
//! rattrape pas.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use time::{Date, PrimitiveDateTime, Weekday};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::mailing::VARIABLES_DOSSIER;

/// La séance retenue d'un dossier, heures déjà dans son fuseau.
#[derive(Debug, Clone)]
pub struct SeanceRetenue {
    pub debut: PrimitiveDateTime,
    pub fin: PrimitiveDateTime,
    pub timezone: String,
    /// La ville hôte de l'édition, qui nomme le fuseau.
    pub ville: Option<String>,
    pub salle: Option<String>,
}

/// Le dossier d'un courriel (`MailingProposal`) : ce que la fenêtre de
/// rédaction en montre, et les valeurs de ses variables.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingProposal {
    pub id: Uuid,
    pub reference_code: String,
    pub title: String,
    /// Nom de variable -> valeur, pour celles qui en ont une.
    pub values: BTreeMap<String, String>,
    /// Pourquoi certaines variables n'ont pas de valeur (pas de séance, pas de
    /// salle) ; nul quand toutes en ont une.
    pub missing: Option<String>,
}

const JOURS: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];

const MOIS: [&str; 12] = [
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

fn jour_de(d: Date) -> &'static str {
    let rang = match d.weekday() {
        Weekday::Monday => 0,
        Weekday::Tuesday => 1,
        Weekday::Wednesday => 2,
        Weekday::Thursday => 3,
        Weekday::Friday => 4,
        Weekday::Saturday => 5,
        Weekday::Sunday => 6,
    };
    JOURS[rang]
}

/// « lundi 9 novembre 2026 », « dimanche 1er novembre 2026 ».
pub fn date_longue(d: Date) -> String {
    let quantieme = match d.day() {
        1 => "1er".to_owned(),
        n => n.to_string(),
    };
    format!(
        "{} {} {} {}",
        jour_de(d),
        quantieme,
        MOIS[usize::from(u8::from(d.month())) - 1],
        d.year()
    )
}

/// Le nom qui désigne le fuseau : la ville hôte, à défaut le dernier segment
/// de l'identifiant (`America/Sao_Paulo` donne « Sao Paulo »).
pub fn nom_du_fuseau(ville: Option<&str>, timezone: &str) -> String {
    match ville.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => v.to_owned(),
        None => timezone
            .rsplit('/')
            .next()
            .unwrap_or(timezone)
            .replace('_', " "),
    }
}

/// « heure d'Antalya », « heure de Belém » : élision devant une voyelle, comme
/// `zoneElides` du site. Le « h » n'est pas compté, muet ou aspiré.
pub fn heure_de(zone: &str) -> String {
    let initiale = zone
        .trim()
        .chars()
        .next()
        .map(|c| c.to_lowercase().collect::<String>())
        .unwrap_or_default();
    let voyelle = matches!(
        initiale.as_str(),
        "a" | "e" | "i" | "o" | "u" | "à" | "â" | "é" | "è" | "ê" | "î" | "ô" | "û"
    );
    if voyelle {
        format!("heure d'{zone}")
    } else {
        format!("heure de {zone}")
    }
}

fn heure(t: PrimitiveDateTime) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}

/// « 10:00 à 11:30, heure d'Antalya ».
pub fn heures(s: &SeanceRetenue) -> String {
    let zone = nom_du_fuseau(s.ville.as_deref(), &s.timezone);
    format!("{} à {}, {}", heure(s.debut), heure(s.fin), heure_de(&zone))
}

const SANS_SEANCE: &str = "Ce dossier n'a pas encore de séance programmée : {{date}}, {{heures}} et {{salle}} n'ont pas de valeur. Programmez l'activité, ou retirez ces variables du courriel.";
const SANS_SALLE: &str = "La séance retenue n'a pas de salle : {{salle}} n'a pas de valeur. Attribuez une salle, ou retirez cette variable du courriel.";

/// Les valeurs des variables du dossier, et la raison de celles qui manquent.
pub fn valeurs(
    titre: &str,
    seance: Option<&SeanceRetenue>,
) -> (BTreeMap<String, String>, Option<String>) {
    let mut v = BTreeMap::from([("activite".to_owned(), titre.trim().to_owned())]);
    let Some(s) = seance else {
        return (v, Some(SANS_SEANCE.to_owned()));
    };
    v.insert("date".to_owned(), date_longue(s.debut.date()));
    v.insert("heures".to_owned(), heures(s));
    match s.salle.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        Some(salle) => {
            v.insert("salle".to_owned(), salle.to_owned());
            (v, None)
        }
        None => (v, Some(SANS_SALLE.to_owned())),
    }
}

/// Refuse, en la nommant, une variable du dossier citée sans valeur : hors
/// dossier (un envoi groupé), ou faute de séance ou de salle. `None` : le
/// courriel n'est rattaché à aucun dossier.
pub fn exiger(citees: &BTreeSet<String>, dossier: Option<&MailingProposal>) -> Result<(), String> {
    let du_dossier: Vec<&String> = citees
        .iter()
        .filter(|v| VARIABLES_DOSSIER.contains(&v.as_str()))
        .collect();
    if du_dossier.is_empty() {
        return Ok(());
    }
    let Some(d) = dossier else {
        let noms: Vec<String> = du_dossier.iter().map(|v| format!("{{{{{v}}}}}")).collect();
        return Err(format!(
            "{} ne servent que dans un courriel écrit depuis la fiche d'un dossier : retirez-les, ou écrivez depuis le dossier.",
            noms.join(", ")
        ));
    };
    if du_dossier
        .iter()
        .any(|v| !d.values.contains_key(v.as_str()))
    {
        return Err(d
            .missing
            .clone()
            .unwrap_or_else(|| "Une variable du dossier n'a pas de valeur.".to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn seance(salle: Option<&str>) -> SeanceRetenue {
        SeanceRetenue {
            debut: datetime!(2026-11-09 10:00),
            fin: datetime!(2026-11-09 11:30),
            timezone: "Asia/Istanbul".into(),
            ville: Some("Antalya".into()),
            salle: salle.map(str::to_owned),
        }
    }

    #[test]
    fn la_date_et_les_heures_se_lisent_en_francais_avec_le_fuseau() {
        let (v, manque) = valeurs("Financer l'adaptation", Some(&seance(Some("Salle 2"))));
        assert_eq!(manque, None);
        assert_eq!(v["activite"], "Financer l'adaptation");
        assert_eq!(v["date"], "lundi 9 novembre 2026");
        assert_eq!(v["heures"], "10:00 à 11:30, heure d'Antalya");
        assert_eq!(v["salle"], "Salle 2");
    }

    #[test]
    fn le_premier_du_mois_et_la_ville_sans_elision() {
        assert_eq!(
            date_longue(time::macros::date!(2026 - 11 - 01)),
            "dimanche 1er novembre 2026"
        );
        assert_eq!(heure_de("Belém"), "heure de Belém");
        assert_eq!(nom_du_fuseau(None, "America/Sao_Paulo"), "Sao Paulo");
    }

    #[test]
    fn sans_seance_seule_lactivite_a_une_valeur() {
        let (v, manque) = valeurs("Titre", None);
        assert_eq!(v.len(), 1);
        assert!(manque.unwrap().contains("pas encore de séance"));
    }

    fn dossier(seance_: Option<&SeanceRetenue>) -> MailingProposal {
        let (values, missing) = valeurs("Titre", seance_);
        MailingProposal {
            id: Uuid::nil(),
            reference_code: "COP31-001".into(),
            title: "Titre".into(),
            values,
            missing,
        }
    }

    fn citees(noms: &[&str]) -> BTreeSet<String> {
        noms.iter().map(|n| (*n).to_owned()).collect()
    }

    #[test]
    fn hors_dossier_les_variables_du_dossier_sont_refusees_en_les_nommant() {
        let erreur = exiger(&citees(&["prenom", "activite", "date"]), None).unwrap_err();
        assert!(
            erreur.starts_with("{{activite}}, {{date}} ne servent"),
            "{erreur}"
        );
        assert!(exiger(&citees(&["prenom", "edition"]), None).is_ok());
    }

    #[test]
    fn sans_seance_date_est_refusee_et_activite_passe() {
        let d = dossier(None);
        assert!(exiger(&citees(&["activite"]), Some(&d)).is_ok());
        let erreur = exiger(&citees(&["activite", "heures"]), Some(&d)).unwrap_err();
        assert!(erreur.contains("pas encore de séance"), "{erreur}");
    }

    #[test]
    fn sans_salle_seule_salle_est_refusee() {
        let s = seance(None);
        let d = dossier(Some(&s));
        assert!(exiger(&citees(&["date", "heures"]), Some(&d)).is_ok());
        let erreur = exiger(&citees(&["salle"]), Some(&d)).unwrap_err();
        assert!(erreur.contains("pas de salle"), "{erreur}");
    }
}
