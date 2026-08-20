//! Lectures et écritures de `event.programme_tracks` — les **journées
//! spéciales**, qui ne sont pas des jours du calendrier.
//!
//! **Rien de ce qui vit hors du schéma `event` n'est ici** : les décomptes de
//! séances (`programme.session_tracks`), les thématiques
//! (`reference.entity_terms`) et le **nom du responsable** (`identity.people`)
//! se lisent dans `repo/cross.rs`, où la frontière se relit. Le service les pose
//! sur les lignes rendues ici. Joindre `identity.people` d'ici aurait été le
//! premier pas vers une frontière invisible (research.md § R14).
//!
//! L'**écriture** des thématiques vit dans `repo/themes.rs`, seul endroit du
//! module qui écrive hors de son schéma — et l'écart y est consigné.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;

use crate::domain::detail::EditionTrack;
use crate::domain::ids::{EventId, TrackId};
use crate::domain::tabs::EditionTrackPayload;

/// Les fils d'une édition, dans leur ordre d'affichage.
pub async fn de_l_edition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<EditionTrack>> {
    let lignes = sqlx::query!(
        r#"SELECT id, code, slug::text AS "slug!", kind::text AS "kind!",
                  title, subtitle, description,
                  starts_on, ends_on, color_hex,
                  curated_by, published_at, sort_order
             FROM event.programme_tracks
            WHERE event_id = $1
            ORDER BY sort_order, starts_on NULLS LAST, code"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| EditionTrack {
            id: l.id,
            code: l.code,
            slug: l.slug,
            kind: l.kind,
            title: l.title,
            subtitle: l.subtitle,
            description: l.description,
            starts_on: l.starts_on,
            ends_on: l.ends_on,
            color_hex: l.color_hex,
            curated_by: l.curated_by,
            published_at: l.published_at,
            sort_order: l.sort_order,
            // Posés par le service, depuis `repo/cross.rs`.
            curator_name: None,
            session_count: 0,
            themes: Vec::new(),
        })
        .collect())
}

/// Créer un fil. L'erreur est rendue **brute** : c'est le service qui sait à
/// quel refus du contrat une contrainte se rapporte.
///
/// `published_at` est posée **dans le même enregistrement** que le reste : le
/// contrat porte `is_published`, et ouvrir la page publique d'un fil n'est pas
/// un geste séparé.
pub async fn creer(
    conn: &mut PgConnection,
    event_id: EventId,
    p: &EditionTrackPayload,
) -> std::result::Result<TrackId, sqlx::Error> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO event.programme_tracks
               (event_id, code, slug, kind, title, subtitle, description,
                starts_on, ends_on, color_hex, curated_by, published_at, sort_order)
           VALUES ($1, $2, $3::text::platform.slug, $4::text::event.track_kind,
                   $5::jsonb, $6::jsonb, $7::jsonb, $8, $9, $10, $11,
                   CASE WHEN $12 THEN now() END, $13)
        RETURNING id"#,
        event_id.as_uuid(),
        p.code,
        p.slug,
        p.kind,
        p.title,
        p.subtitle,
        p.description,
        p.starts_on,
        p.ends_on,
        p.color_hex,
        p.curated_by,
        p.is_published,
        p.sort_order
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(TrackId::from(id))
}

/// Modifier un fil — écriture **totale**, `event_id` excepté : un fil ne change
/// pas d'édition.
///
/// **`published_at` ne se réécrit pas quand elle existe déjà** : refermer puis
/// rouvrir une page publique ne doit pas effacer la date de sa première
/// ouverture, qui est ce que l'écran affiche.
pub async fn modifier(
    conn: &mut PgConnection,
    id: TrackId,
    p: &EditionTrackPayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.programme_tracks SET
               code         = $2,
               slug         = $3::text::platform.slug,
               kind         = $4::text::event.track_kind,
               title        = $5::jsonb,
               subtitle     = $6::jsonb,
               description  = $7::jsonb,
               starts_on    = $8,
               ends_on      = $9,
               color_hex    = $10,
               curated_by   = $11,
               published_at = CASE WHEN $12 THEN COALESCE(published_at, now()) END,
               sort_order   = $13
         WHERE id = $1"#,
        id.as_uuid(),
        p.code,
        p.slug,
        p.kind,
        p.title,
        p.subtitle,
        p.description,
        p.starts_on,
        p.ends_on,
        p.color_hex,
        p.curated_by,
        p.is_published,
        p.sort_order
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Supprimer un fil.
///
/// **Aucune séance n'est supprimée** : ce qui disparaît, ce sont les
/// rattachements `programme.session_tracks`, par cascade. C'est du travail
/// éditorial perdu, et le service l'a chiffré avant (research.md § R8).
pub async fn supprimer(conn: &mut PgConnection, id: TrackId) -> Result<bool> {
    let touchees = sqlx::query!(
        "DELETE FROM event.programme_tracks WHERE id = $1",
        id.as_uuid()
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}
