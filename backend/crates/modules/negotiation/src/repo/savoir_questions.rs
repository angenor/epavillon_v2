//! Les questions aux experts. Les lectures de la file ne lisent ni `asker_id`
//! ni `answered_by` pour les rendre (R9) ; seule `a_trancher` lit l'adresse de
//! l'auteure, pour la charge du courriel.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::savoir_questions::{AdminQuestion, MyQuestion};

/// `None` : la même référence existe déjà pour cette personne (rejeu).
pub async fn inserer(
    conn: &mut PgConnection,
    auteure: Uuid,
    client_ref: Uuid,
    theme_id: Uuid,
    body: &str,
    consentement: bool,
) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar!(
        "INSERT INTO negotiation.expert_questions
             (asker_id, client_ref, theme_term_id, body, consent_to_faq)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (asker_id, client_ref) DO NOTHING
         RETURNING id",
        auteure,
        client_ref,
        theme_id,
        body,
        consentement
    )
    .fetch_optional(conn)
    .await?)
}

pub async fn par_reference(
    conn: &mut PgConnection,
    auteure: Uuid,
    client_ref: Uuid,
) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar!(
        "SELECT id FROM negotiation.expert_questions WHERE asker_id = $1 AND client_ref = $2",
        auteure,
        client_ref
    )
    .fetch_optional(conn)
    .await?)
}

/// Les questions de la personne, la plus récente d'abord ; `id` en restreint
/// la lecture à une seule.
pub async fn miennes(
    conn: &mut PgConnection,
    auteure: Uuid,
    id: Option<Uuid>,
    locale: &str,
) -> Result<Vec<MyQuestion>> {
    Ok(sqlx::query_as!(
        MyQuestion,
        r#"SELECT q.id, q.client_ref, t.code AS "theme_code!",
                  COALESCE(platform.t(t.label, $3), t.code) AS "theme_label!",
                  q.body, q.consent_to_faq, q.status::text AS "status!", q.answer,
                  CASE WHEN e.id IS NULL THEN NULL
                       ELSE concat_ws(' ', e.first_name, e.last_name) END AS answered_by_name,
                  q.answered_at, q.created_at
             FROM negotiation.expert_questions q
             JOIN reference.taxonomy_terms t ON t.id = q.theme_term_id
             LEFT JOIN identity.people e ON e.id = q.answered_by
            WHERE q.asker_id = $1 AND ($2::uuid IS NULL OR q.id = $2)
            ORDER BY q.created_at DESC, q.id DESC"#,
        auteure,
        id,
        locale
    )
    .fetch_all(conn)
    .await?)
}

/// La part « questions » de la file : celles qui attendent, la plus ancienne
/// d'abord ; puis celles répondues depuis trente jours et pas encore promues,
/// la plus récente d'abord — c'est là que l'expert les promeut.
pub async fn file(conn: &mut PgConnection, id: Option<Uuid>, locale: &str) -> Result<Vec<AdminQuestion>> {
    Ok(sqlx::query_as!(
        AdminQuestion,
        r#"SELECT q.id, t.code AS "theme_code!",
                  COALESCE(platform.t(t.label, $2), t.code) AS "theme_label!",
                  q.body, q.consent_to_faq, q.status::text AS "status!", q.answer,
                  q.answered_at, q.faq_entry_id, q.created_at
             FROM negotiation.expert_questions q
             JOIN reference.taxonomy_terms t ON t.id = q.theme_term_id
            WHERE CASE WHEN $1::uuid IS NULL
                       THEN q.status = 'pending'
                            OR (q.status = 'answered' AND q.answered_at > now() - interval '30 days')
                       ELSE q.id = $1 END
            ORDER BY q.status <> 'pending',
                     CASE WHEN q.status = 'pending' THEN q.created_at END,
                     q.answered_at DESC, q.id"#,
        id,
        locale
    )
    .fetch_all(conn)
    .await?)
}

pub struct ATrancher {
    pub status: String,
    pub body: String,
    pub answer: Option<String>,
    pub email: String,
    pub first_name: String,
    pub locale: String,
}

pub async fn a_trancher(conn: &mut PgConnection, id: Uuid) -> Result<Option<ATrancher>> {
    Ok(sqlx::query_as!(
        ATrancher,
        r#"SELECT q.status::text AS "status!", q.body, q.answer,
                  p.primary_email::text AS "email!", p.first_name,
                  p.preferred_locale AS "locale!"
             FROM negotiation.expert_questions q
             JOIN identity.people p ON p.id = q.asker_id
            WHERE q.id = $1
              FOR UPDATE OF q"#,
        id
    )
    .fetch_optional(conn)
    .await?)
}

pub async fn repondre(conn: &mut PgConnection, id: Uuid, answer: &str, expert: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.expert_questions
            SET status = 'answered', answer = $2, answered_by = $3, answered_at = now()
          WHERE id = $1",
        id,
        answer,
        expert
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Le brouillon né de la question : **sans auteur**, relié par
/// `origin_question_id` seulement. Le texte va sous `fr`, que le modèle exige ;
/// l'expert le réécrit avant publication.
pub async fn promouvoir(
    conn: &mut PgConnection,
    id: Uuid,
    section_id: Uuid,
    question: &str,
    answer: &str,
) -> Result<Uuid> {
    let entree = sqlx::query_scalar!(
        "INSERT INTO negotiation.faq_entries
             (section_term_id, question, answer, origin_question_id, created_by)
         VALUES ($1, jsonb_build_object('fr', $2::text)::platform.i18n_text,
                 jsonb_build_object('fr', $3::text)::platform.i18n_text, $4, NULL)
         RETURNING id",
        section_id,
        question,
        answer,
        id
    )
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE negotiation.expert_questions
            SET status = 'added_to_faq', faq_entry_id = $2
          WHERE id = $1",
        id,
        entree
    )
    .execute(conn)
    .await?;
    Ok(entree)
}
