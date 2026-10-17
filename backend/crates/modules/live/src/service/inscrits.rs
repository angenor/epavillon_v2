//! D'une inscription plateforme à un inscrit chez Zoom, sans jamais appeler
//! Zoom : on écrit la ligne, la file fait le reste.
//!
//! Partagé par le consommateur d'inscriptions et par la reconstruction du
//! back-office, pour qu'un même geste ait un seul chemin.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::repo::cross::programme::{self, Inscription, Seance};
use crate::repo::meetings::{self, NouveauParticipant, NouvelleReunion};

/// Les statuts qui donnent une place chez Zoom. La liste d'attente en est :
/// le jour même, Zoom laisse entrer les trois cents premiers arrivés.
pub fn a_une_place(statut: &str) -> bool {
    matches!(statut, "registered" | "waitlisted" | "attended")
}

/// Aligne la ligne `live.meeting_participants` sur l'état courant de
/// l'inscription, lu en base plutôt que dans l'événement : des événements
/// traités en retard ne remettent pas un état dépassé.
pub async fn synchroniser(conn: &mut PgConnection, registration_id: Uuid) -> Result<()> {
    let Some(inscription) = programme::inscription(conn, registration_id).await? else {
        return Ok(());
    };

    if a_une_place(&inscription.status) {
        let reunion = match meetings::reunion_active(conn, inscription.seance.id).await? {
            Some(r) => r.id,
            // Une séance non diffusée n'a de réunion que si le back-office l'a créée.
            None if inscription.seance.is_streamed => {
                assurer_reunion(conn, &inscription.seance, None).await?
            }
            None => return Ok(()),
        };
        assurer_participant(conn, reunion, &inscription).await
    } else if inscription.status == "cancelled" {
        if let Some(p) = meetings::participant_de_inscription(conn, registration_id).await? {
            meetings::annuler_participant(conn, p.id).await?;
        }
        Ok(())
    } else {
        Ok(())
    }
}

pub async fn assurer_reunion(
    conn: &mut PgConnection,
    seance: &Seance,
    external_id: Option<&str>,
) -> Result<Uuid> {
    let topic = intitule(&seance.title);
    let minutes = ((seance.ends_at - seance.starts_at).whole_seconds() + 59) / 60;
    meetings::creer_reunion(
        conn,
        &NouvelleReunion {
            session_id: seance.id,
            event_id: seance.event_id,
            topic: &topic,
            scheduled_start_at: seance.starts_at,
            duration_minutes: minutes.clamp(1, 1440) as i32,
            timezone: &seance.timezone,
            external_id,
        },
    )
    .await
}

async fn assurer_participant(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    i: &Inscription,
) -> Result<()> {
    let nouveau = NouveauParticipant {
        meeting_id,
        registration_id: i.registration_id,
        person_id: i.person_id,
        display_name: &i.display_name,
        email: &i.email,
        first_name: &i.first_name,
        last_name: &i.last_name,
    };

    if let Some(p) = meetings::participant_de_inscription(conn, i.registration_id).await? {
        if p.meeting_id == meeting_id && p.cancelled_at.is_none() {
            return Ok(());
        }
        return meetings::reprendre_participant(conn, p.id, &nouveau).await;
    }

    // L'adresse est unique par réunion : une personne qui s'était désinscrite
    // retrouve sa ligne, et chez Zoom le même inscrit.
    if let Some(p) = meetings::participant_par_email(conn, meeting_id, &i.email).await? {
        if p.cancelled_at.is_none() && p.registration_id == Some(i.registration_id) {
            return Ok(());
        }
        return meetings::reprendre_participant(conn, p.id, &nouveau).await;
    }

    meetings::inserer_participant(conn, &nouveau).await?;
    Ok(())
}

/// L'intitulé de la réunion chez Zoom : le titre français, à défaut le premier
/// présent. Zoom le borne à deux cents caractères.
pub fn intitule(titre: &Value) -> String {
    let texte = titre
        .get("fr")
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .or_else(|| {
            titre.as_object().and_then(|o| {
                o.values()
                    .filter_map(Value::as_str)
                    .find(|t| !t.trim().is_empty())
            })
        })
        .unwrap_or("Activité du Pavillon de la Francophonie");
    texte.trim().chars().take(200).collect()
}
