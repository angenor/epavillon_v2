//! `live.meeting.started` / `live.meeting.ended` → la séance passe en direct,
//! puis en terminée. Décision et règles : en-tête de `080_live.sql`.
//!
//! Démarrage : `scheduled`, ou `completed` pour une réunion relancée, vers
//! `live`. Fin : `live` vers `completed`. Une séance annulée, reportée ou non
//! programmée ne bouge jamais ; une répétition n'arrive jamais ici, la base ne
//! l'annonce qu'à début − 15 min. Le déclencheur des séances émet ensuite
//! `programme.session.live` ou `.completed`, comme pour tout changement d'état,
//! et `tg_sessions_public_signal` le signal `edition:<id>:public:live` des
//! pages publiques ouvertes.

use async_trait::async_trait;
use contracts::live::{MEETING_ENDED, MEETING_STARTED};
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

pub struct DirectConsumer;

#[async_trait]
impl EventConsumer for DirectConsumer {
    /// Inscrit dans `platform.inbox_events` : ne se renomme pas.
    fn name(&self) -> &'static str {
        "programme.direct"
    }

    fn handles(&self, event_type: &str) -> bool {
        matches!(event_type, MEETING_STARTED | MEETING_ENDED)
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        let Some(session_id) = event.payload["session_id"]
            .as_str()
            .and_then(|s| s.parse::<Uuid>().ok())
        else {
            return Ok(());
        };
        let (depuis, vers): (&[&str], &str) = if event.event_type == MEETING_STARTED {
            (&["scheduled", "completed"], "live")
        } else {
            (&["live"], "completed")
        };
        sqlx::query_scalar!(
            "UPDATE programme.sessions
                SET status = $3::text::programme.session_status
              WHERE id = $1 AND status::text = ANY ($2)
          RETURNING event_id",
            session_id,
            depuis as &[&str],
            vers
        )
        .fetch_optional(conn)
        .await?;
        Ok(())
    }
}
