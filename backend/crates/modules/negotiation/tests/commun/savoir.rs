//! Les fabriques du savoir : entrée de FAQ, terme du lexique, étape du
//! parcours. Elles écrivent en SQL, comme le back-office le fera.

use super::Bac;
use uuid::Uuid;

/// Un terme semé par `020_reference.sql` : lu, jamais inventé.
pub async fn vocabulaire(bac: &Bac, taxonomie: &str, code: &str) -> Uuid {
    sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms WHERE taxonomy_code = $1 AND code = $2",
        taxonomie,
        code
    )
    .fetch_one(bac.pool())
    .await
    .expect("terme de vocabulaire semé")
}

/// Une entrée de la rubrique « Ma première COP ». Hors brouillon, elle porte
/// sa réponse et la vérification de l'expert.
pub async fn entree_faq(bac: &Bac, expert: Uuid, question: &str, statut: &str) -> Uuid {
    let rubrique = vocabulaire(bac, "faq_section", "first_cop").await;
    let brouillon = statut == "draft";
    sqlx::query_scalar!(
        r#"INSERT INTO negotiation.faq_entries
               (section_term_id, question, answer, status, verified_on, verified_by)
           VALUES ($1, jsonb_build_object('fr', $2::text)::platform.i18n_text,
                   CASE WHEN $4 THEN NULL
                        ELSE jsonb_build_object('fr', 'Réponse à : ' || $2::text)::platform.i18n_text END,
                   $3::text::negotiation.knowledge_status,
                   CASE WHEN $4 THEN NULL ELSE current_date END,
                   CASE WHEN $4 THEN NULL ELSE $5::uuid END)
           RETURNING id"#,
        rubrique,
        question,
        statut,
        brouillon,
        expert
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'entrée de FAQ")
}

pub async fn statut_faq(bac: &Bac, id: Uuid, statut: &str) {
    sqlx::query!(
        "UPDATE negotiation.faq_entries SET status = $2::text::negotiation.knowledge_status WHERE id = $1",
        id,
        statut
    )
    .execute(bac.pool())
    .await
    .expect("changement de statut");
}

/// Un terme de la famille « Réunions ».
pub async fn terme(
    bac: &Bac,
    term: &str,
    statut: &str,
    acronym: Option<&str>,
    variants: &[&str],
) -> Uuid {
    let famille = vocabulaire(bac, "glossary_family", "meetings").await;
    let variants: Vec<String> = variants.iter().map(|v| (*v).to_owned()).collect();
    sqlx::query_scalar!(
        r#"INSERT INTO negotiation.glossary_entries
               (slug, family_term_id, term, acronym, variants, translation, definition, status)
           VALUES ('', $1, $2, $3, $4, '{"fr":"traduction"}', '{"fr":"définition"}',
                   $5::text::negotiation.knowledge_status)
           RETURNING id"#,
        famille,
        term,
        acronym,
        &variants,
        statut
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du terme")
}

/// Un groupe publié et une étape publiée dedans, sans lien.
pub async fn etape(bac: &Bac, groupe: &str, libelle: &str) -> (Uuid, Uuid) {
    let groupe_id = sqlx::query_scalar!(
        r#"INSERT INTO negotiation.pathway_groups (label, is_published)
           VALUES (jsonb_build_object('fr', $1::text)::platform.i18n_text, true)
           RETURNING id"#,
        groupe
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du groupe");
    let etape_id = sqlx::query_scalar!(
        r#"INSERT INTO negotiation.pathway_steps (group_id, label, is_published)
           VALUES ($1, jsonb_build_object('fr', $2::text)::platform.i18n_text, true)
           RETURNING id"#,
        groupe_id,
        libelle
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'étape");
    (groupe_id, etape_id)
}

/// Recule `updated_at` d'une entrée, déclencheurs coupés le temps de
/// l'écriture : `tg_set_updated_at` le reposerait à `now()`.
pub async fn vieillir(bac: &Bac, table: &str, id: Uuid, minutes: i32) {
    let mut tx = bac.pool().begin().await.expect("transaction");
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .expect("déclencheurs coupés");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE negotiation.{table} SET updated_at = now() - make_interval(mins => $2) WHERE id = $1"
    )))
    .bind(id)
    .bind(minutes)
    .execute(&mut *tx)
    .await
    .expect("vieillissement");
    tx.commit().await.expect("validation");
}
