//! La part **active** des messages d'une édition — la lecture publique.
//!
//! `live.active_incidents_for_event()` est écrite **au-dessus** de
//! `live.event_incidents()` plutôt qu'à côté : deux balayages de portée qui
//! divergent, et le même message s'affiche dans un écran sans apparaître dans
//! l'autre. Le code ne rejoue donc ni le balayage ni le filtre d'état.

use kernel::error::Result;
use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Une ligne du bandeau public. **`target_label` est déjà résolu par le
/// modèle** — « Atelier de négociation », « Journée finance », le nom légal
/// d'une organisation : le bandeau nomme son sujet, et un message de portée
/// `session` reste lisible sur une page qui parle de trente activités.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ActiveIncident {
    pub incident_id: Uuid,
    pub scope: String,
    pub severity: String,
    pub kind_code: String,
    pub title: Option<Value>,
    pub message: Value,
    pub action_url: Option<String>,
    pub is_dismissible: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub display_from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub display_until: Option<OffsetDateTime>,
    pub target_id: Option<Uuid>,
    pub target_label: Option<String>,
}

/// **Le plus grave en tête**, dans l'ordre où la fonction les rend.
///
/// Une édition inconnue rend une **liste vide, jamais 404** : cette route ne dit
/// pas si une édition existe, et un bandeau absent se lit exactement comme une
/// édition sans incident — ce qui est le cas normal.
pub async fn pour_ledition(pool: &PgPool, event_id: Uuid) -> Result<Vec<ActiveIncident>> {
    let lignes = sqlx::query!(
        r#"SELECT incident_id     AS "incident_id!",
                  scope::text     AS "scope!",
                  severity::text  AS "severity!",
                  kind_code       AS "kind_code!",
                  title           AS "title?: Value",
                  message         AS "message!: Value",
                  action_url::text AS "action_url?",
                  is_dismissible  AS "is_dismissible!",
                  display_from    AS "display_from!",
                  display_until,
                  target_id,
                  target_label
             FROM live.active_incidents_for_event($1, now())"#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ActiveIncident {
            incident_id: l.incident_id,
            scope: l.scope,
            severity: l.severity,
            kind_code: l.kind_code,
            title: l.title,
            message: l.message,
            action_url: l.action_url,
            is_dismissible: l.is_dismissible,
            display_from: l.display_from,
            display_until: l.display_until,
            target_id: l.target_id,
            target_label: l.target_label,
        })
        .collect())
}
