//! Les sources d'une entrée de FAQ ou du lexique, lues et remplacées en bloc.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::admin_savoir::{AdminKnowledgeSource, AdminKnowledgeSourceInput};

/// Le propriétaire d'une source : exactement l'un des deux.
#[derive(Clone, Copy)]
pub enum Proprietaire {
    Faq(Uuid),
    Lexique(Uuid),
}

impl Proprietaire {
    fn colonnes(self) -> (Option<Uuid>, Option<Uuid>) {
        match self {
            Self::Faq(id) => (Some(id), None),
            Self::Lexique(id) => (None, Some(id)),
        }
    }
}

pub async fn lire(
    conn: &mut PgConnection,
    locale: &str,
    proprietaire: Proprietaire,
) -> Result<Vec<AdminKnowledgeSource>> {
    let (faq, lexique) = proprietaire.colonnes();
    let lignes = sqlx::query!(
        r#"SELECT s.document_id, platform.t(d.title, $1) AS document_title,
                  s.external_title, s.external_url::text AS external_url, s.section_label,
                  s.page_from, s.page_to, s.quote
             FROM negotiation.knowledge_sources s
             LEFT JOIN negotiation.documents d ON d.id = s.document_id
            WHERE s.faq_entry_id = $2 OR s.glossary_entry_id = $3
            ORDER BY s.sort_order, s.id"#,
        locale,
        faq,
        lexique
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| AdminKnowledgeSource {
            document_id: l.document_id,
            document_title: l.document_title,
            external_title: l.external_title,
            external_url: l.external_url,
            section_label: l.section_label,
            page_from: l.page_from,
            page_to: l.page_to,
            quote: l.quote,
        })
        .collect())
}

fn vide(texte: &Option<String>) -> Option<&str> {
    texte.as_deref().map(str::trim).filter(|t| !t.is_empty())
}

/// Les contraintes de la table disent ce qui est valide : le refus remonte
/// traduit (`NEGOTIATION_SOURCE_TARGET_INVALID`).
pub async fn remplacer(
    conn: &mut PgConnection,
    proprietaire: Proprietaire,
    sources: &[AdminKnowledgeSourceInput],
) -> Result<()> {
    let (faq, lexique) = proprietaire.colonnes();
    sqlx::query!(
        "DELETE FROM negotiation.knowledge_sources WHERE faq_entry_id = $1 OR glossary_entry_id = $2",
        faq,
        lexique
    )
    .execute(&mut *conn)
    .await?;
    for (rang, s) in sources.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO negotiation.knowledge_sources
                 (faq_entry_id, glossary_entry_id, document_id, external_title, external_url,
                  section_label, page_from, page_to, quote, sort_order)
             VALUES ($1, $2, $3, $4, $5::text::platform.url, $6, $7, $8, $9, $10)",
            faq,
            lexique,
            s.document_id,
            vide(&s.external_title),
            vide(&s.external_url),
            vide(&s.section_label),
            s.page_from,
            s.page_to,
            vide(&s.quote),
            i16::try_from(rang).unwrap_or(i16::MAX)
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}
