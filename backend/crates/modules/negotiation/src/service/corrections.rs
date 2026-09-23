//! Les notes de correction de l'expert : poser, retirer, lister. Chaque
//! écriture passe par `Db::write` et laisse son auteur dans l'audit ; une note
//! ne se supprime jamais, un retrait se date.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::admin_documents::{
    AdminCorrectionNote, AdminCorrectionNoteList, CorrectionNoteInput, PersonLink,
};
use crate::repo::corrections::{self, NoteLue};
use crate::repo::documents;
use crate::state::NegotiationState;

fn vue(n: NoteLue) -> AdminCorrectionNote {
    AdminCorrectionNote {
        id: n.id,
        document_id: n.document_id,
        page_index: n.page_index,
        passage: n.passage,
        body: n.body,
        author: PersonLink {
            id: n.author_id,
            name: n.author_name,
        },
        posted_at: n.created_at,
        withdrawn_at: n.withdrawn_at,
        withdrawn_by: n.withdrawn_by.map(|id| PersonLink {
            id,
            name: n.withdrawn_by_name.unwrap_or_default(),
        }),
    }
}

pub async fn du_document(
    state: &NegotiationState,
    document_id: Uuid,
    peut_poser: bool,
    peut_retirer: bool,
) -> Result<AdminCorrectionNoteList> {
    let mut conn = state.pool().acquire().await?;
    documents::quelconque(&mut conn, document_id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationDocumentNotFound))?;
    let notes = corrections::du_document(&mut conn, document_id).await?;
    Ok(AdminCorrectionNoteList {
        notes: notes.into_iter().map(vue).collect(),
        can_post: peut_poser,
        can_withdraw: peut_retirer,
    })
}

async fn relire(
    state: &NegotiationState,
    document_id: Uuid,
    note_id: Uuid,
) -> Result<AdminCorrectionNote> {
    let mut conn = state.pool().acquire().await?;
    corrections::du_document(&mut conn, document_id)
        .await?
        .into_iter()
        .find(|n| n.id == note_id)
        .map(vue)
        .ok_or_else(ApiError::not_found)
}

/// La page doit exister dans la forme lisible : la base le vérifie, et le refus
/// se traduit en `NEGOTIATION_CORRECTION_PAGE_UNKNOWN`.
pub async fn poser(
    state: &NegotiationState,
    ctx: &RequestContext,
    document_id: Uuid,
    entree: &CorrectionNoteInput,
) -> Result<AdminCorrectionNote> {
    let auteur = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let fr = entree
        .body
        .get("fr")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    // Les clés suivent le domaine `platform.i18n_text` : « fr », « pt-BR ».
    let langue = |k: &str| {
        let b = k.as_bytes();
        (b.len() == 2
            || (b.len() == 5 && b[2] == b'-' && b[3..].iter().all(u8::is_ascii_uppercase)))
            && b[..2].iter().all(u8::is_ascii_lowercase)
    };
    if fr.is_empty()
        || !entree
            .body
            .as_object()
            .is_some_and(|o| o.iter().all(|(k, v)| langue(k) && v.is_string()))
    {
        return Err(ApiError::validation(
            "Le texte de la note en français est obligatoire.",
            "body",
        ));
    }
    let passage = entree
        .passage
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty());
    let mut tx = state.db().write(ctx).await?;
    documents::quelconque(&mut tx, document_id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationDocumentNotFound))?;
    let id = corrections::poser(
        &mut tx,
        document_id,
        entree.page_index,
        passage,
        &entree.body,
        auteur,
    )
    .await?;
    tx.commit().await?;
    relire(state, document_id, id).await
}

/// Idempotent : retirer une note déjà retirée la rend telle qu'elle est.
pub async fn retirer(
    state: &NegotiationState,
    ctx: &RequestContext,
    note_id: Uuid,
) -> Result<AdminCorrectionNote> {
    let par = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let mut tx = state.db().write(ctx).await?;
    let document_id = corrections::retirer(&mut tx, note_id, par)
        .await?
        .ok_or_else(ApiError::not_found)?;
    tx.commit().await?;
    relire(state, document_id, note_id).await
}
