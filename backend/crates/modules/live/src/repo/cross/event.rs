//! Lecture du schéma `event` — **en lecture seule**.
//!
//! Trois questions du module, et rien d'autre : dans quel fuseau se lit la
//! fenêtre d'affichage d'un message ? Quelles journées puis-je viser ? Dans
//! quelle salle se tient l'activité que le poste de direct surveille ?

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::Date;
use uuid::Uuid;

/// Ce que l'en-tête de l'écran demande à l'édition.
///
/// **`zone_label` est la VILLE**, pas l'identifiant IANA : l'écran affiche
/// « heure de Belém », et non « heure de America/Belem ».
pub struct EnteteEdition {
    pub event_id: Uuid,
    pub title: Value,
    /// Le même titre, **résolu par `platform.t()`** : le choix de portée
    /// affiche un libellé, pas un document multilingue.
    pub title_label: String,
    pub acronym: Option<String>,
    pub timezone: String,
    pub zone_label: Option<String>,
    /// Le jour de l'édition à l'instant de la transaction — calculé **en base**,
    /// dans le fuseau de l'édition et jamais dans celui du serveur.
    pub aujourdhui: Date,
}

pub async fn entete(
    conn: &mut PgConnection,
    event_id: Uuid,
    locale: &str,
) -> Result<Option<EnteteEdition>> {
    let ligne = sqlx::query!(
        r#"SELECT e.id, e.title AS "title!: Value",
                  platform.t(e.title, $2) AS "title_label!",
                  e.acronym,
                  e.timezone::text AS "timezone!",
                  e.city,
                  (now() AT TIME ZONE e.timezone)::date AS "aujourdhui!"
             FROM event.events e
            WHERE e.id = $1"#,
        event_id,
        locale
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| EnteteEdition {
        event_id: l.id,
        title: l.title,
        title_label: l.title_label,
        acronym: l.acronym,
        timezone: l.timezone,
        zone_label: l.city,
        aujourdhui: l.aujourdhui,
    }))
}

/// Les journées de l'édition, par date.
///
/// **Une journée sans titre est désignée par sa date**, au format `JJ/MM/AAAA` —
/// exactement comme le fait `live.event_incidents()` pour résoudre sa cible.
/// Deux libellés différents pour la même journée selon l'écran seraient un
/// défaut que personne ne saurait rapporter.
pub struct JourneeCible {
    pub id: Uuid,
    pub label: String,
    pub day_date: Date,
}

pub async fn journees(
    conn: &mut PgConnection,
    event_id: Uuid,
    locale: &str,
) -> Result<Vec<JourneeCible>> {
    let lignes = sqlx::query!(
        r#"SELECT d.id,
                  COALESCE(platform.t(d.title, $2), to_char(d.day_date, 'DD/MM/YYYY')) AS "label!",
                  d.day_date
             FROM event.event_days d
            WHERE d.event_id = $1
            ORDER BY d.day_date"#,
        event_id,
        locale
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| JourneeCible {
            id: l.id,
            label: l.label,
            day_date: l.day_date,
        })
        .collect())
}
