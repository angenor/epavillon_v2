//! Lecture du schéma `programme` — **en lecture seule**.
//!
//! « Que se passe-t-il aujourd'hui, et sur quoi puis-je publier ? »
//!
//! # LA LECTURE PORTE SUR LA TABLE, PAS SUR `v_public_schedule`
//!
//! La vue écarte les activités **non publiées**, et le poste de direct est un
//! écran de back-office : **une activité non publiée peut parfaitement tomber en
//! panne**. Prendre la vue reviendrait à rendre invisible, sur l'écran du
//! direct, exactement ce qui n'a pas encore été annoncé.
//!
//! L'état temporel est donc **recopié** de la vue — et cette duplication est
//! tenue par un test qui compare les deux sur les cinq branches, pour une
//! activité publiée. Ajouter une fonction au modèle pour la partager coûterait
//! plus qu'elle ne rapporterait : deux appelants, dont une vue, et un `CASE` de
//! cinq branches dans une fonction `STABLE` appelée par ligne.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// Une activité du poste de direct, avant que le compteur de messages actifs y
/// soit posé.
pub struct ActiviteDuPoste {
    pub session_id: Uuid,
    pub title: Value,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub room_name: Option<Value>,
    pub is_streamed: bool,
    pub status: String,
    pub temporal_state: String,
}

/// Les activités dont le début tombe **ce jour-là dans le fuseau de
/// l'édition**, par début croissant.
pub async fn du_jour(
    conn: &mut PgConnection,
    event_id: Uuid,
    jour: Date,
) -> Result<Vec<ActiviteDuPoste>> {
    let lignes = sqlx::query_as!(
        ActiviteDuPoste,
        r#"SELECT s.id AS "session_id!", s.title AS "title!: Value", s.starts_at, s.ends_at,
                  r.name AS "room_name?: Value",
                  s.is_streamed, s.status::text AS "status!",
                  CASE
                      WHEN s.status = 'cancelled'                   THEN 'cancelled'
                      WHEN s.status = 'postponed'                   THEN 'postponed'
                      WHEN now() < s.starts_at                      THEN 'upcoming'
                      WHEN now() BETWEEN s.starts_at AND s.ends_at  THEN 'ongoing'
                      ELSE 'past'
                  END AS "temporal_state!"
             FROM programme.sessions s
             JOIN event.events e ON e.id = s.event_id
             LEFT JOIN event.rooms r ON r.id = s.room_id
            WHERE s.event_id = $1
              AND (s.starts_at AT TIME ZONE e.timezone)::date = $2
            ORDER BY s.starts_at"#,
        event_id,
        jour
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes)
}

/// Les **quatre** prochaines activités, par début croissant — le repli quand le
/// jour est vide. Aucune notion de repli n'existe en base : c'est une règle
/// d'écran, et le nombre est le sien.
pub async fn les_prochaines(
    conn: &mut PgConnection,
    event_id: Uuid,
    combien: i64,
) -> Result<Vec<ActiviteDuPoste>> {
    let lignes = sqlx::query_as!(
        ActiviteDuPoste,
        r#"SELECT s.id AS "session_id!", s.title AS "title!: Value", s.starts_at, s.ends_at,
                  r.name AS "room_name?: Value",
                  s.is_streamed, s.status::text AS "status!",
                  CASE
                      WHEN s.status = 'cancelled'                   THEN 'cancelled'
                      WHEN s.status = 'postponed'                   THEN 'postponed'
                      WHEN now() < s.starts_at                      THEN 'upcoming'
                      WHEN now() BETWEEN s.starts_at AND s.ends_at  THEN 'ongoing'
                      ELSE 'past'
                  END AS "temporal_state!"
             FROM programme.sessions s
             LEFT JOIN event.rooms r ON r.id = s.room_id
            WHERE s.event_id = $1
              AND s.starts_at >= now()
            ORDER BY s.starts_at
            LIMIT $2"#,
        event_id,
        combien
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes)
}

/// Une cible d'activité, pour le choix de portée.
///
/// **Le libellé est résolu par `platform.t()`**, la fonction du modèle, et non
/// côté Rust : c'est elle qui résout déjà `target_label` dans
/// `live.event_incidents()`, et deux résolutions différentes pour la même
/// activité — l'une dans la liste déroulante, l'autre sur la ligne publiée —
/// finiraient par diverger sur le repli de langue.
pub struct ActiviteCible {
    pub id: Uuid,
    pub label: String,
    pub starts_at: OffsetDateTime,
}

/// Les activités de l'édition, par début croissant. **`starts_at` est rendu
/// comme instant**, à part de toute précision textuelle : l'interface seule sait
/// l'afficher dans le fuseau de l'édition.
pub async fn cibles(
    conn: &mut PgConnection,
    event_id: Uuid,
    locale: &str,
) -> Result<Vec<ActiviteCible>> {
    let lignes = sqlx::query!(
        r#"SELECT s.id, platform.t(s.title, $2) AS "label!", s.starts_at
             FROM programme.sessions s
            WHERE s.event_id = $1
            ORDER BY s.starts_at"#,
        event_id,
        locale
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ActiviteCible {
            id: l.id,
            label: l.label,
            starts_at: l.starts_at,
        })
        .collect())
}

/// Le gabarit du raccourci « Signaler un débordement ».
///
/// **Le titre est ici RÉSOLU**, à la différence du reste de l'écran : c'est une
/// valeur de pré-remplissage de champ, que le site pose telle quelle dans le
/// formulaire, pas une donnée à afficher.
pub struct GabaritActivite {
    pub session_id: Uuid,
    pub title: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub event_id: Uuid,
}

pub async fn gabarit(
    conn: &mut PgConnection,
    session_id: Uuid,
    locale: &str,
) -> Result<Option<GabaritActivite>> {
    let ligne = sqlx::query!(
        r#"SELECT s.id, platform.t(s.title, $2) AS "title!",
                  s.starts_at, s.ends_at, s.event_id
             FROM programme.sessions s
            WHERE s.id = $1"#,
        session_id,
        locale
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| GabaritActivite {
        session_id: l.id,
        title: l.title,
        starts_at: l.starts_at,
        ends_at: l.ends_at,
        event_id: l.event_id,
    }))
}
