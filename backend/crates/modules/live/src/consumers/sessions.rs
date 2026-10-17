//! Les événements de séance : la réunion Zoom suit sa séance (annulée chez
//! Zoom, ou déplacée). Une séance reportée (`postponed`) garde sa réunion :
//! l'accès est refusé, rien n'est défait chez Zoom.
//!
//! Une séance diffusée en entrée libre n'aura aucune inscription pour faire
//! naître sa réunion : elle naît quand la séance est créée, programmée
//! (`scheduled`, ce que pose la publication du programme), en cours ou
//! déplacée, pourvu qu'elle soit publiée.

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
        if seance.status == "cancelled" {
            return meetings::annuler_reunion(conn, seance.id).await;
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
