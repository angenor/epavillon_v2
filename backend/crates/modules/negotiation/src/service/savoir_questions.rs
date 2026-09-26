//! Les questions aux experts : les poser (accès négociateur), y répondre et les
//! promouvoir en FAQ (l'expert). Le courriel de la réponse est mis en file dans
//! la transaction de la réponse (R11) ; le refus d'une promotion sans
//! consentement vient de la base (`ck_expert_questions_promotion`).

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::admin_savoir::AdminFaqEntry;
use crate::domain::savoir_questions::{
    question_valide, reponse_valide, AdminQuestion, AdminQuestionAnswerInput,
    AdminQuestionPromoteInput, MyQuestion, MyQuestionInput, MyQuestionList,
};
use crate::jobs::emails;
use crate::repo::admin_import::theme_actif;
use crate::repo::savoir_faq as faq;
use crate::repo::savoir_questions as repo;
use crate::service::savoir_admin::{fiche_faq, Droits};
use crate::state::NegotiationState;

/// Rejouée avec le même `client_ref` : la question d'origine, et `false`.
pub async fn poser(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    entree: &MyQuestionInput,
    locale: &str,
) -> Result<(MyQuestion, bool)> {
    let mut tx = state.db().write(ctx).await?;
    if let Some(id) = repo::par_reference(&mut tx, personne, entree.client_ref).await? {
        return Ok((une(&mut tx, personne, id, locale).await?, false));
    }
    let body = question_valide(entree)?;
    let theme = theme_actif(&mut tx, &entree.theme_code)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationThemeUnknown).field("theme_code"))?;
    let (id, nouvelle) = match repo::inserer(
        &mut tx,
        personne,
        entree.client_ref,
        theme,
        &body,
        entree.consent_to_faq,
    )
    .await?
    {
        Some(id) => (id, true),
        // Un envoi jumeau est passé entre la lecture et l'écriture.
        None => (
            repo::par_reference(&mut tx, personne, entree.client_ref)
                .await?
                .ok_or_else(|| ApiError::internal("question jumelle introuvable"))?,
            false,
        ),
    };
    let question = une(&mut tx, personne, id, locale).await?;
    tx.commit().await?;
    Ok((question, nouvelle))
}

async fn une(
    conn: &mut sqlx::PgConnection,
    personne: Uuid,
    id: Uuid,
    locale: &str,
) -> Result<MyQuestion> {
    repo::miennes(conn, personne, Some(id), locale)
        .await?
        .pop()
        .ok_or_else(|| ApiError::internal("question posée introuvable"))
}

pub async fn miennes(
    state: &NegotiationState,
    personne: Uuid,
    locale: &str,
) -> Result<(MyQuestionList, String)> {
    let mut conn = state.pool().acquire().await?;
    let questions = repo::miennes(&mut conn, personne, None, locale).await?;
    let empreinte = kernel::empreinte::de(&format!(
        "{locale}\n{}",
        questions
            .iter()
            .map(|q| format!("{}:{}:{:?}", q.id, q.status, q.answered_at))
            .collect::<Vec<_>>()
            .join("\n")
    ));
    Ok((MyQuestionList { questions }, empreinte))
}

pub async fn file(state: &NegotiationState, locale: &str) -> Result<Vec<AdminQuestion>> {
    let mut conn = state.pool().acquire().await?;
    repo::file(&mut conn, None, locale).await
}

async fn relue(
    conn: &mut sqlx::PgConnection,
    id: Uuid,
    locale: &str,
) -> Result<AdminQuestion> {
    repo::file(conn, Some(id), locale)
        .await?
        .pop()
        .ok_or_else(ApiError::not_found)
}

pub async fn repondre(
    state: &NegotiationState,
    ctx: &RequestContext,
    expert: Uuid,
    id: Uuid,
    entree: &AdminQuestionAnswerInput,
    locale: &str,
) -> Result<AdminQuestion> {
    let answer = reponse_valide(entree)?;
    let mut tx = state.db().write(ctx).await?;
    let q = repo::a_trancher(&mut tx, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if q.status != "pending" {
        return Err(ApiError::new(ErrorCode::NegotiationQueueItemClosed));
    }
    repo::repondre(&mut tx, id, &answer, expert).await?;
    emails::mettre_en_file_reponse(&mut tx, id, &q.email, &q.locale, &q.first_name, &q.body)
        .await?;
    let question = relue(&mut tx, id, locale).await?;
    tx.commit().await?;
    Ok(question)
}

/// Une question répondue seulement : sa réponse fait celle du brouillon.
pub async fn promouvoir(
    state: &NegotiationState,
    ctx: &RequestContext,
    droits: &Droits,
    id: Uuid,
    entree: &AdminQuestionPromoteInput,
    locale: &str,
) -> Result<AdminFaqEntry> {
    let mut tx = state.db().write(ctx).await?;
    let q = repo::a_trancher(&mut tx, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let answer = match (q.status.as_str(), q.answer) {
        ("answered", Some(a)) => a,
        ("pending", _) => {
            return Err(ApiError::validation(
                "Répondez d'abord à la question : sa réponse fait celle de l'entrée.",
                "answer",
            ))
        }
        _ => return Err(ApiError::new(ErrorCode::NegotiationQueueItemClosed)),
    };
    let section = faq::rubrique_id(&mut tx, &entree.section_code)
        .await?
        .ok_or_else(|| ApiError::validation("Cette rubrique n'existe pas.", "section_code"))?;
    let entree_id = repo::promouvoir(&mut tx, id, section, &q.body, &answer).await?;
    tx.commit().await?;
    fiche_faq(state, droits, entree_id, locale).await
}
