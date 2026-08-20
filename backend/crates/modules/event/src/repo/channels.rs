//! Lectures et écritures de `event.broadcast_channels` — le canal du direct,
//! **ressource réservable** au même titre qu'une salle.
//!
//! Deux index gouvernent l'écriture, et tous deux ont une **asymétrie** qu'il
//! faut avoir en tête : `ux_broadcast_channels_code` est `NULLS NOT DISTINCT`,
//! et `ux_broadcast_channels_default` regroupe les canaux généraux sous un
//! identifiant de substitution (research.md § R6).
//!
//! Les décomptes de séances diffusées viennent de `repo/cross.rs`.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;

use crate::domain::detail::EditionChannel;
use crate::domain::ids::{ChannelId, EventId};
use crate::domain::tabs::EditionChannelPayload;

/// Les canaux que l'onglet affiche : **ceux de l'édition et ceux de la
/// plateforme**, comme le front les compose déjà.
///
/// Un canal général porte `event_id IS NULL`. Il n'est pas modifiable depuis
/// une édition — c'est le refus `platform_channel` — mais il doit s'afficher :
/// le semis en pose un, `ifdd_principal`, et le taire ferait croire à l'équipe
/// qu'aucun canal n'existe.
///
/// **Ceux de l'édition d'abord**, les généraux ensuite : c'est l'ordre dans
/// lequel l'écran les lit.
pub async fn de_l_edition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<EditionChannel>> {
    let lignes = sqlx::query!(
        r#"SELECT id, event_id, code, name, provider, channel_ref, locale,
                  is_default, is_active
             FROM event.broadcast_channels
            WHERE event_id = $1 OR event_id IS NULL
            ORDER BY (event_id IS NULL), is_default DESC, code"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| EditionChannel {
            id: l.id,
            event_id: l.event_id,
            code: l.code,
            name: l.name,
            provider: l.provider,
            channel_ref: l.channel_ref,
            locale: l.locale,
            is_default: l.is_default,
            is_active: l.is_active,
            // Posé par le service, depuis `repo/cross.rs`.
            session_count: 0,
        })
        .collect())
}

/// **Retirer le défaut du groupe, AVANT d'en poser un nouveau** (research.md
/// § R6).
///
/// `ux_broadcast_channels_default` est un index unique **partiel** sur
/// `COALESCE(event_id, …)`, restreint aux canaux `is_default AND is_active`. Il
/// n'est **pas différable** : poser d'abord violerait l'unicité au milieu de la
/// transaction. Retirer d'abord est la seule séquence qui passe.
///
/// Le `COALESCE` n'est pas une commodité : il reproduit **exactement** le groupe
/// de l'index. Les canaux généraux de la plateforme forment leur propre groupe,
/// sous un identifiant de substitution — poser un défaut d'édition **ne déloge
/// donc pas** le canal général semé, et c'est voulu : il sert les diffusions dont
/// l'événement n'a pas le sien.
pub async fn retirer_le_defaut(conn: &mut PgConnection, event_id: EventId) -> Result<()> {
    sqlx::query!(
        "UPDATE event.broadcast_channels
            SET is_default = false
          WHERE COALESCE(event_id, '00000000-0000-0000-0000-000000000000'::uuid)
              = COALESCE($1::uuid, '00000000-0000-0000-0000-000000000000'::uuid)
            AND is_default",
        event_id.as_uuid()
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Créer un canal d'édition.
pub async fn creer(
    conn: &mut PgConnection,
    event_id: EventId,
    p: &EditionChannelPayload,
) -> std::result::Result<ChannelId, sqlx::Error> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO event.broadcast_channels
               (event_id, code, name, provider, channel_ref, locale, is_default, is_active)
           VALUES ($1, $2, $3::jsonb, $4, $5, $6, $7, $8)
        RETURNING id"#,
        event_id.as_uuid(),
        p.code,
        p.name,
        p.provider,
        p.channel_ref,
        p.locale,
        p.is_default,
        p.is_active
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(ChannelId::from(id))
}

/// Modifier un canal — **d'édition seulement** : le service a écarté les canaux
/// généraux de la plateforme avant d'arriver ici (`platform_channel`).
pub async fn modifier(
    conn: &mut PgConnection,
    id: ChannelId,
    p: &EditionChannelPayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.broadcast_channels SET
               code        = $2,
               name        = $3::jsonb,
               provider    = $4,
               channel_ref = $5,
               locale      = $6,
               is_default  = $7,
               is_active   = $8
         WHERE id = $1"#,
        id.as_uuid(),
        p.code,
        p.name,
        p.provider,
        p.channel_ref,
        p.locale,
        p.is_default,
        p.is_active
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// **Désactiver** un canal qui a servi, plutôt que le supprimer (research.md
/// § R7).
///
/// La clé étrangère est `ON DELETE SET NULL` : aucune séance ne serait perdue.
/// Ce qui serait perdu, c'est **la trace du canal sur lequel une activité passée
/// a été diffusée** — précisément ce qu'un bilan d'édition va chercher.
pub async fn desactiver(conn: &mut PgConnection, id: ChannelId) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE event.broadcast_channels SET is_active = false, is_default = false
          WHERE id = $1",
        id.as_uuid()
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Supprimer un canal qui n'a jamais servi. Laisser s'accumuler des canaux créés
/// par erreur garderait leurs codes pris.
pub async fn supprimer(conn: &mut PgConnection, id: ChannelId) -> Result<bool> {
    let touchees = sqlx::query!(
        "DELETE FROM event.broadcast_channels WHERE id = $1",
        id.as_uuid()
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}
