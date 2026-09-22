//! Trancher une demande — **une transaction, ou rien**.
//!
//! # POURQUOI TOUT TIENT DANS UNE SEULE TRANSACTION
//!
//! Admettre, c'est quatre écritures : l'état de la demande, l'attribution du
//! rôle, l'appartenance au réseau si le code de la demande en portait un, et le
//! courriel mis en file. Les séparer ouvrirait la porte à une décision
//! enregistrée sans accès accordé, ou — pire — à un courriel annonçant un accès
//! qui n'existe pas (FR-027, FR-028).
//!
//! L'événement de domaine, lui, part **de la base** :
//! `tg_access_request_event()` écrit `negotiation.access_request.approved` ou
//! `.rejected` dans cette même transaction. L'émettre ici le doublerait.
//!
//! # LA TRANSITION EST REFUSÉE PAR LE TRIGGER, PAS PAR UN `IF`
//!
//! `tg_access_request_transition()` interdit de retrancher un état final. Le
//! service lit l'état **pour répondre avant d'écrire**, et le trigger reste le
//! dernier mot : deux administrateurs qui décident à la même seconde sont
//! sérialisés par le verrou de ligne, et le second reçoit
//! `NEGOTIATION_ACCESS_REQUEST_DECIDED`.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::access::RequestStatus;
use crate::domain::requests::AccessRequestQueue;
use crate::jobs::emails;
use crate::repo::access;
use crate::repo::requests::{self, DemandeATrancher, FiltreDemandes};
use crate::state::NegotiationState;

pub async fn file(
    state: &NegotiationState,
    filtre: &FiltreDemandes<'_>,
    locale: &str,
) -> Result<AccessRequestQueue> {
    let mut conn = state.pool().acquire().await?;
    let (rows, total, pending) = requests::lister(&mut conn, filtre, locale).await?;

    Ok(AccessRequestQueue {
        rows,
        total,
        pending,
    })
}

/// Admet la demande : l'état, l'accès, le réseau, l'annuaire et le courriel.
pub async fn admettre(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    request_id: Uuid,
    motif: Option<&str>,
    locale: &str,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;

    let (demande, etat) = requests::a_trancher(&mut tx, request_id, locale)
        .await?
        .ok_or_else(ApiError::not_found)?;
    exiger_en_attente(etat)?;

    requests::trancher(&mut tx, request_id, RequestStatus::Approved, acteur, motif).await?;

    let attribution = access::accorder(
        &mut tx,
        demande.person_id,
        &demande.scope_type,
        demande.space_id,
        "Demande d'accès admise",
    )
    .await?;

    if let Some(space_id) = demande.space_id {
        access::inscrire_a_lespace(&mut tx, space_id, demande.person_id).await?;
    }

    // Le réseau vient du code que la demande portait, et de rien d'autre
    // (FR-027, SC-006) : aucun champ de personne ne le commande.
    if let (Some(terme), Some(code_id)) = (demande.network_term_id, demande.invitation_code_id) {
        access::rejoindre_le_reseau(&mut tx, demande.person_id, terme, code_id).await?;
    }

    if let Some(attribution) = attribution {
        emettre_lacces(&mut tx, &demande, attribution).await?;
    }

    emails::mettre_en_file_admission(
        &mut tx,
        request_id,
        &demande.email,
        &demande.locale,
        &demande.first_name,
        demande.space_name.as_deref(),
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Refuse la demande. Le motif est facultatif, et repris tel quel dans le
/// courriel : c'est la seule chose que la personne lira pour comprendre.
pub async fn refuser(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    request_id: Uuid,
    motif: Option<&str>,
    locale: &str,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;

    let (demande, etat) = requests::a_trancher(&mut tx, request_id, locale)
        .await?
        .ok_or_else(ApiError::not_found)?;
    exiger_en_attente(etat)?;

    requests::trancher(&mut tx, request_id, RequestStatus::Rejected, acteur, motif).await?;

    emails::mettre_en_file_refus(
        &mut tx,
        request_id,
        &demande.email,
        &demande.locale,
        &demande.first_name,
        motif,
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

fn exiger_en_attente(etat: RequestStatus) -> Result<()> {
    if etat == RequestStatus::Pending {
        Ok(())
    } else {
        Err(ApiError::new(ErrorCode::NegotiationAccessRequestDecided))
    }
}

/// L'accès accordé par une décision se distingue de celui accordé par un code :
/// c'est ce que porte `origin`, et c'est ce qui permettra plus tard d'imputer
/// une entrée à une diffusion ou à un arbitrage.
async fn emettre_lacces(
    conn: &mut sqlx::postgres::PgConnection,
    demande: &DemandeATrancher,
    attribution: Uuid,
) -> Result<()> {
    use contracts::negotiation as evenements;

    let charge = serde_json::to_value(evenements::SpaceAccessGranted {
        person_id: demande.person_id,
        scope_type: demande.scope_type.clone(),
        space_id: demande.space_id,
        origin: evenements::AccessOrigin::AccessRequest,
        invitation_code_id: demande.invitation_code_id,
        network_term_id: demande.network_term_id,
    })
    .map_err(ApiError::internal)?;

    kernel::events::emit(
        conn,
        kernel::events::DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_SPACE_ACCESS,
            aggregate_id: attribution,
            event_type: evenements::SPACE_ACCESS_GRANTED,
            payload: charge,
        },
    )
    .await?;

    Ok(())
}
