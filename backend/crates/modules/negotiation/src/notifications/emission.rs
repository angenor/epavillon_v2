//! Émettre les avis d'un signalement, dans la transaction qui le publie ou le
//! refuse. Aucun événement quand personne n'est à prévenir.

use contracts::negotiation::{
    WithNotification, AGGREGATE_MEETING, AGGREGATE_NETWORK_MEETING, AGGREGATE_SCHEMA,
    AGGREGATE_SESSION_REPORT, MEETING_CHANGED, NETWORK_MEETING_PUBLISHED, REPORT_DECIDED,
    REPORT_PUBLISHED,
};
use kernel::error::Result;
use kernel::events::{emit, DomainEvent};
use serde_json::json;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::reports::ReportReason;
use crate::import::comparaison::Changement;
use crate::jobs::change_email::Cible as CibleCourriel;
use crate::notifications::avis::{self, Cible, Detail, Etat, Origine, Sujet};
use crate::repo::courriel;
use crate::repo::publication::{self as depot, Tenu};

/// Qui prévenir par courriel, et de quoi : ceux-là mêmes qui ont reçu l'avis.
pub struct APrevenir {
    pub cible: CibleCourriel,
    pub id: Uuid,
    pub destinataires: Vec<Uuid>,
}

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
pub async fn publication(
    conn: &mut PgConnection,
    t: &Tenu,
    reunion: Option<Uuid>,
) -> Result<Option<APrevenir>> {
    let Some(motif) = ReportReason::from_db(&t.reason) else {
        return Ok(None);
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
            let destinataires = depot::destinataires_reunion(conn, id).await?;
            let notification = avis::notification(
                NETWORK_MEETING_PUBLISHED,
                destinataires.clone(),
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
            .await?;
            Ok(Some(APrevenir {
                cible: CibleCourriel::NetworkMeeting,
                id,
                destinataires,
            }))
        }
        (None, Some(session)) => {
            let destinataires = depot::destinataires_session(conn, session).await?;
            let notification = avis::notification(
                REPORT_PUBLISHED,
                destinataires.clone(),
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
            .await?;
            Ok(Some(APrevenir {
                cible: CibleCourriel::Meeting,
                id: session,
                destinataires,
            }))
        }
        (None, None) => Ok(None),
    }
}

/// Ce qu'un changement lu à la source dit à qui suit la session : l'annulation
/// d'abord, puis l'heure, puis la salle. `None` : rien qui prévienne.
pub fn etat_importe(changements: &[Changement]) -> Option<Etat> {
    let a = |champ: &str| changements.iter().any(|c| c.champ == champ);
    let annulee = changements
        .iter()
        .any(|c| c.champ == "status" && c.apres["status"] == "cancelled");
    if annulee {
        Some(Etat::Annulee)
    } else if a("start") {
        Some(Etat::Deplacee)
    } else if a("venue") {
        Some(Etat::SalleChangee)
    } else {
        None
    }
}

/// `negotiation.meeting.changed`, dans la transaction de la lecture. La
/// session est relue après écriture : l'avis dit la nouvelle valeur.
pub async fn changement_importe(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    etat: Etat,
) -> Result<Option<APrevenir>> {
    let Some(s) = courriel::session_pour_avis(conn, meeting_id).await? else {
        return Ok(None);
    };
    let destinataires = depot::destinataires_session(conn, meeting_id).await?;
    if destinataires.is_empty() {
        return Ok(None);
    }
    let detail = match etat {
        Etat::Deplacee => Detail::Heure {
            heure: &s.heure,
            ville: s.ville.as_deref(),
        },
        Etat::SalleChangee => s.salle.as_deref().map_or(Detail::Aucun, Detail::Salle),
        _ => Detail::Aucun,
    };
    let sujet = Sujet {
        fr: s.title_fr.as_deref().unwrap_or(&s.title_en),
        en: &s.title_en,
    };
    let texte = avis::changement(etat, sujet, detail, Origine::Source);
    let notification = avis::notification(
        MEETING_CHANGED,
        destinataires.clone(),
        texte,
        Cible {
            table: "meetings",
            id: meeting_id,
            lien: CibleCourriel::Meeting.chemin(meeting_id),
        },
        Some(avis::cle_du_jour(MEETING_CHANGED, meeting_id, s.jour)),
        json!({ "change": format!("{etat:?}").to_lowercase() }),
    );
    emettre(
        conn,
        AGGREGATE_MEETING,
        meeting_id,
        MEETING_CHANGED,
        notification,
    )
    .await?;
    Ok(Some(APrevenir {
        cible: CibleCourriel::Meeting,
        id: meeting_id,
        destinataires,
    }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn c(champ: &'static str, apres: serde_json::Value) -> Changement {
        Changement {
            champ,
            avant: json!(null),
            apres,
        }
    }

    #[test]
    fn lannulation_prime_puis_lheure_puis_la_salle() {
        let annulee = c(
            "status",
            json!({ "status": "cancelled", "reason": "removed" }),
        );
        let reparue = c("status", json!({ "status": "scheduled" }));
        let heure = c("start", json!("2026-11-10T11:00:00Z"));
        let salle = c("venue", json!("Salle 3"));
        assert_eq!(
            etat_importe(&[salle.clone(), heure.clone(), annulee]),
            Some(Etat::Annulee)
        );
        assert_eq!(etat_importe(&[salle.clone(), heure]), Some(Etat::Deplacee));
        assert_eq!(etat_importe(&[salle]), Some(Etat::SalleChangee));
        assert_eq!(etat_importe(&[reparue, c("title", json!("x"))]), None);
    }
}
