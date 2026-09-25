//! Signaler, et relire ses signalements. Rien n'est émis ici : les avis naissent
//! de la publication et du refus (R3, R8).

use kernel::auth::{has_permission, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::permissions::SPACE_ACCESS;
use crate::domain::reports::{MyReport, MyReports, ReportPayload};
use crate::repo::reports::{self, Filtre};
use crate::repo::sessions;
use crate::state::NegotiationState;

pub enum Envoi {
    Cree(MyReport),
    /// La même référence a déjà été reçue : rien n'est écrit une seconde fois.
    Rejoue(MyReport),
}

async fn par_reference(
    conn: &mut sqlx::PgConnection,
    author_id: Uuid,
    client_ref: Uuid,
) -> Result<Option<MyReport>> {
    let filtre = Filtre {
        client_ref: Some(client_ref),
        event_id: None,
    };
    Ok(reports::lister(conn, author_id, filtre)
        .await?
        .into_iter()
        .next())
}

pub async fn signaler(
    state: &NegotiationState,
    ctx: &RequestContext,
    author_id: Uuid,
    charge: &ReportPayload,
) -> Result<Envoi> {
    if !has_permission(state.pool(), author_id, SPACE_ACCESS, Scope::Global).await? {
        return Err(ApiError::new(ErrorCode::NegotiationReportForbidden));
    }

    let mut tx = state.db().write(ctx).await?;
    if let Some(deja) = par_reference(&mut tx, author_id, charge.client_ref).await? {
        tx.rollback().await?;
        return Ok(Envoi::Rejoue(deja));
    }

    let signalement = charge.valider()?;
    let edition = sessions::edition(&mut tx, charge.edition.trim())
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"))?;
    if let Some(session) = signalement.session_id {
        if !reports::session_de_ledition(&mut tx, edition.event_id, session).await? {
            return Err(ApiError::new(ErrorCode::NegotiationSessionUnknown).field("session_id"));
        }
    }
    let theme = match signalement.theme.as_deref() {
        Some(code) => Some(reports::theme(&mut tx, code).await?.ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::NegotiationReportInvalid,
                "Cette thématique n'existe pas.",
            )
            .field("theme")
        })?),
        None => None,
    };

    let insere = reports::inserer(
        &mut tx,
        author_id,
        charge.client_ref,
        edition.event_id,
        &signalement,
        theme,
    )
    .await?;
    let lu = par_reference(&mut tx, author_id, charge.client_ref)
        .await?
        .ok_or_else(|| ApiError::internal("signalement introuvable après écriture"))?;
    tx.commit().await?;
    Ok(match insere {
        Some(_) => Envoi::Cree(lu),
        None => Envoi::Rejoue(lu),
    })
}

pub async fn mes_signalements(
    state: &NegotiationState,
    author_id: Uuid,
    edition: &str,
) -> Result<MyReports> {
    let mut conn = state.pool().acquire().await?;
    let edition = sessions::edition(&mut conn, edition.trim())
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"))?;
    let filtre = Filtre {
        client_ref: None,
        event_id: Some(edition.event_id),
    };
    Ok(MyReports {
        reports: reports::lister(&mut conn, author_id, filtre).await?,
    })
}
