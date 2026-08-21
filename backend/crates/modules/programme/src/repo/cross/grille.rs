//! Ce que le planificateur lit dans `event` — jours, salles, fils, canaux — et
//! les valeurs admises d'une réponse au formulaire.
//!
//! **Même espace de noms que `cross/mod.rs`, mêmes règles** : la question porte
//! sur les séances de ce module, la réponse vit ailleurs, et **aucune ligne
//! n'écrit**. Voir l'en-tête de `cross/mod.rs` pour le tableau des seize
//! lectures autorisées.
//!
//! Le partage avec `cross/mod.rs` est une affaire de volume, pas de frontière :
//! le fichier approchait des mille lignes du garde-fou, et B5 y ajoutait cinq
//! lectures.

use kernel::error::Result;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::ids::EventId;
use crate::domain::sessions::{PlannerChannel, PlannerDay, PlannerRoom, PlannerTrack};

/// Les jours du calendrier d'une édition — les colonnes de l'écran.
///
/// Vide pour un cycle de webinaires, qui n'a pas de calendrier : c'est un état
/// normal, jamais une erreur.
pub async fn jours_de_ledition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PlannerDay>> {
    let lignes = sqlx::query!(
        "SELECT id, day_date, title, is_featured, color_hex
           FROM event.event_days
          WHERE event_id = $1
          ORDER BY day_date",
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PlannerDay {
            id: l.id,
            day_date: l.day_date,
            title: l.title,
            is_featured: l.is_featured,
            color_hex: l.color_hex,
        })
        .collect())
}

/// Les salles d'une édition, **toutes lieux confondus**.
///
/// `is_virtual` n'est pas un ornement : c'est de lui que la base dérive
/// l'exclusivité de salle, et donc la gravité d'un chevauchement. Une salle
/// virtuelle accepte les créneaux simultanés sans occuper le stand.
pub async fn salles_de_ledition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PlannerRoom>> {
    let lignes = sqlx::query!(
        r#"SELECT r.id, r.name, r.code, r.capacity::int4 AS "capacity?",
                  r.is_virtual, r.has_streaming, r.sort_order
             FROM event.rooms r
             JOIN event.venues v ON v.id = r.venue_id
            WHERE v.event_id = $1
            ORDER BY r.sort_order, r.code"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PlannerRoom {
            id: l.id,
            name: l.name,
            code: l.code,
            capacity: l.capacity,
            is_virtual: l.is_virtual,
            has_streaming: l.has_streaming,
            sort_order: l.sort_order,
        })
        .collect())
}

/// La salle appartient-elle bien à cette édition ?
///
/// Ni la base ni aucun déclencheur ne le vérifient — seul le fil de
/// programmation est contrôlé (data-model § 3, ligne 15). Sans cette lecture,
/// une URL forgée installerait une séance de la COP31 dans une salle de la COP30.
pub async fn salle_de_ledition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
    room_id: Uuid,
) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM event.rooms r
                 JOIN event.venues v ON v.id = r.venue_id
                WHERE r.id = $2 AND v.event_id = $1
           ) AS "existe!""#,
        event_id.as_uuid(),
        room_id
    )
    .fetch_one(executor)
    .await?;

    Ok(existe)
}

/// Les fils de programmation d'une édition — les journées spéciales offertes au
/// rattachement.
pub async fn fils_de_ledition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PlannerTrack>> {
    let lignes = sqlx::query!(
        r#"SELECT id, title, kind::text AS "kind!", color_hex, starts_on, ends_on
             FROM event.programme_tracks
            WHERE event_id = $1
            ORDER BY sort_order, code"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PlannerTrack {
            id: l.id,
            title: l.title,
            kind: l.kind,
            color_hex: l.color_hex,
            starts_on: l.starts_on,
            ends_on: l.ends_on,
        })
        .collect())
}

/// Les canaux **applicables** à une édition : les siens, et ceux de la
/// plateforme.
///
/// Les canaux généraux portent `event_id IS NULL` — c'est ainsi que le
/// déclencheur les retient à défaut d'un canal d'édition. Les canaux
/// **désactivés** sont écartés : ils ne s'offrent plus au choix.
pub async fn canaux_applicables<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PlannerChannel>> {
    let lignes = sqlx::query!(
        "SELECT id, name, provider, is_default
           FROM event.broadcast_channels
          WHERE is_active AND (event_id = $1 OR event_id IS NULL)
          ORDER BY (event_id IS NOT NULL) DESC, is_default DESC, code",
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PlannerChannel {
            id: l.id,
            name: l.name,
            provider: l.provider,
            is_default: l.is_default,
        })
        .collect())
}

/// Le canal est-il **actif** et applicable à cette édition ?
///
/// Comme pour la salle, rien en base ne le vérifie : la clé étrangère accepte
/// n'importe quel canal, y compris celui d'une autre édition ou un canal retiré.
pub async fn canal_applicable<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
    channel_id: Uuid,
) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM event.broadcast_channels
                WHERE id = $2 AND is_active AND (event_id = $1 OR event_id IS NULL)
           ) AS "existe!""#,
        event_id.as_uuid(),
        channel_id
    )
    .fetch_one(executor)
    .await?;

    Ok(existe)
}

/// La journée du calendrier appartient-elle à cette édition ?
pub async fn jour_de_ledition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
    day_id: Uuid,
) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM event.event_days WHERE id = $2 AND event_id = $1
           ) AS "existe!""#,
        event_id.as_uuid(),
        day_id
    )
    .fetch_one(executor)
    .await?;

    Ok(existe)
}

// -----------------------------------------------------------------------------
// Les valeurs admises d'une réponse
//
// Une par lecture, et **jamais une par champ** : un formulaire de six questions
// à choix produirait sinon six requêtes là où deux suffisent (R15).
// -----------------------------------------------------------------------------

/// Les codes ISO 3166-1 alpha-2 du référentiel — ce qu'une réponse « pays »
/// peut valoir (R18).
pub async fn codes_pays<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<String>> {
    let codes = sqlx::query_scalar!("SELECT iso2 FROM reference.countries ORDER BY iso2")
        .fetch_all(executor)
        .await?;

    Ok(codes)
}

/// Les codes **actifs** de plusieurs taxonomies, en une lecture.
///
/// Rend le couple `(taxonomie, code)` : le service en compose la liste d'options
/// de chaque champ, sans revenir en base.
pub async fn codes_de_taxonomies<'e>(
    executor: impl PgExecutor<'e>,
    taxonomies: &[String],
) -> Result<Vec<(String, String)>> {
    let lignes = sqlx::query!(
        "SELECT taxonomy_code, code
           FROM reference.taxonomy_terms
          WHERE taxonomy_code = ANY($1) AND is_active
          ORDER BY taxonomy_code, sort_order, code",
        taxonomies
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| (l.taxonomy_code, l.code))
        .collect())
}

/// Les pastilles d'une **séance** — libellé traduit et couleur venus de la base.
///
/// Le pendant de `pastilles_du_dossier` pour la seconde entité que B5 rattache
/// à la taxonomie des thématiques.
pub async fn pastilles_de_la_seance<'e>(
    executor: impl PgExecutor<'e>,
    session_id: Uuid,
) -> Result<serde_json::Value> {
    let pastilles = sqlx::query_scalar!(
        r#"SELECT reference.term_badges('programme', 'sessions', $1, 'activity_theme')
               AS "pastilles!""#,
        session_id
    )
    .fetch_one(executor)
    .await?;

    Ok(pastilles)
}
