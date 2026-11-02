//! Les événements de séance : la réunion Zoom suit sa séance (annulée chez Zoom
//! cinq minutes plus tard, délai pour défaire l'annulation en régie ; ou
//! déplacée). Une séance reportée (`postponed`) garde sa réunion :
//! l'accès est refusé, rien n'est défait chez Zoom.
//!
//! Une séance diffusée en entrée libre n'aura aucune inscription pour faire
//! naître sa réunion : elle naît quand la séance est créée, programmée
//! (`scheduled`, ce que pose la publication du programme), en cours ou
//! déplacée, pourvu qu'elle soit publiée.
//!
//! Une activité de JOURNÉE (02/11) n'a pas de réunion propre : son changement
//! recale seulement la réunion de la journée sur le nouveau déroulé.
//!
//! Le passage en direct par la régie (`live`, 02/11) fait naître la réunion qui
//! manque, celle de l'activité ou de sa journée, même sans aucune inscription :
//! sans elle, l'animateur n'a pas de lien et le public pas de salle.

use async_trait::async_trait;
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;

use crate::repo::cross::programme;
use crate::repo::meetings;
use crate::service::inscrits;

pub const ANNULEE: &str = "programme.session.cancelled";
pub const DEPLACEE: &str = "programme.session.rescheduled";
pub const CREEE: &str = "programme.session.created";
pub const PROGRAMMEE: &str = "programme.session.scheduled";
pub const EN_COURS: &str = "programme.session.live";

pub struct SessionsConsumer;

#[async_trait]
impl EventConsumer for SessionsConsumer {
    /// Inscrit dans `platform.inbox_events` : ne se renomme pas.
    fn name(&self) -> &'static str {
        "live.sessions"
    }

    fn handles(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            ANNULEE | DEPLACEE | CREEE | PROGRAMMEE | EN_COURS
        )
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        let Some(seance) = programme::seance(conn, event.aggregate_id).await? else {
            return Ok(());
        };
        if seance.status == "live" {
            inscrits::assurer_reunion_du_direct(conn, &seance).await?;
        }
        if let Some(track_id) = programme::journee_de_seance(conn, seance.id).await? {
            return inscrits::recaler_journee(conn, track_id).await;
        }
        if seance.status == "cancelled" {
            return crate::jobs::annulation_differee::poser(conn, seance.id).await;
        }
        if event.event_type == DEPLACEE {
            let minutes = inscrits::duree_minutes(&seance);
            meetings::recaler_reunion(conn, seance.id, seance.starts_at, minutes).await?;
        }
        let entree_libre = !seance.registration_required
            && seance.is_streamed
            && seance.is_published
            && matches!(seance.status.as_str(), "scheduled" | "live");
        if entree_libre && meetings::reunion_active(conn, seance.id).await?.is_none() {
            inscrits::assurer_reunion(conn, &seance, None).await?;
        }
        Ok(())
    }
}
