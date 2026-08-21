//! `event.programme.published` → les séances deviennent publiques.
//!
//! Les cinq règles du contrat, et où chacune est tenue :
//!
//! 1. **Garde de rejeu** — portée par le noyau, pas par ce fichier ;
//! 2. **Le prédicat porté, et pas un autre** — `selection` est appliquée telle
//!    quelle ;
//! 3. **Il n'écrit pas `event.events.programme_published_at`** — déjà posée par
//!    l'émetteur, et hors de son schéma ;
//! 4. **Une seconde livraison ne publie rien de plus** — conséquence de la
//!    règle 1, mesurée et non supposée ;
//! 5. **La date posée est celle de l'annonce**, jamais l'instant du traitement.

use async_trait::async_trait;
use kernel::error::{ApiError, Result};
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;

use crate::service::publication::{self, Designation};

pub struct PublicationConsumer;

#[async_trait]
impl EventConsumer for PublicationConsumer {
    /// **Ce nom est inscrit dans `platform.inbox_events`** : il identifie le
    /// consommateur pour la garde d'idempotence, et le renommer ferait rejouer
    /// toutes les annonces déjà traitées.
    fn name(&self) -> &'static str {
        "programme.publication"
    }

    fn handles(&self, event_type: &str) -> bool {
        event_type == contracts::event::PROGRAMME_PUBLISHED
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        let annonce: contracts::event::ProgrammePublished =
            serde_json::from_value(event.payload.clone()).map_err(ApiError::internal)?;

        let publiees = publication::publier_les_seances(
            conn,
            Designation {
                event_id: annonce.event_id,
                published_at: annonce.published_at,
                statuses: &annonce.selection.statuses,
                only_unpublished: annonce.selection.only_unpublished,
            },
        )
        .await?;

        // L'effet peut dépasser l'annonce, jamais l'inverse : une séance née
        // entre les deux instants porte l'état visé et n'est pas publiée
        // (research.md § R14). L'écart est tracé plutôt que supposé nul.
        if publiees as i64 != annonce.published_count {
            tracing::info!(
                edition = %annonce.event_id,
                annonce = annonce.published_count,
                effet = publiees,
                "publication : l'effet diffère du nombre annoncé"
            );
        }

        Ok(())
    }
}
