//! Lecture du schéma `live` — **une FONCTION SQL, jamais le crate `live`**.
//!
//! C'est ce qui fait que les deux crates de ce jalon ne partagent aucune ligne
//! de Rust et qu'aucune arête ne les relie dans `cargo tree`. Le seul rapport
//! entre eux — le tableau de bord affiche les messages actifs de l'édition —
//! passe par `live.active_incidents_for_event()`, écrite au-dessus de
//! `live.event_incidents()` pour que les deux écrans ne balaient jamais les
//! portées différemment.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::dashboard::EventIncident;

/// Les messages actifs de l'édition, **le plus grave en tête**, dans l'ordre où
/// la fonction les rend. `target_label` est **déjà résolu par le modèle**.
pub async fn incidents_actifs(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Vec<EventIncident>> {
    let lignes = sqlx::query_as!(
        EventIncident,
        r#"SELECT incident_id    AS "incident_id!",
                  scope::text    AS "scope!",
                  severity::text AS "severity!",
                  kind_code      AS "kind_code!",
                  title          AS "title?: Value",
                  message        AS "message!: Value",
                  target_label,
                  display_from   AS "display_from!",
                  display_until
             FROM live.active_incidents_for_event($1, now())"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes)
}
