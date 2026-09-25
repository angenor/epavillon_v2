//! Émettre les avis d'un signalement, dans la transaction qui le publie ou le
//! refuse. Aucun événement quand personne n'est à prévenir.

use contracts::negotiation::{
    WithNotification, AGGREGATE_NETWORK_MEETING, AGGREGATE_SCHEMA, AGGREGATE_SESSION_REPORT,
    NETWORK_MEETING_PUBLISHED, REPORT_DECIDED, REPORT_PUBLISHED,
};
use kernel::error::Result;
use kernel::events::{emit, DomainEvent};
use serde_json::json;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::reports::ReportReason;
use crate::notifications::avis::{self, Cible, Detail, Etat, Origine, Sujet};
use crate::repo::publication::{self as depot, Tenu};

const LIEN_MES_SIGNALEMENTS: &str = "/guide-nego/negociations/signalements";

fn sujet(t: &Tenu) -> Sujet<'_> {
    match (&t.title_en, &t.what) {
        (Some(en), _) => Sujet {
            fr: t.title_fr.as_deref().unwrap_or(en),
            en,
        },
        (None, Some(quoi)) => Sujet { fr: quoi, en: quoi },
        (None, None) => Sujet { fr: "", en: "" },
    }
}

async fn emettre(
    conn: &mut PgConnection,
    aggregate_type: &str,
    aggregate_id: Uuid,
    event_type: &str,
    notification: contracts::negotiation::Notification,
) -> Result<()> {
    if notification.recipients.is_empty() {
        return Ok(());
    }
    let payload = serde_json::to_value(WithNotification { notification })
        .map_err(|e| kernel::error::ApiError::internal(format!("avis illisible : {e}")))?;
    emit(
        conn,
        DomainEvent {
            aggregate_schema: AGGREGATE_SCHEMA,
            aggregate_type,
            aggregate_id,
            event_type,
            payload,
        },
    )
    .await?;
    Ok(())
}

/// L'encart ou la réunion non annoncée, à qui les suit.
pub async fn publication(conn: &mut PgConnection, t: &Tenu, reunion: Option<Uuid>) -> Result<()> {
    let Some(motif) = ReportReason::from_db(&t.reason) else {
        return Ok(());
    };
    let etat = Etat::du_motif(motif);
    let detail = match motif {
        ReportReason::Time => t.heure.as_deref().map(|heure| Detail::Heure {
            heure,
            ville: t.ville.as_deref(),
        }),
        ReportReason::Venue => t.proposed_venue.as_deref().map(Detail::Salle),
        ReportReason::Unannounced => t.proposed_day.map(|jour| Detail::Reunion {
            jour,
            heure: t.heure.as_deref(),
            lieu: t.proposed_venue.as_deref(),
        }),
        ReportReason::Cancelled | ReportReason::Other => None,
    }
    .unwrap_or(Detail::Aucun);
    let texte = avis::changement(etat, sujet(t), detail, Origine::Reseau);
    let variables = json!({ "reason": t.reason, "report_id": t.id });

    match (reunion, t.meeting_id) {
        (Some(id), _) => {
            let notification = avis::notification(
                NETWORK_MEETING_PUBLISHED,
                depot::destinataires_reunion(conn, id).await?,
                texte,
                Cible {
                    table: "network_meetings",
                    id,
                    lien: format!("/guide-nego/negociations/reseau/{id}"),
                },
                Some(avis::cle_du_jour(NETWORK_MEETING_PUBLISHED, id, t.jour)),
                variables,
            );
            emettre(
                conn,
                AGGREGATE_NETWORK_MEETING,
                id,
                NETWORK_MEETING_PUBLISHED,
                notification,
            )
            .await
        }
        (None, Some(session)) => {
            let notification = avis::notification(
                REPORT_PUBLISHED,
                depot::destinataires_session(conn, session).await?,
                texte,
                Cible {
                    table: "meetings",
                    id: session,
                    lien: format!("/guide-nego/negociations/{session}"),
                },
                Some(avis::cle_du_jour(REPORT_PUBLISHED, session, t.jour)),
                variables,
            );
            emettre(
                conn,
                AGGREGATE_SESSION_REPORT,
                t.id,
                REPORT_PUBLISHED,
                notification,
            )
            .await
        }
        (None, None) => Ok(()),
    }
}

/// Pour l'autrice : publié, ou non retenu avec son motif.
pub async fn decision(conn: &mut PgConnection, t: &Tenu) -> Result<()> {
    let texte = match t.reject_reason.as_deref() {
        Some(motif) => avis::refuse(sujet(t), motif),
        None => avis::publie(sujet(t)),
    };
    let notification = avis::notification(
        REPORT_DECIDED,
        vec![t.author_id],
        texte,
        Cible {
            table: "session_reports",
            id: t.id,
            lien: LIEN_MES_SIGNALEMENTS.to_owned(),
        },
        None,
        json!({ "status": t.status, "reject_reason": t.reject_reason }),
    );
    emettre(
        conn,
        AGGREGATE_SESSION_REPORT,
        t.id,
        REPORT_DECIDED,
        notification,
    )
    .await
}
