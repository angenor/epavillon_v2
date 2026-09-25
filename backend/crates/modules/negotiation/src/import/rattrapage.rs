//! Le rattrapage (research R7, FR-019) : l'encart se retire quand la source dit
//! ce qu'il disait. Fonction pure, appliquée par l'import à l'état en base
//! **après** l'écriture de la lecture — absences comprises.

use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::reports::ReportReason;
use crate::import::denominations::normaliser;

/// Un signalement publié et encore affiché.
#[derive(Debug, Clone)]
pub struct Affiche {
    pub id: Uuid,
    pub reason: ReportReason,
    pub proposed_start: Option<OffsetDateTime>,
    pub proposed_venue: Option<String>,
}

/// La session telle qu'elle est en base après la lecture.
#[derive(Debug, Clone)]
pub struct Officielle {
    pub annulee: bool,
    pub debut: OffsetDateTime,
    pub salle: Option<String>,
}

fn a_la_minute(t: OffsetDateTime) -> i64 {
    t.unix_timestamp().div_euclid(60)
}

/// « Autre chose » ne se rattrape jamais : rien ne dit ce qu'il faudrait lire.
pub fn rattrape(s: &Affiche, o: &Officielle) -> bool {
    match s.reason {
        ReportReason::Cancelled => o.annulee,
        ReportReason::Time => s
            .proposed_start
            .is_some_and(|p| a_la_minute(p) == a_la_minute(o.debut)),
        ReportReason::Venue => match (&s.proposed_venue, &o.salle) {
            (Some(p), Some(salle)) => {
                let p = normaliser(p);
                !p.is_empty() && p == normaliser(salle)
            }
            _ => false,
        },
        ReportReason::Other | ReportReason::Unannounced => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn affiche(reason: ReportReason) -> Affiche {
        Affiche {
            id: Uuid::nil(),
            reason,
            proposed_start: Some(datetime!(2026-11-10 09:30:40 UTC)),
            proposed_venue: Some("Salle  Pacifique".into()),
        }
    }

    fn officielle(annulee: bool, debut: OffsetDateTime, salle: &str) -> Officielle {
        Officielle {
            annulee,
            debut,
            salle: Some(salle.into()),
        }
    }

    #[test]
    fn chaque_motif_se_rattrape_par_sa_valeur() {
        let avant = officielle(false, datetime!(2026-11-10 08:00 UTC), "Meeting Room 01");
        let apres = officielle(true, datetime!(2026-11-10 09:30 UTC), "salle pacifique");
        for motif in [
            ReportReason::Cancelled,
            ReportReason::Time,
            ReportReason::Venue,
        ] {
            assert!(!rattrape(&affiche(motif), &avant), "{motif:?}");
            assert!(rattrape(&affiche(motif), &apres), "{motif:?}");
        }
    }

    #[test]
    fn autre_chose_ne_se_rattrape_jamais() {
        let o = officielle(true, datetime!(2026-11-10 09:30 UTC), "Salle Pacifique");
        assert!(!rattrape(&affiche(ReportReason::Other), &o));
    }

    #[test]
    fn une_minute_decart_ne_rattrape_pas() {
        let o = officielle(false, datetime!(2026-11-10 09:31 UTC), "x");
        assert!(!rattrape(&affiche(ReportReason::Time), &o));
    }
}
