//! Le lecteur d'un jeu archivé, embarqué dans le binaire (research R3) : le
//! mécanisme s'éprouve entier sur un vrai programme, sans la source.

use async_trait::async_trait;
use time::Date;

use super::ccnucc;
use super::source::{EchecLecture, SessionLue, SourceOfficielle};

/// Les jeux qu'offre le back-office.
pub const JEUX: [&str; 4] = [
    "cop30/lecture-1",
    "cop30/lecture-2",
    "cop30/illisible",
    "cop30/injoignable",
];

enum Jeu {
    Texte(&'static str),
    Injoignable,
}

fn jeu(nom: &str) -> Option<Jeu> {
    Some(match nom {
        "cop30/lecture-1" => Jeu::Texte(include_str!("archives/cop30/lecture-1.json")),
        "cop30/lecture-2" => Jeu::Texte(include_str!("archives/cop30/lecture-2.json")),
        "cop30/illisible" => Jeu::Texte(include_str!("archives/cop30/illisible.json")),
        "cop30/injoignable" => Jeu::Injoignable,
        _ => return None,
    })
}

pub struct LecteurArchive {
    pub nom: String,
    /// Pose le premier jour de l'archive sur ce jour.
    pub premier_jour: Option<Date>,
    pub correction_minutes: i64,
}

#[async_trait]
impl SourceOfficielle for LecteurArchive {
    async fn lire(&self) -> Result<Vec<SessionLue>, EchecLecture> {
        let texte = match jeu(&self.nom) {
            Some(Jeu::Texte(t)) => t,
            Some(Jeu::Injoignable) => {
                return Err(EchecLecture::Injoignable(
                    "panne simulée par le jeu archivé « injoignable »".into(),
                ))
            }
            None => {
                return Err(EchecLecture::Injoignable(format!(
                    "jeu archivé « {} » inconnu",
                    self.nom
                )))
            }
        };
        let mut sessions = ccnucc::analyser(texte, self.correction_minutes)?;
        if let Some(jour) = self.premier_jour {
            translater(&mut sessions, jour);
        }
        Ok(sessions)
    }
}

/// Décale tous les jours d'autant, heure murale gardée : le fuseau de l'édition
/// situe ensuite ces heures, heure d'été comprise.
pub fn translater(sessions: &mut [SessionLue], premier_jour: Date) {
    let Some(depart) = sessions.iter().map(|s| s.debut.date()).min() else {
        return;
    };
    let ecart = premier_jour - depart;
    for s in sessions {
        s.debut += ecart;
        s.fin = s.fin.map(|f| f + ecart);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{date, datetime};

    fn lecteur(nom: &str, premier_jour: Option<Date>) -> LecteurArchive {
        LecteurArchive {
            nom: nom.into(),
            premier_jour,
            correction_minutes: 60,
        }
    }

    fn cle<'a>(sessions: &'a [SessionLue], cle: &str) -> &'a SessionLue {
        sessions
            .iter()
            .find(|s| s.cle == cle)
            .expect("session du jeu")
    }

    #[tokio::test]
    async fn le_jeu_se_lit_tel_quel() {
        let sessions = lecteur("cop30/lecture-1", None)
            .lire()
            .await
            .expect("lecture");
        assert_eq!(sessions.len(), 47);
        assert_eq!(cle(&sessions, "654006").debut, datetime!(2025-11-18 11:00));
    }

    #[tokio::test]
    async fn le_premier_jour_translate_les_jours_et_garde_l_heure() {
        let sessions = lecteur("cop30/lecture-1", Some(date!(2026 - 11 - 09)))
            .lire()
            .await
            .expect("lecture");
        assert_eq!(cle(&sessions, "651031").debut, datetime!(2026-11-09 09:00));
        let s = cle(&sessions, "654006");
        assert_eq!(s.debut, datetime!(2026-11-10 11:00));
        assert_eq!(s.fin, Some(datetime!(2026-11-10 12:00)));
    }

    #[tokio::test]
    async fn lecture_2_porte_ses_quatre_ecarts() {
        let un = lecteur("cop30/lecture-1", None)
            .lire()
            .await
            .expect("lecture");
        let deux = lecteur("cop30/lecture-2", None)
            .lire()
            .await
            .expect("lecture");
        assert_eq!(
            cle(&deux, "654214").debut,
            cle(&un, "654214").debut - time::Duration::hours(1)
        );
        assert_eq!(
            cle(&deux, "654010").salle.as_deref(),
            Some("Meeting Room 03")
        );
        assert!(deux.iter().all(|s| s.cle != "654364"));
        assert!(un.iter().all(|s| s.cle != "699001"));
        assert!(deux.iter().any(|s| s.cle == "699001"));
    }

    #[tokio::test]
    async fn les_jeux_d_echec() {
        assert!(matches!(
            lecteur("cop30/injoignable", None).lire().await,
            Err(EchecLecture::Injoignable(_))
        ));
        assert!(matches!(
            lecteur("cop30/illisible", None).lire().await,
            Err(EchecLecture::Illisible(_))
        ));
        assert!(matches!(
            lecteur("cop99/inconnu", None).lire().await,
            Err(EchecLecture::Injoignable(_))
        ));
    }
}
