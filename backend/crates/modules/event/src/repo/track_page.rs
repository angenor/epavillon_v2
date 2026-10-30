//! Lectures et écritures de la page d'un fil, **dans le schéma `event`** :
//! colonnes de la page sur `programme_tracks`, parties, moments, partenaires.
//!
//! Les noms des partenaires et la composition en activités vivent hors du
//! schéma : ils se lisent dans `repo/cross_page.rs`.
//!
//! Les écritures rendent l'erreur **brute** : le service la traduit.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::ids::TrackId;
use crate::domain::track_page::{
    TrackMoment, TrackMomentPayload, TrackPagePayload, TrackPart, TrackPartPayload,
    TrackPartnerPayload,
};

/// Les colonnes de la page portées par le fil lui-même.
pub struct ColonnesDeLaPage {
    pub event_id: Uuid,
    pub venue_id: Option<Uuid>,
    pub tagline: Option<serde_json::Value>,
    /// Tableau JSON, un texte multilingue par public.
    pub target_audiences: serde_json::Value,
    pub support_note: Option<serde_json::Value>,
    pub feature_kicker: Option<serde_json::Value>,
    pub feature_title: Option<serde_json::Value>,
    pub feature_body: Option<serde_json::Value>,
    pub feature_link_label: Option<serde_json::Value>,
    pub feature_link_url: Option<String>,
}

pub async fn colonnes<'e>(
    executor: impl PgExecutor<'e>,
    track: TrackId,
) -> Result<Option<ColonnesDeLaPage>> {
    let ligne = sqlx::query_as!(
        ColonnesDeLaPage,
        r#"SELECT event_id, venue_id,
                  tagline AS "tagline: serde_json::Value",
                  to_jsonb(target_audiences) AS "target_audiences!: serde_json::Value",
                  support_note AS "support_note: serde_json::Value",
                  feature_kicker AS "feature_kicker: serde_json::Value",
                  feature_title AS "feature_title: serde_json::Value",
                  feature_body AS "feature_body: serde_json::Value",
                  feature_link_label AS "feature_link_label: serde_json::Value",
                  feature_link_url
             FROM event.programme_tracks
            WHERE id = $1"#,
        track.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne)
}

/// Les parties d'un fil, dans leur ordre.
pub async fn parties<'e>(executor: impl PgExecutor<'e>, track: TrackId) -> Result<Vec<TrackPart>> {
    let lignes = sqlx::query_as!(
        TrackPart,
        r#"SELECT id, track_id,
                  label AS "label: serde_json::Value",
                  title AS "title: serde_json::Value",
                  summary AS "summary: serde_json::Value",
                  sort_order
             FROM event.programme_track_parts
            WHERE track_id = $1
            ORDER BY sort_order, created_at"#,
        track.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Les moments sans activité d'un fil, dans l'ordre du temps.
pub async fn moments<'e>(
    executor: impl PgExecutor<'e>,
    track: TrackId,
) -> Result<Vec<TrackMoment>> {
    let lignes = sqlx::query_as!(
        TrackMoment,
        r#"SELECT id, track_id, part_id, starts_at, ends_at,
                  label AS "label!: serde_json::Value",
                  description AS "description: serde_json::Value"
             FROM event.programme_track_moments
            WHERE track_id = $1
            ORDER BY starts_at, created_at"#,
        track.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Écrire les colonnes de la page : écriture **totale**.
pub async fn ecrire(
    conn: &mut PgConnection,
    track: TrackId,
    p: &TrackPagePayload,
) -> std::result::Result<bool, sqlx::Error> {
    let publics = serde_json::Value::Array(p.target_audiences.clone());
    let touchees = sqlx::query!(
        r#"UPDATE event.programme_tracks SET
               venue_id           = $2,
               tagline            = $3::jsonb,
               target_audiences   = ARRAY(SELECT a::platform.i18n_text
                                            FROM jsonb_array_elements($4::jsonb) AS a),
               support_note       = $5::jsonb,
               feature_kicker     = $6::jsonb,
               feature_title      = $7::jsonb,
               feature_body       = $8::jsonb,
               feature_link_label = $9::jsonb,
               feature_link_url   = $10
         WHERE id = $1"#,
        track.as_uuid(),
        p.venue_id,
        p.tagline,
        publics,
        p.support_note,
        p.feature_kicker,
        p.feature_title,
        p.feature_body,
        p.feature_link_label,
        p.feature_link_url
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Les codes de rôle inconnus parmi ceux-là : un code absent de la taxonomie
/// ne doit pas faire disparaître un partenaire en silence.
pub async fn roles_inconnus(conn: &mut PgConnection, codes: &[String]) -> Result<Vec<String>> {
    let inconnus = sqlx::query_scalar!(
        r#"SELECT c AS "c!"
             FROM unnest($1::text[]) AS c
            WHERE NOT EXISTS (SELECT 1 FROM reference.taxonomy_terms t
                               WHERE t.taxonomy_code = 'track_partner_role'
                                 AND t.code = c AND t.is_active)"#,
        codes
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(inconnus)
}

/// **Remplacer** les partenaires : la liste envoyée est la liste voulue, dans
/// son ordre. Le service a dédoublonné et vérifié les rôles.
pub async fn remplacer_partenaires(
    conn: &mut PgConnection,
    track: TrackId,
    partenaires: &[TrackPartnerPayload],
) -> std::result::Result<(), sqlx::Error> {
    let organisations: Vec<Uuid> = partenaires.iter().map(|p| p.organization_id).collect();
    let roles: Vec<String> = partenaires.iter().map(|p| p.role_code.clone()).collect();

    sqlx::query!(
        "DELETE FROM event.programme_track_partners
          WHERE track_id = $1 AND NOT (organization_id = ANY($2))",
        track.as_uuid(),
        &organisations
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query!(
        "INSERT INTO event.programme_track_partners (track_id, organization_id, role_term_id, sort_order)
         SELECT $1, p.organization_id, t.id, p.rang
           FROM unnest($2::uuid[], $3::text[]) WITH ORDINALITY AS p(organization_id, role_code, rang)
           JOIN reference.taxonomy_terms t
             ON t.taxonomy_code = 'track_partner_role' AND t.code = p.role_code
         ON CONFLICT (track_id, organization_id)
         DO UPDATE SET role_term_id = EXCLUDED.role_term_id, sort_order = EXCLUDED.sort_order",
        track.as_uuid(),
        &organisations,
        &roles
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn creer_partie(
    conn: &mut PgConnection,
    track: TrackId,
    p: &TrackPartPayload,
) -> std::result::Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO event.programme_track_parts (track_id, label, title, summary, sort_order)
           VALUES ($1, $2::jsonb, $3::jsonb, $4::jsonb, $5)
        RETURNING id"#,
        track.as_uuid(),
        p.label,
        p.title,
        p.summary,
        p.sort_order
    )
    .fetch_one(&mut *conn)
    .await
}

/// Faux : la partie n'existe pas **dans ce fil**.
pub async fn modifier_partie(
    conn: &mut PgConnection,
    track: TrackId,
    part: Uuid,
    p: &TrackPartPayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.programme_track_parts SET
               label = $3::jsonb, title = $4::jsonb, summary = $5::jsonb, sort_order = $6
         WHERE id = $2 AND track_id = $1"#,
        track.as_uuid(),
        part,
        p.label,
        p.title,
        p.summary,
        p.sort_order
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Retirer une partie : ses moments et ses activités restent dans le fil, hors
/// partie (`ON DELETE SET NULL (part_id)`).
pub async fn supprimer_partie(conn: &mut PgConnection, track: TrackId, part: Uuid) -> Result<bool> {
    let touchees = sqlx::query!(
        "DELETE FROM event.programme_track_parts WHERE id = $2 AND track_id = $1",
        track.as_uuid(),
        part
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

pub async fn creer_moment(
    conn: &mut PgConnection,
    track: TrackId,
    p: &TrackMomentPayload,
) -> std::result::Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO event.programme_track_moments
               (track_id, part_id, starts_at, ends_at, label, description)
           VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb)
        RETURNING id"#,
        track.as_uuid(),
        p.part_id,
        p.starts_at,
        p.ends_at,
        p.label,
        p.description
    )
    .fetch_one(&mut *conn)
    .await
}

/// Faux : le moment n'existe pas **dans ce fil**.
pub async fn modifier_moment(
    conn: &mut PgConnection,
    track: TrackId,
    moment: Uuid,
    p: &TrackMomentPayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.programme_track_moments SET
               part_id = $3, starts_at = $4, ends_at = $5,
               label = $6::jsonb, description = $7::jsonb
         WHERE id = $2 AND track_id = $1"#,
        track.as_uuid(),
        moment,
        p.part_id,
        p.starts_at,
        p.ends_at,
        p.label,
        p.description
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

pub async fn supprimer_moment(
    conn: &mut PgConnection,
    track: TrackId,
    moment: Uuid,
) -> Result<bool> {
    let touchees = sqlx::query!(
        "DELETE FROM event.programme_track_moments WHERE id = $2 AND track_id = $1",
        track.as_uuid(),
        moment
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}
