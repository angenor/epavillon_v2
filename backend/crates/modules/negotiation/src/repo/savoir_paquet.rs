//! La lecture du paquet du savoir : le publié entier, ou ce qui a changé
//! depuis un instant. `depuis` est déjà reculé du chevauchement par le service.

use std::collections::HashMap;

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::savoir::{
    FaqEntry, FaqSection, GlossaryEntry, GlossaryFamily, KnowledgeSource, KnowledgeStatus,
    PathwayGroup, PathwayLink, PathwayStep,
};

pub async fn empreinte(conn: &mut PgConnection) -> Result<String> {
    Ok(
        sqlx::query_scalar!(r#"SELECT negotiation.knowledge_fingerprint() AS "f!""#)
            .fetch_one(conn)
            .await?,
    )
}

pub async fn rubriques(conn: &mut PgConnection, locale: &str) -> Result<Vec<FaqSection>> {
    let lignes = sqlx::query!(
        r#"SELECT code AS "code!", platform.t(label, $1) AS "label!", icon,
                  sort_order::int AS "sort_order!"
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = 'faq_section' AND is_active
            ORDER BY sort_order, code"#,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| FaqSection {
            code: l.code,
            label: l.label,
            icon: l.icon,
            sort_order: l.sort_order,
        })
        .collect())
}

pub async fn familles(conn: &mut PgConnection, locale: &str) -> Result<Vec<GlossaryFamily>> {
    let lignes = sqlx::query!(
        r#"SELECT code AS "code!", platform.t(label, $1) AS "label!",
                  sort_order::int AS "sort_order!"
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = 'glossary_family' AND is_active
            ORDER BY sort_order, code"#,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| GlossaryFamily {
            code: l.code,
            label: l.label,
            sort_order: l.sort_order,
        })
        .collect())
}

/// Les entrées servies — `published` et `to_review` —, sources jointes.
pub async fn faq(
    conn: &mut PgConnection,
    locale: &str,
    depuis: Option<OffsetDateTime>,
) -> Result<Vec<FaqEntry>> {
    let lignes = sqlx::query!(
        r#"SELECT f.id, t.code AS "section_code!",
                  platform.t(f.question, $1) AS "question!", platform.t(f.answer, $1) AS "answer!",
                  f.status::text AS "status!", f.verified_on, f.updated_at,
                  COALESCE((SELECT array_agg(r.related_id ORDER BY r.sort_order, r.related_id)
                              FROM negotiation.faq_related r
                             WHERE r.entry_id = f.id), '{}') AS "related_ids!: Vec<Uuid>"
             FROM negotiation.faq_entries f
             JOIN reference.taxonomy_terms t ON t.id = f.section_term_id
            WHERE f.status <> 'draft'
              AND ($2::timestamptz IS NULL OR f.updated_at >= $2)
            ORDER BY t.sort_order, f.created_at, f.id"#,
        locale,
        depuis
    )
    .fetch_all(&mut *conn)
    .await?;

    let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
    let mut sources = sources(conn, locale, &ids, &[]).await?;
    Ok(lignes
        .into_iter()
        .map(|l| FaqEntry {
            sources: sources.remove(&l.id).unwrap_or_default(),
            id: l.id,
            section_code: l.section_code,
            question: l.question,
            answer: l.answer,
            status: KnowledgeStatus::depuis(&l.status),
            verified_on: l.verified_on,
            related_ids: l.related_ids,
            updated_at: l.updated_at,
        })
        .collect())
}

pub async fn lexique(
    conn: &mut PgConnection,
    locale: &str,
    depuis: Option<OffsetDateTime>,
) -> Result<Vec<GlossaryEntry>> {
    let lignes = sqlx::query!(
        r#"SELECT g.id, g.slug, t.code AS "family_code!", g.term, g.acronym, g.variants,
                  platform.t(g.translation, $1) AS "translation!",
                  platform.t(g.definition, $1) AS "definition!",
                  g.heard_in_room, g.status::text AS "status!", g.updated_at,
                  COALESCE((SELECT array_agg(r.related_id ORDER BY r.sort_order, r.related_id)
                              FROM negotiation.glossary_related r
                             WHERE r.entry_id = g.id), '{}') AS "related_ids!: Vec<Uuid>"
             FROM negotiation.glossary_entries g
             JOIN reference.taxonomy_terms t ON t.id = g.family_term_id
            WHERE g.status <> 'draft'
              AND ($2::timestamptz IS NULL OR g.updated_at >= $2)
            ORDER BY g.term_norm, g.id"#,
        locale,
        depuis
    )
    .fetch_all(&mut *conn)
    .await?;

    let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
    let mut sources = sources(conn, locale, &[], &ids).await?;
    Ok(lignes
        .into_iter()
        .map(|l| GlossaryEntry {
            sources: sources.remove(&l.id).unwrap_or_default(),
            id: l.id,
            slug: l.slug,
            family_code: l.family_code,
            term: l.term,
            acronym: l.acronym,
            variants: l.variants,
            translation: l.translation,
            definition: l.definition,
            heard_in_room: l.heard_in_room,
            related_ids: l.related_ids,
            status: KnowledgeStatus::depuis(&l.status),
            updated_at: l.updated_at,
        })
        .collect())
}

/// Les sources de ces entrées, rangées par entrée dans leur ordre.
async fn sources(
    conn: &mut PgConnection,
    locale: &str,
    faq: &[Uuid],
    lexique: &[Uuid],
) -> Result<HashMap<Uuid, Vec<KnowledgeSource>>> {
    if faq.is_empty() && lexique.is_empty() {
        return Ok(HashMap::new());
    }
    let lignes = sqlx::query!(
        r#"SELECT COALESCE(s.faq_entry_id, s.glossary_entry_id) AS "entree!",
                  s.document_id, platform.t(d.title, $1) AS document_title,
                  s.external_title, s.external_url::text AS external_url, s.section_label,
                  s.page_from, s.page_to, s.quote
             FROM negotiation.knowledge_sources s
             LEFT JOIN negotiation.documents d ON d.id = s.document_id
            WHERE s.faq_entry_id = ANY($2::uuid[]) OR s.glossary_entry_id = ANY($3::uuid[])
            ORDER BY s.sort_order, s.id"#,
        locale,
        faq,
        lexique
    )
    .fetch_all(conn)
    .await?;

    let mut par_entree: HashMap<Uuid, Vec<KnowledgeSource>> = HashMap::new();
    for l in lignes {
        par_entree
            .entry(l.entree)
            .or_default()
            .push(KnowledgeSource {
                document_id: l.document_id,
                document_title: l.document_title,
                external_title: l.external_title,
                external_url: l.external_url,
                section_label: l.section_label,
                page_from: l.page_from,
                page_to: l.page_to,
                quote: l.quote,
            });
    }
    Ok(par_entree)
}

/// Les entrées publiées une fois et sorties depuis : dépubliées, donc
/// revenues au brouillon. Un brouillon jamais publié n'a rien à retirer.
pub async fn sorties(
    conn: &mut PgConnection,
    depuis: OffsetDateTime,
) -> Result<(Vec<Uuid>, Vec<Uuid>)> {
    let faq = sqlx::query_scalar!(
        "SELECT id FROM negotiation.faq_entries
          WHERE status = 'draft' AND first_published_at IS NOT NULL AND updated_at >= $1
          ORDER BY id",
        depuis
    )
    .fetch_all(&mut *conn)
    .await?;
    let lexique = sqlx::query_scalar!(
        "SELECT id FROM negotiation.glossary_entries
          WHERE status = 'draft' AND first_published_at IS NOT NULL AND updated_at >= $1
          ORDER BY id",
        depuis
    )
    .fetch_all(conn)
    .await?;
    Ok((faq, lexique))
}

/// Le parcours publié, entier : groupes et étapes publiés, dans leur ordre.
pub async fn parcours(conn: &mut PgConnection, locale: &str) -> Result<Vec<PathwayGroup>> {
    let groupes = sqlx::query!(
        r#"SELECT id, platform.t(label, $1) AS "label!", sort_order::int AS "sort_order!"
             FROM negotiation.pathway_groups
            WHERE is_published
            ORDER BY sort_order, id"#,
        locale
    )
    .fetch_all(&mut *conn)
    .await?;
    let etapes = sqlx::query!(
        r#"SELECT s.id, s.group_id, platform.t(s.label, $1) AS "label!",
                  platform.t(s.detail, $1) AS detail, platform.t(s.origin_label, $1) AS origin_label,
                  s.link_kind, COALESCE(s.link_document_id, s.link_faq_id, s.link_glossary_id) AS link_target,
                  s.link_page, s.link_section, platform.t(s.link_label, $1) AS link_label,
                  s.sort_order::int AS "sort_order!"
             FROM negotiation.pathway_steps s
            WHERE s.is_published
            ORDER BY s.sort_order, s.id"#,
        locale
    )
    .fetch_all(conn)
    .await?;

    let mut par_groupe: HashMap<Uuid, Vec<PathwayStep>> = HashMap::new();
    for e in etapes {
        let link = match (e.link_kind, e.link_target) {
            (Some(kind), Some(target_id)) => Some(PathwayLink {
                kind,
                target_id,
                page: e.link_page,
                section: e.link_section,
                label: e.link_label,
            }),
            _ => None,
        };
        par_groupe.entry(e.group_id).or_default().push(PathwayStep {
            id: e.id,
            label: e.label,
            detail: e.detail,
            origin_label: e.origin_label,
            link,
            sort_order: e.sort_order,
        });
    }
    Ok(groupes
        .into_iter()
        .map(|g| PathwayGroup {
            steps: par_groupe.remove(&g.id).unwrap_or_default(),
            id: g.id,
            label: g.label,
            sort_order: g.sort_order,
        })
        .collect())
}

/// Les trois entrées servies les plus lues sur trente jours ; à défaut de
/// lectures, l'ordre éditorial complète.
pub async fn plus_lues(conn: &mut PgConnection) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT f.id
             FROM negotiation.faq_entries f
             LEFT JOIN LATERAL (
                  SELECT sum(r.count) AS lectures
                    FROM negotiation.faq_reads r
                   WHERE r.entry_id = f.id
                     AND r.day > (now() AT TIME ZONE 'Europe/Paris')::date - 30
             ) l ON true
            WHERE f.status <> 'draft'
              AND (l.lectures IS NOT NULL OR f.editorial_rank IS NOT NULL)
            ORDER BY l.lectures DESC NULLS LAST, f.editorial_rank NULLS LAST, f.id
            LIMIT 3"#
    )
    .fetch_all(conn)
    .await?)
}
