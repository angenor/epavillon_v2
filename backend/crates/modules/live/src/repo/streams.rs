//! Les flux en cours d'une séance — `live.current_streams`, **telle quelle**.
//!
//! La vue porte déjà la règle d'affichage (`kind = 'live'`, `status = 'live'`)
//! et l'URL d'intégration de chaque diffuseur : le code ne la recompose pas.

use kernel::error::{ApiError, Result};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::repo::{cross, lecture};

/// `PublicSessionStream` — un lecteur possible, dans une langue.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PublicSessionStream {
    pub id: Uuid,
    pub locale: Option<String>,
    pub provider: String,
    pub embed_url: Option<String>,
    pub watch_url: Option<String>,
    pub is_primary: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub started_at: Option<OffsetDateTime>,
}

/// Flux principal d'abord, puis par langue. **404 pour une séance inconnue ou
/// non publiée — indiscernables**, comme la fiche publique de la séance.
pub async fn en_cours(pool: &PgPool, session_id: Uuid) -> Result<Vec<PublicSessionStream>> {
    let mut tx = lecture(pool).await?;
    if !cross::programme::publiee(&mut tx, session_id).await? {
        return Err(ApiError::not_found());
    }

    let lignes = sqlx::query_as!(
        PublicSessionStream,
        r#"SELECT cs.id            AS "id!",
                  cs.locale,
                  cs.provider::text AS "provider!",
                  cs.embed_url::text AS "embed_url?",
                  cs.watch_url::text AS "watch_url?",
                  cs.is_primary    AS "is_primary!",
                  cs.started_at
             FROM live.current_streams cs
            WHERE cs.session_id = $1
            ORDER BY cs.is_primary DESC, cs.locale NULLS LAST, cs.id"#,
        session_id
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(lignes)
}
