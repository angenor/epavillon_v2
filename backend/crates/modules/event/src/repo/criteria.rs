//! Lectures et écritures de `event.review_criteria` — la **grille** qui rend une
//! décision de sélection explicable.
//!
//! L'écriture se fait par **diff sur le code** (research.md § R9), dans la
//! transaction de l'appel. Le décompte des notes posées est joint ici, ligne à
//! ligne : c'est lui qui interdit le retrait d'un critère porteur de notes.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::domain::call::{CritereExistant, CriterionPayload};
use crate::domain::detail::EditionCriterion;
use crate::domain::ids::CallId;

/// La grille d'un appel, dans son ordre d'affichage.
///
/// Les barèmes sont rendus en flottant : le contrat du front les lit comme des
/// nombres, et une chaîne l'obligerait à les convertir pour additionner.
pub async fn de_l_appel<'e>(
    executor: impl PgExecutor<'e>,
    call_id: CallId,
) -> Result<Vec<EditionCriterion>> {
    let lignes = sqlx::query!(
        r#"SELECT id, code, label, description,
                  max_score::float8 AS "max_score!",
                  weight::float8    AS "weight!",
                  is_knockout, sort_order
             FROM event.review_criteria
            WHERE call_id = $1
            ORDER BY sort_order, code"#,
        call_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| EditionCriterion {
            id: Some(l.id),
            code: l.code,
            label: l.label,
            description: l.description,
            max_score: l.max_score,
            weight: l.weight,
            is_knockout: l.is_knockout,
            sort_order: l.sort_order,
            // Posé par le service, depuis `repo/cross.rs`.
            score_count: 0,
        })
        .collect())
}

/// Les critères **tels que le diff les compare** : identifiant, code, libellé et
/// barème.
///
/// `score_count` reste à zéro ici : les notes vivent dans `programme`, et toute
/// lecture hors du schéma `event` se fait dans `repo/cross.rs`, où la frontière
/// se relit (research.md § R14). Le service recolle les deux.
pub async fn existants(conn: &mut PgConnection, call_id: CallId) -> Result<Vec<CritereExistant>> {
    let lignes = sqlx::query!(
        r#"SELECT id, code, label,
                  max_score::float8 AS "max_score!",
                  weight::float8    AS "weight!"
             FROM event.review_criteria
            WHERE call_id = $1
            ORDER BY sort_order, code"#,
        call_id.as_uuid()
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| CritereExistant {
            id: l.id,
            code: l.code,
            label: l.label,
            max_score: l.max_score,
            weight: l.weight,
            score_count: 0,
        })
        .collect())
}

/// Insérer une ligne de grille. L'erreur est rendue **brute** : c'est le service
/// qui sait à quel rang de la charge utile elle se rapporte.
pub async fn inserer(
    conn: &mut PgConnection,
    call_id: CallId,
    c: &CriterionPayload,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO event.review_criteria
               (call_id, code, label, description, max_score, weight, is_knockout, sort_order)
           VALUES ($1, $2, $3::jsonb, $4::jsonb,
                   $5::float8::numeric(5,2), $6::float8::numeric(5,2), $7, $8)"#,
        call_id.as_uuid(),
        c.code,
        c.label,
        c.description,
        c.max_score,
        c.weight,
        c.is_knockout,
        c.sort_order
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Modifier une ligne conservée. **Le code n'y figure pas** : c'est la clé du
/// diff, et le changer ferait de la ligne une autre ligne — une insertion et une
/// suppression, ce que le diff a déjà décidé.
pub async fn modifier(
    conn: &mut PgConnection,
    id: Uuid,
    c: &CriterionPayload,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE event.review_criteria SET
               label       = $2::jsonb,
               description = $3::jsonb,
               max_score   = $4::float8::numeric(5,2),
               weight      = $5::float8::numeric(5,2),
               is_knockout = $6,
               sort_order  = $7
         WHERE id = $1"#,
        id,
        c.label,
        c.description,
        c.max_score,
        c.weight,
        c.is_knockout,
        c.sort_order
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Supprimer une ligne. **Le service a compté ses notes avant d'en arriver là**
/// (research.md § R9) : `xmod_fk_review_scores_criterion` est `ON DELETE
/// CASCADE`, et cet ordre effacerait sans un mot l'argumentaire d'une décision
/// de sélection.
pub async fn supprimer(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!("DELETE FROM event.review_criteria WHERE id = $1", id)
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// **La grille par défaut, lue en base et jamais recopiée** (FR-062).
///
/// `event.seed_default_criteria()` écrit : elle ne sait poser ses six lignes que
/// sur un appel. On lui en donne un **jetable**, dans une transaction qu'on
/// annule — l'édition et l'appel créés n'existent que le temps de la lecture, et
/// rien n'en subsiste, pas même une ligne d'audit.
///
/// Recopier les six libellés bilingues, leurs poids et l'éliminatoire dans un
/// tableau Rust en ferait une seconde vérité, désynchronisée du modèle au
/// premier ajustement de la grille — le défaut n° 1 de la v1 appliqué à une
/// grille d'évaluation.
pub async fn grille_par_defaut(pool: &PgPool) -> Result<Vec<EditionCriterion>> {
    let mut tx = pool.begin().await?;

    let edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2000, '{"fr":"Gabarit de grille"}'::jsonb,
                   ('grille-par-defaut-' || gen_random_uuid())::platform.slug,
                   '{"fr":"Édition jetable, jamais validée."}'::jsonb,
                   'online', 'UTC'::platform.timezone_name,
                   now(), now() + interval '1 day')
        RETURNING id"#
    )
    .fetch_one(&mut *tx)
    .await?;

    let appel = sqlx::query_scalar!(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, opens_at, closes_at)
           VALUES ($1, 'gabarit', '{"fr":"Gabarit"}'::jsonb, now(), now() + interval '1 day')
        RETURNING id"#,
        edition
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!("SELECT event.seed_default_criteria($1)", appel)
        .execute(&mut *tx)
        .await?;

    let grille = de_l_appel(&mut *tx, CallId::from(appel)).await?;

    // **Rien ne subsiste.** L'édition et l'appel jetables disparaissent avec la
    // transaction ; c'est ce qui autorise cette écriture dans une route de
    // lecture.
    tx.rollback().await?;

    Ok(grille
        .into_iter()
        .map(|mut c| {
            // Les identifiants viennent d'une transaction annulée : les rendre
            // ferait croire à l'écran que ces lignes existent. Une grille
            // proposée est faite de lignes NOUVELLES.
            c.id = None;
            c
        })
        .collect())
}
