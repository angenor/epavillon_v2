//! La file RGPD et son traitement.
//!
//! **La portée exigée est globale, ou rien** (FR-059). Une demande d'effacement
//! porte sur la plateforme entière : il n'existe aucune édition à laquelle la
//! rapporter, et un administrateur détaché sur une COP ne peut pas décider du
//! sort d'une identité qui sert ailleurs. C'est pourquoi la garde de route est
//! `Requires<PersonManage>` — la portée globale — et non `RequiresAnyScope` :
//! rendre une file filtrée donnerait l'illusion d'un traitement complet.

use contracts::identity as contrats;
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self as evenements, DomainEvent};
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::admin_users::{PrivacyRequestStatus, PrivacyRequestType, PrivacyRequestView};
use crate::domain::ids::PersonId;
use crate::domain::privacy::{
    PrivacyAction, PrivacyQueueScreen, PrivacyWriteOutcome, DEADLINE_DAYS,
};
use crate::repo::privacy;
use crate::state::IdentityState;

pub async fn queue_screen(pool: &PgPool) -> Result<PrivacyQueueScreen> {
    let requests = privacy::queue(pool).await?;

    let open_count = requests
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                PrivacyRequestStatus::Received | PrivacyRequestStatus::InProgress
            )
        })
        .count();
    let overdue_count = requests.iter().filter(|r| r.is_overdue).count();

    Ok(PrivacyQueueScreen {
        requests,
        open_count,
        overdue_count,
        deadline_days: DEADLINE_DAYS,
    })
}

/// Dépôt d'une demande.
///
/// L'écran de profil qui l'appellera n'existe pas dans ce jalon : la route de
/// dépôt arrive avec lui. Le service, lui, est ici — c'est le seul endroit d'où
/// `identity.privacy_request.received` doit partir, et l'écrire ailleurs plus
/// tard produirait deux dépôts qui ne se ressemblent pas.
pub async fn submit(
    state: &IdentityState,
    ctx: &RequestContext,
    acteur: Uuid,
    person_id: PersonId,
    request_type: PrivacyRequestType,
) -> Result<Uuid> {
    let mut tx = state.db().write(&ctx.with_actor(acteur)).await?;

    let (request_id, due_at) = privacy::submit(&mut tx, person_id, request_type).await?;

    evenements::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: contrats::AGGREGATE_SCHEMA,
            aggregate_type: contrats::AGGREGATE_PRIVACY_REQUEST,
            aggregate_id: request_id,
            event_type: contrats::PRIVACY_REQUEST_RECEIVED,
            payload: serde_json::to_value(contrats::PrivacyRequestReceived {
                request_id,
                person_id: person_id.as_uuid(),
                request_type: request_type.as_db().to_owned(),
                due_at,
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;

    tx.commit().await?;
    Ok(request_id)
}

/// Traitement d'une demande : la faire avancer, la clore, la refuser, ou
/// exécuter l'effacement qu'elle réclame.
pub async fn handle(
    state: &IdentityState,
    ctx: &RequestContext,
    acteur: Uuid,
    request_id: Uuid,
    action: PrivacyAction,
    resolution: Option<&str>,
) -> Result<PrivacyWriteOutcome> {
    let pool = state.pool();
    let mut tx = state.db().write(&ctx.with_actor(acteur)).await?;

    let Some(cible) = privacy::lock(&mut tx, request_id).await? else {
        tx.rollback().await?;
        return Ok(PrivacyWriteOutcome::not_found(privacy::queue(pool).await?));
    };

    // **L'anonymisation ne répond qu'à une demande d'effacement** (FR-060).
    // Le contrôle vient avant l'écriture : un export anonymisé détruirait une
    // identité que personne n'a demandé d'effacer, et rien ne la rendrait.
    if action == PrivacyAction::Anonymize && cible.request_type != PrivacyRequestType::Erasure {
        tx.rollback().await?;
        let file = privacy::queue(pool).await?;
        let demande = trouver(&file, request_id)?;
        return Ok(PrivacyWriteOutcome::wrong_type(demande, file));
    }

    if action == PrivacyAction::Anonymize {
        effacer(&mut tx, cible.person_id, resolution).await?;
    }

    privacy::mark(
        &mut tx,
        request_id,
        etat_apres(action),
        PersonId(acteur),
        resolution,
    )
    .await?;

    tx.commit().await?;

    let file = privacy::queue(pool).await?;
    let demande = trouver(&file, request_id)?;

    Ok(match action {
        PrivacyAction::Anonymize => PrivacyWriteOutcome::anonymized(demande, file),
        _ => PrivacyWriteOutcome::saved(demande, file),
    })
}

/// L'effacement lui-même (FR-061).
///
/// **Aucun événement n'est émis ici, et c'est délibéré** :
/// `identity.anonymize_person()` appelle elle-même `platform.emit_event()` pour
/// `identity.person.anonymized`. En émettre un second passerait sans erreur —
/// deux lignes s'écriraient, et un consommateur idempotent traiterait la
/// première puis ignorerait la mauvaise. Le défaut ne se verrait qu'en relisant
/// l'outbox six mois plus tard, sur un agrégat qui aurait deux fois la même
/// histoire.
///
/// Ce que la fonction de base fait, et que le service ne refait pas : purger
/// l'identité, supprimer les adresses et les comptes, révoquer les sessions —
/// **en conservant l'identifiant technique**, donc les compteurs d'inscriptions
/// et les moyennes de notation des COP passées.
async fn effacer(conn: &mut PgConnection, person_id: PersonId, motif: Option<&str>) -> Result<()> {
    sqlx::query!(
        "SELECT identity.anonymize_person($1, $2)",
        person_id.as_uuid(),
        motif
    )
    .execute(conn)
    .await?;

    Ok(())
}

fn etat_apres(action: PrivacyAction) -> PrivacyRequestStatus {
    match action {
        PrivacyAction::Start => PrivacyRequestStatus::InProgress,
        // L'effacement exécuté clôt la demande : il n'y a plus rien à faire.
        PrivacyAction::Complete | PrivacyAction::Anonymize => PrivacyRequestStatus::Completed,
        PrivacyAction::Reject => PrivacyRequestStatus::Rejected,
    }
}

fn trouver(file: &[PrivacyRequestView], request_id: Uuid) -> Result<PrivacyRequestView> {
    file.iter()
        .find(|r| r.id == request_id)
        .cloned()
        .ok_or_else(|| ApiError::internal("demande introuvable après écriture"))
}
