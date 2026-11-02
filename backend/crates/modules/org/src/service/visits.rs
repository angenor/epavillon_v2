//! Les visites de l'espace d'une organisation : noter l'ouverture d'une page,
//! relire l'onglet « Visites » de la fiche du back-office.

use kernel::auth::AdminScope;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::ids::OrganizationId;
use crate::domain::visits::{OrganizationVisits, SpaceVisit, VisitEvent, VisitMember};
use crate::repo::visits;
use crate::service::admin_detail::dans_le_perimetre;
use crate::state::OrgState;

/// Une page de l'espace s'est ouverte. Rien n'est noté pour qui n'est pas
/// membre actif : la réponse est la même, la page n'a pas à le savoir.
pub async fn noter(
    state: &OrgState,
    ctx: &RequestContext,
    personne: Uuid,
    visite: SpaceVisit,
) -> Result<()> {
    if visite.organization_id.is_none() == visite.proposal_id.is_none() {
        return Err(ApiError::validation(
            "Une visite porte sur une organisation ou sur un dossier, pas les deux.",
            "organization_id",
        ));
    }

    let mut tx = state.db().write(ctx).await?;
    visits::noter(&mut tx, personne, visite.organization_id, visite.proposal_id).await?;
    tx.commit().await
}

/// L'onglet entier, ou `None` hors périmètre (indiscernable d'une fiche
/// inexistante, comme la fiche elle-même). `jours` à 0 : depuis le début.
pub async fn lire(
    pool: &PgPool,
    perimetre: &AdminScope,
    id: OrganizationId,
    jours: u16,
) -> Result<Option<OrganizationVisits>> {
    if !dans_le_perimetre(pool, perimetre, id).await? {
        return Ok(None);
    }

    let fenetre =
        (jours > 0).then(|| OffsetDateTime::now_utc() - Duration::days(i64::from(jours)));
    let organisation = id.as_uuid();

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *conn)
        .await?;

    let recording_since = visits::depuis(&mut *conn).await?;
    let membres = visits::membres(&mut conn, organisation, fenetre).await?;
    let personnes: Vec<Uuid> = membres.iter().map(|m| m.person_id).collect();
    let connexions = visits::connexions(&mut conn, &personnes, fenetre).await?;
    let pages = visits::visites(&mut conn, organisation, fenetre).await?;
    let modifications = visits::modifications(&mut conn, &personnes, fenetre).await?;
    let refus = visits::refus(&mut conn, &personnes, fenetre).await?;

    sqlx::query("COMMIT").execute(&mut *conn).await?;

    let mut events: Vec<VisitEvent> = Vec::new();
    events.extend(connexions.into_iter().map(|c| VisitEvent {
        device: c.agent.as_deref().map(appareil),
        until: (c.fin > c.debut).then_some(c.fin),
        ..evenement("connection", c.person_id, c.debut)
    }));
    events.extend(pages.into_iter().map(|v| VisitEvent {
        until: (v.last_at > v.first_at).then_some(v.last_at),
        count: Some(v.open_count),
        proposal_id: v.proposal_id,
        proposal_reference: v.reference,
        proposal_title: v.title,
        ..evenement("visit", v.person_id, v.first_at)
    }));
    events.extend(modifications.into_iter().map(|m| VisitEvent {
        proposal_id: m.proposal_id,
        proposal_reference: m.reference,
        proposal_title: m.title,
        tables: m.tables,
        request_id: m.request_id,
        ..evenement("change", m.person_id, m.at)
    }));
    events.extend(refus.into_iter().map(|r| VisitEvent {
        message: message_du_refus(r.error_code.as_deref(), r.status),
        route: Some(r.route),
        status: Some(r.status),
        request_id: r.request_id,
        ..evenement("refusal", r.person_id, r.at)
    }));
    events.sort_by_key(|e| std::cmp::Reverse(e.at));

    Ok(Some(OrganizationVisits {
        recording_since,
        window_start: fenetre,
        members: membres
            .into_iter()
            .map(|m| VisitMember {
                person_id: m.person_id,
                display_name: m.display_name,
                role: m.role,
                active: m.active,
                last_connection_at: m.last_connection_at,
                last_visit_at: m.last_visit_at,
                last_visit_reference: m.last_visit_reference,
                visit_days: m.visit_days,
                changes: m.changes,
                last_change_at: m.last_change_at,
            })
            .collect(),
        events,
    }))
}

fn evenement(kind: &str, person_id: Uuid, at: OffsetDateTime) -> VisitEvent {
    VisitEvent {
        kind: kind.to_owned(),
        person_id,
        at,
        until: None,
        count: None,
        proposal_id: None,
        proposal_reference: None,
        proposal_title: None,
        device: None,
        tables: Vec::new(),
        route: None,
        status: None,
        message: None,
        request_id: None,
    }
}

/// Le message que la personne a lu : celui du catalogue pour le code journalisé,
/// à défaut celui du statut.
fn message_du_refus(code: Option<&str>, statut: i16) -> Option<String> {
    let code = code.and_then(ErrorCode::from_code).or(match statut {
        401 => Some(ErrorCode::Unauthenticated),
        403 => Some(ErrorCode::Forbidden),
        404 => Some(ErrorCode::NotFound),
        500..=599 => Some(ErrorCode::Internal),
        _ => None,
    });
    code.map(|c| c.message().to_owned())
}

/// « Chrome, ordinateur » : de quoi reconnaître un appareil, sans garder
/// l'agent utilisateur brut à l'écran.
fn appareil(agent: &str) -> String {
    let navigateur = if agent.contains("Edg/") {
        "Edge"
    } else if agent.contains("OPR/") || agent.contains("Opera") {
        "Opera"
    } else if agent.contains("Firefox/") {
        "Firefox"
    } else if agent.contains("Chrome/") || agent.contains("CriOS/") {
        "Chrome"
    } else if agent.contains("Safari/") {
        "Safari"
    } else {
        "Navigateur inconnu"
    };
    let support = if agent.contains("iPad") || agent.contains("Tablet") {
        "tablette"
    } else if agent.contains("Mobile") || agent.contains("Android") || agent.contains("iPhone") {
        "téléphone"
    } else {
        "ordinateur"
    };
    format!("{navigateur}, {support}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_appareil_se_lit_dans_l_agent() {
        assert_eq!(
            appareil("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36"),
            "Chrome, ordinateur"
        );
        assert_eq!(
            appareil("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1"),
            "Safari, téléphone"
        );
    }

    #[test]
    fn un_refus_rend_le_message_du_catalogue() {
        assert_eq!(
            message_du_refus(Some("FORBIDDEN"), 403).as_deref(),
            Some(ErrorCode::Forbidden.message())
        );
        assert_eq!(
            message_du_refus(None, 404).as_deref(),
            Some(ErrorCode::NotFound.message())
        );
        assert_eq!(message_du_refus(None, 409), None);
    }
}
