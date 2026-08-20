//! Lectures et écritures de `event.event_days` — le **calendrier** d'une
//! édition, une ligne par jour.
//!
//! Rien en base ne dérive ces lignes : `event.event_days` n'a aucun déclencheur
//! de dérivation. La génération est donc écrite ici, et elle est **additive**
//! dans ce jalon — un enregistrement d'édition crée les journées manquantes et
//! **n'en supprime aucune** (FR-033). Le retrait est un geste séparé, explicite,
//! et il vit dans la génération du calendrier (phase 8).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::Date;

use crate::domain::detail::EditionDay;
use crate::domain::ids::{EventDayId, EventId};
use crate::domain::tabs::EditionDayPayload;

/// Les dates déjà créées, croissantes. C'est l'entrée du plan de génération.
pub async fn dates_existantes<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<Date>> {
    let dates = sqlx::query_scalar!(
        "SELECT day_date FROM event.event_days WHERE event_id = $1 ORDER BY day_date",
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(dates)
}

/// Crée les journées manquantes, chacune avec **sa date et son rang, et rien
/// d'autre** (FR-037).
///
/// Inventer « Jour 3 » produirait un titre que personne n'a écrit et qui
/// s'afficherait tel quel sur la page publique.
///
/// **Aucun `ON CONFLICT` ici, et c'est délibéré.** Les dates viennent d'un plan
/// recalculé dans cette transaction : `ux_event_days_date` ne peut donc pas être
/// violée, et si elle l'était, c'est que l'ordre du service a été inversé — un
/// défaut de code, que le noyau traduit en `INTERNAL` pour qu'il se voie. Un
/// `DO NOTHING` l'aurait rendu silencieux.
pub async fn creer(
    conn: &mut PgConnection,
    event_id: EventId,
    a_creer: &[(Date, i16)],
) -> Result<u64> {
    if a_creer.is_empty() {
        return Ok(0);
    }

    let (dates, rangs): (Vec<Date>, Vec<i16>) = a_creer.iter().copied().unzip();

    let creees = sqlx::query!(
        "INSERT INTO event.event_days (event_id, day_date, sort_order)
         SELECT $1, d.jour, d.rang
           FROM unnest($2::date[], $3::int2[]) AS d(jour, rang)",
        event_id.as_uuid(),
        &dates,
        &rangs
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(creees)
}

/// Les journées d'une édition, telles que l'onglet les affiche.
///
/// `is_outside_period` et `session_count` ne sont **pas** posés ici : la période
/// se calcule en base dans le fuseau de l'édition (§ R5) et le décompte des
/// séances vit dans `programme`. Le service les pose, avec ce qu'il a déjà lu.
pub async fn de_l_edition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<EditionDay>> {
    let lignes = sqlx::query!(
        r#"SELECT id, day_date, title, slug::text AS "slug?", description,
                  is_featured, color_hex, sort_order
             FROM event.event_days
            WHERE event_id = $1
            ORDER BY day_date"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| EditionDay {
            id: l.id,
            day_date: l.day_date,
            title: l.title,
            slug: l.slug,
            description: l.description,
            is_featured: l.is_featured,
            color_hex: l.color_hex,
            sort_order: l.sort_order,
            session_count: 0,
            is_outside_period: false,
        })
        .collect())
}

/// Les journées d'une édition **avec leur identifiant et leur date**, pour le
/// plan de génération. Il ne lui en faut pas davantage.
pub async fn journees_du_plan<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<(uuid::Uuid, Date)>> {
    let lignes = sqlx::query!(
        "SELECT id, day_date FROM event.event_days WHERE event_id = $1 ORDER BY day_date",
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.id, l.day_date)).collect())
}

/// L'habillage **éditorial** d'une journée — et rien d'autre.
///
/// **La date n'y est pas.** Une journée du calendrier tient sa date de la
/// période de l'édition ; la déplacer ferait un doublon ou un trou, et
/// `ux_event_days_date` refuserait le premier sans rien dire du second. Le rang
/// non plus : il est posé par la génération, qui seule connaît la période
/// entière.
pub async fn habiller(
    conn: &mut PgConnection,
    id: EventDayId,
    p: &EditionDayPayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.event_days SET
               title       = $2::jsonb,
               slug        = $3::text::platform.slug,
               description = $4::jsonb,
               is_featured = $5,
               color_hex   = $6
         WHERE id = $1"#,
        id.as_uuid(),
        p.title,
        p.slug,
        p.description,
        p.is_featured,
        p.color_hex
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Retirer les journées désignées. **Le service les a comptées avant** : les
/// séances qu'elles portaient perdent leur jour (`ON DELETE SET NULL`) et
/// survivent (research.md § R8).
pub async fn supprimer(conn: &mut PgConnection, ids: &[uuid::Uuid]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }

    let retirees = sqlx::query!(
        "DELETE FROM event.event_days WHERE id = ANY($1::uuid[])",
        ids
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(retirees)
}
