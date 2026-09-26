//! La lecture des réunions de la Francophonie d'une édition, et la traduction
//! des refus que la base porte sur elles.

use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::meetings::{FrancophoneMeeting, NatureDeReunion, StatutDeReunion};

pub struct Edition {
    pub event_id: Uuid,
    pub slug: String,
    pub timezone: String,
    pub city: Option<String>,
}

pub async fn edition(conn: &mut PgConnection, slug: &str) -> Result<Option<Edition>> {
    let ligne = sqlx::query!(
        r#"SELECT e.id AS "event_id!", e.slug::text AS "slug!", e.timezone::text AS "timezone!",
                  e.city
             FROM event.events e
            WHERE e.slug::text = $1"#,
        slug
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Edition {
        event_id: l.event_id,
        slug: l.slug,
        timezone: l.timezone,
        city: l.city,
    }))
}

/// Les réunions publiées de l'édition — un brouillon jamais —, par début.
pub async fn publiees(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<FrancophoneMeeting>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id, t.code AS type_code, t.label::jsonb AS "type_label!: Value",
                  m.title::jsonb AS "title!: Value", m.description::jsonb AS "description?: Value",
                  m.start_at, m.end_at AS "end_at!", m.format::text AS "format!", m.venue_label,
                  (m.external_url IS NOT NULL) AS "has_video!",
                  m.is_ifdd_organized, o.legal_name AS "organizer_name?",
                  coalesce(m.is_open_access, true) AS "open_access!",
                  m.access_audience::jsonb AS "access_audience?: Value",
                  m.requires_registration, m.capacity, m.registered_count, m.waitlist_enabled,
                  m.registration_opens_at, m.registration_closes_at,
                  (m.status = 'cancelled') AS "annulee!",
                  m.cancellation_reason, m.pavilion_session_id
             FROM negotiation.meetings m
             JOIN reference.taxonomy_terms t ON t.id = m.francophone_type_term_id
             LEFT JOIN org.organizations o ON o.id = org.resolve_organization(m.organizer_org_id)
            WHERE m.event_id = $1
              AND m.kind IN ('preparatory_workshop', 'francophone_consultation')
              AND m.source_key IS NULL
              AND m.status <> 'draft'
            ORDER BY m.start_at, m.id"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| FrancophoneMeeting {
            id: l.id,
            meeting_type: NatureDeReunion {
                code: l.type_code,
                label: l.type_label,
            },
            title: l.title,
            description: l.description,
            start_at: l.start_at,
            end_at: l.end_at,
            format: l.format,
            venue: l.venue_label,
            has_video: l.has_video,
            organizer: if l.is_ifdd_organized {
                "IFDD".to_owned()
            } else {
                l.organizer_name.unwrap_or_default()
            },
            open_access: l.open_access,
            access_audience: l.access_audience,
            requires_registration: l.requires_registration,
            capacity: l.capacity,
            registered_count: l.registered_count,
            waitlist_enabled: l.waitlist_enabled,
            registration_opens_at: l.registration_opens_at,
            registration_closes_at: l.registration_closes_at,
            status: if l.annulee {
                StatutDeReunion::Cancelled
            } else {
                StatutDeReunion::Scheduled
            },
            cancellation_reason: l.cancellation_reason,
            pavilion_session_id: l.pavilion_session_id,
        })
        .collect())
}

/// `(SQLSTATE, contrainte)` → code stable. Le message de la base commence par
/// son code : il ne s'affiche jamais tel quel.
pub fn refus(erreur: sqlx::Error) -> ApiError {
    let etat = kernel::pg_error::sqlstate(&erreur);
    let code = match (etat.as_deref(), kernel::pg_error::constraint(&erreur)) {
        (Some("23001"), Some("meeting_unavailable")) => ErrorCode::NegotiationMeetingUnavailable,
        (Some("23001"), Some("meeting_closed")) => ErrorCode::NegotiationMeetingClosed,
        (Some("23001"), Some("meeting_full")) => ErrorCode::NegotiationMeetingFull,
        (Some("23514" | "23503"), Some(c)) => match champ_de(c) {
            Some((champ, message)) => return invalide(champ, message),
            None => return erreur.into(),
        },
        _ => return erreur.into(),
    };
    ApiError::new(code)
}

/// `400 NEGOTIATION_MEETING_INVALID` qui nomme le champ.
pub fn invalide(champ: &str, message: &str) -> ApiError {
    ApiError::with_message(ErrorCode::NegotiationMeetingInvalid, message).field(champ)
}

/// La contrainte de la base → le champ du formulaire, et ce qu'on en dit.
fn champ_de(contrainte: &str) -> Option<(&'static str, &'static str)> {
    Some(match contrainte {
        "ck_meetings_period" | "ck_meetings_imported_end" => {
            ("end_at", "La fin de la réunion doit suivre son début.")
        }
        "ck_meetings_registration_window" => (
            "registration_closes_at",
            "La fermeture des inscriptions doit suivre leur ouverture.",
        ),
        "ck_meetings_online_access" => (
            "external_url",
            "Une réunion en ligne ou hybride demande un lien de connexion.",
        ),
        "ck_meetings_onsite_venue" => {
            ("venue", "Une réunion sur place ou hybride demande un lieu.")
        }
        "ck_meetings_cancellation" => ("reason", "L'annulation demande un motif."),
        "ck_meetings_francophone_type" | "ck_meetings_francophone_kind" => {
            ("type", "La nature de la réunion est requise.")
        }
        "ck_meetings_francophone_event" => ("edition", "La réunion doit appartenir à une édition."),
        "ck_meetings_access_audience" => (
            "access_audience",
            "Une réunion à accès limité dit à qui elle est ouverte.",
        ),
        "ck_meetings_pavilion_edition" | "xmod_fk_negotiation_meetings_pavilion_session" => (
            "pavilion_session_id",
            "Cette activité n'appartient pas au Pavillon de l'édition de la réunion.",
        ),
        "meetings_capacity_check" => ("capacity", "La capacité doit être d'au moins une place."),
        "url_check" => (
            "external_url",
            "Le lien de connexion doit commencer par http:// ou https://.",
        ),
        "xmod_fk_negotiation_meetings_organizer" => {
            ("organizer_org_id", "Cette organisation n'existe pas.")
        }
        _ => return None,
    })
}
