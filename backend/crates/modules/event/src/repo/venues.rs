//! Lectures et écritures de `event.venues` et `event.rooms` — le stand et ses
//! salles, ce qui permet à un conflit d'avoir un **sujet nommable**.
//!
//! Les décomptes de séances — ce qu'un retrait déplacerait — viennent de
//! `repo/cross.rs`, où la frontière se relit.

use kernel::error::Result;
use sqlx::postgres::PgConnection;

use crate::domain::detail::{EditionRoom, EditionVenue};
use crate::domain::ids::{EventId, RoomId, VenueId};
use crate::domain::tabs::{EditionRoomPayload, EditionVenuePayload};

/// Les lieux d'une édition, **chacun avec ses salles**.
///
/// Deux requêtes plutôt qu'une jointure : un lieu sans salle doit rester
/// visible dans son propre onglet, et l'agrégation en Rust évite le `LEFT JOIN`
/// dont il faudrait ensuite démêler les lignes nulles.
pub async fn de_l_edition(conn: &mut PgConnection, event_id: EventId) -> Result<Vec<EditionVenue>> {
    let lieux = sqlx::query!(
        r#"SELECT id, name, kind, address, map_url::text AS "map_url?"
             FROM event.venues
            WHERE event_id = $1
            ORDER BY name->>'fr', id"#,
        event_id.as_uuid()
    )
    .fetch_all(&mut *conn)
    .await?;

    let salles = sqlx::query!(
        r#"SELECT r.id, r.venue_id, r.name, r.code, r.capacity,
                  r.is_virtual, r.has_streaming, r.equipment, r.sort_order
             FROM event.rooms r
             JOIN event.venues v ON v.id = r.venue_id
            WHERE v.event_id = $1
            ORDER BY r.sort_order, r.code"#,
        event_id.as_uuid()
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(lieux
        .into_iter()
        .map(|l| EditionVenue {
            id: l.id,
            name: l.name,
            kind: l.kind,
            address: l.address,
            map_url: l.map_url,
            rooms: salles
                .iter()
                .filter(|s| s.venue_id == l.id)
                .map(|s| EditionRoom {
                    id: s.id,
                    venue_id: s.venue_id,
                    name: s.name.clone(),
                    code: s.code.clone(),
                    capacity: s.capacity,
                    is_virtual: s.is_virtual,
                    has_streaming: s.has_streaming,
                    equipment: s.equipment.clone(),
                    sort_order: s.sort_order,
                    // Posé par le service, depuis `repo/cross.rs`.
                    session_count: 0,
                })
                .collect(),
        })
        .collect())
}

/// Créer un lieu. L'erreur est rendue **brute** : c'est le service qui sait à
/// quel refus du contrat une contrainte se rapporte.
pub async fn creer_lieu(
    conn: &mut PgConnection,
    event_id: EventId,
    p: &EditionVenuePayload,
) -> std::result::Result<VenueId, sqlx::Error> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO event.venues (event_id, name, kind, address, map_url)
           VALUES ($1, $2::jsonb, $3, $4, $5::text::platform.url)
        RETURNING id"#,
        event_id.as_uuid(),
        p.name,
        p.kind,
        p.address,
        p.map_url
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(VenueId::from(id))
}

/// Modifier un lieu — écriture **totale**, `event_id` excepté : un lieu ne
/// change pas d'édition.
pub async fn modifier_lieu(
    conn: &mut PgConnection,
    id: VenueId,
    p: &EditionVenuePayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.venues SET
               name    = $2::jsonb,
               kind    = $3,
               address = $4,
               map_url = $5::text::platform.url
         WHERE id = $1"#,
        id.as_uuid(),
        p.name,
        p.kind,
        p.address,
        p.map_url
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Retirer un lieu — **et ses salles avec lui** (`ON DELETE CASCADE`).
///
/// Les séances qui s'y tenaient ne disparaissent pas : `sessions.room_id` est
/// `ON DELETE SET NULL`, elles retournent au panneau « à placer ». **Le service
/// les a comptées avant** (research.md § R8).
pub async fn supprimer_lieu(conn: &mut PgConnection, id: VenueId) -> Result<bool> {
    let touchees = sqlx::query!("DELETE FROM event.venues WHERE id = $1", id.as_uuid())
        .execute(&mut *conn)
        .await?
        .rows_affected();

    Ok(touchees == 1)
}

/// Créer une salle.
pub async fn creer_salle(
    conn: &mut PgConnection,
    p: &EditionRoomPayload,
) -> std::result::Result<RoomId, sqlx::Error> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO event.rooms
               (venue_id, name, code, capacity, is_virtual, has_streaming, equipment, sort_order)
           VALUES ($1, $2::jsonb, $3, $4, $5, $6, $7, $8)
        RETURNING id"#,
        p.venue_id,
        p.name,
        p.code,
        p.capacity,
        p.is_virtual,
        p.has_streaming,
        &p.equipment,
        p.sort_order
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(RoomId::from(id))
}

/// Modifier une salle. `venue_id` en fait partie : déplacer une salle d'un lieu
/// à l'autre du **même événement** est un geste légitime — le service a déjà
/// vérifié que le lieu visé appartient à l'édition.
pub async fn modifier_salle(
    conn: &mut PgConnection,
    id: RoomId,
    p: &EditionRoomPayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.rooms SET
               venue_id      = $2,
               name          = $3::jsonb,
               code          = $4,
               capacity      = $5,
               is_virtual    = $6,
               has_streaming = $7,
               equipment     = $8,
               sort_order    = $9
         WHERE id = $1"#,
        id.as_uuid(),
        p.venue_id,
        p.name,
        p.code,
        p.capacity,
        p.is_virtual,
        p.has_streaming,
        &p.equipment,
        p.sort_order
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

pub async fn supprimer_salle(conn: &mut PgConnection, id: RoomId) -> Result<bool> {
    let touchees = sqlx::query!("DELETE FROM event.rooms WHERE id = $1", id.as_uuid())
        .execute(&mut *conn)
        .await?
        .rows_affected();

    Ok(touchees == 1)
}

/// L'édition d'un lieu, **relue dans la transaction d'écriture**.
///
/// La route a déjà vérifié le périmètre ; ceci répond à une autre question :
/// « ce lieu appartient-il bien à l'édition où l'on écrit ? ». Sans elle, une
/// salle pourrait être posée dans le lieu d'une autre édition, que l'appelant
/// administre peut-être aussi.
pub async fn edition_du_lieu(
    conn: &mut PgConnection,
    venue_id: VenueId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT event_id FROM event.venues WHERE id = $1",
        venue_id.as_uuid()
    )
    .fetch_optional(&mut *conn)
    .await?;

    Ok(id.map(EventId::from))
}
