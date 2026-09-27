//! Le paquet du savoir, public : tout le publié, ou la différence depuis la
//! dernière lecture du téléphone (R2).

use kernel::error::Result;
use time::OffsetDateTime;

use crate::domain::savoir::{KnowledgeBundle, Pathway, Removed, CHEVAUCHEMENT};
use crate::repo::savoir_paquet as repo;
use crate::state::NegotiationState;

/// L'empreinte du paquet dans cette langue. Elle ne dépend pas de `since` : si
/// rien n'a changé, aucune différence n'est à rendre.
pub async fn empreinte(state: &NegotiationState, locale: &str) -> Result<String> {
    let mut conn = state.pool().acquire().await?;
    let base = repo::empreinte(&mut conn).await?;
    Ok(kernel::empreinte::de(&format!("{base}|{locale}")))
}

/// Lu dans une seule photographie de la base : une entrée publiée pendant la
/// lecture ne peut pas manquer à la FAQ et paraître dans « les plus lues ».
pub async fn paquet(
    state: &NegotiationState,
    locale: &str,
    since: Option<OffsetDateTime>,
) -> Result<(KnowledgeBundle, String)> {
    let served_at = OffsetDateTime::now_utc();
    let depuis = since.map(|s| s - CHEVAUCHEMENT);

    let mut tx = state.pool().begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let base = repo::empreinte(&mut tx).await?;
    let faq_sections = repo::rubriques(&mut tx, locale).await?;
    let glossary_families = repo::familles(&mut tx, locale).await?;
    let faq = repo::faq(&mut tx, locale, depuis).await?;
    let glossary = repo::lexique(&mut tx, locale, depuis).await?;
    let groups = repo::parcours(&mut tx, locale).await?;
    let most_read = repo::plus_lues(&mut tx).await?;
    let removed = match depuis {
        Some(d) => {
            let (faq, glossary) = repo::sorties(&mut tx, d).await?;
            Removed { faq, glossary }
        }
        None => Removed::default(),
    };
    tx.commit().await?;

    Ok((
        KnowledgeBundle {
            served_at,
            complete: since.is_none(),
            faq_sections,
            glossary_families,
            faq,
            glossary,
            pathway: Pathway { groups },
            most_read,
            removed,
        },
        kernel::empreinte::de(&format!("{base}|{locale}")),
    ))
}
