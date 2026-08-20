//! Lectures et écritures de `event.calls_for_proposals` — **un seul appel par
//! édition**, cardinalité tenue par `ux_calls_one_per_event` et non par
//! l'application.
//!
//! **Les trois fonctions du modèle sont appelées, jamais recalculées** :
//! `event.effective_deadline()`, `event.is_call_open()` et
//! `event.max_weighted_score()`. Les réécrire en Rust ferait deux définitions de
//! l'échéance, et c'est la seconde qui finit par se tromper.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::call::EditionCallPayload;
use crate::domain::detail::EditionCall;
use crate::domain::ids::{CallId, EventId};

/// L'appel **non annulé** d'une édition, s'il existe. Zéro ou un, jamais un
/// tableau : les annulés restent à l'historique et sont exclus de l'index.
///
/// L'heure d'ouverture du pavillon est rendue **en texte** par la base
/// (`HH:MM:SS`) : la mettre en forme ici inventerait une seconde écriture de
/// l'heure, et le contrat du front lit une chaîne.
pub async fn de_l_edition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Option<EditionCall>> {
    let ligne = sqlx::query!(
        r#"SELECT c.id, c.event_id, c.code, c.title, c.description,
                  c.status::text AS "status!",
                  c.opens_at, c.closes_at, c.extended_until, c.results_expected_at,
                  c.max_proposals_per_organization, c.requires_verified_organization,
                  c.min_speakers, c.max_speakers,
                  c.default_duration_minutes, c.min_duration_minutes, c.max_duration_minutes,
                  c.daily_start_time::text AS "daily_start_time!",
                  c.daily_end_time::text   AS "daily_end_time!",
                  c.allowed_formats::text[] AS "allowed_formats!",
                  c.required_reviews, c.blind_review,
                  c.guidelines_url::text AS "guidelines_url?",
                  event.effective_deadline(c.id)  AS "effective_deadline!",
                  event.is_call_open(c.id)        AS "is_open!",
                  event.max_weighted_score(c.id)::float8 AS "max_weighted_score!"
             FROM event.calls_for_proposals c
            WHERE c.event_id = $1 AND c.status <> 'cancelled'"#,
        event_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| EditionCall {
        id: l.id,
        event_id: l.event_id,
        code: l.code,
        title: l.title,
        description: l.description,
        status: l.status,
        opens_at: l.opens_at,
        closes_at: l.closes_at,
        extended_until: l.extended_until,
        results_expected_at: l.results_expected_at,
        max_proposals_per_organization: l.max_proposals_per_organization,
        requires_verified_organization: l.requires_verified_organization,
        min_speakers: l.min_speakers,
        max_speakers: l.max_speakers,
        default_duration_minutes: l.default_duration_minutes,
        min_duration_minutes: l.min_duration_minutes,
        max_duration_minutes: l.max_duration_minutes,
        daily_start_time: l.daily_start_time,
        daily_end_time: l.daily_end_time,
        allowed_formats: l.allowed_formats,
        required_reviews: l.required_reviews,
        blind_review: l.blind_review,
        guidelines_url: l.guidelines_url,
        effective_deadline: l.effective_deadline,
        is_open: l.is_open,
        max_weighted_score: l.max_weighted_score,
        // Posés par le service, depuis `repo/cross.rs` et `repo/criteria.rs`.
        proposal_count: 0,
        criteria: Vec::new(),
    }))
}

/// Un appel **par son identifiant**, quel que soit son statut.
///
/// La lecture par édition écarte l'annulé — c'est ce que l'écran veut voir.
/// Après une écriture, il faut au contraire rendre **l'appel qu'on vient
/// d'écrire**, fût-il annulé : sans cela, annuler un appel rendrait `null` et
/// l'écran croirait l'avoir perdu.
pub async fn par_id<'e>(
    executor: impl PgExecutor<'e>,
    call_id: CallId,
) -> Result<Option<EditionCall>> {
    let ligne = sqlx::query!(
        r#"SELECT c.id, c.event_id, c.code, c.title, c.description,
                  c.status::text AS "status!",
                  c.opens_at, c.closes_at, c.extended_until, c.results_expected_at,
                  c.max_proposals_per_organization, c.requires_verified_organization,
                  c.min_speakers, c.max_speakers,
                  c.default_duration_minutes, c.min_duration_minutes, c.max_duration_minutes,
                  c.daily_start_time::text AS "daily_start_time!",
                  c.daily_end_time::text   AS "daily_end_time!",
                  c.allowed_formats::text[] AS "allowed_formats!",
                  c.required_reviews, c.blind_review,
                  c.guidelines_url::text AS "guidelines_url?",
                  event.effective_deadline(c.id)  AS "effective_deadline!",
                  event.is_call_open(c.id)        AS "is_open!",
                  event.max_weighted_score(c.id)::float8 AS "max_weighted_score!"
             FROM event.calls_for_proposals c
            WHERE c.id = $1"#,
        call_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| EditionCall {
        id: l.id,
        event_id: l.event_id,
        code: l.code,
        title: l.title,
        description: l.description,
        status: l.status,
        opens_at: l.opens_at,
        closes_at: l.closes_at,
        extended_until: l.extended_until,
        results_expected_at: l.results_expected_at,
        max_proposals_per_organization: l.max_proposals_per_organization,
        requires_verified_organization: l.requires_verified_organization,
        min_speakers: l.min_speakers,
        max_speakers: l.max_speakers,
        default_duration_minutes: l.default_duration_minutes,
        min_duration_minutes: l.min_duration_minutes,
        max_duration_minutes: l.max_duration_minutes,
        daily_start_time: l.daily_start_time,
        daily_end_time: l.daily_end_time,
        allowed_formats: l.allowed_formats,
        required_reviews: l.required_reviews,
        blind_review: l.blind_review,
        guidelines_url: l.guidelines_url,
        effective_deadline: l.effective_deadline,
        is_open: l.is_open,
        max_weighted_score: l.max_weighted_score,
        proposal_count: 0,
        criteria: Vec::new(),
    }))
}

/// L'état d'un appel **avant** l'écriture, réduit à ce que l'annonce compare :
/// le statut, la clôture et la prolongation.
///
/// Sans lui, on ne saurait dire ni qu'un appel *vient de* s'ouvrir, ni qu'une
/// échéance *a été déplacée* — et une annonce qui ne porte que l'état final
/// oblige chaque consommateur à retenir le précédent.
#[derive(Debug, Clone)]
pub struct EtatAvant {
    pub status: String,
    pub closes_at: OffsetDateTime,
    pub extended_until: Option<OffsetDateTime>,
}

impl EtatAvant {
    /// L'échéance effective d'alors — `COALESCE(extended_until, closes_at)`,
    /// la même expression que `event.effective_deadline()`, appliquée à un état
    /// **passé** que la fonction du modèle ne peut plus lire.
    pub fn echeance(&self) -> OffsetDateTime {
        self.extended_until.unwrap_or(self.closes_at)
    }
}

pub async fn etat_avant(conn: &mut PgConnection, call_id: CallId) -> Result<Option<EtatAvant>> {
    let ligne = sqlx::query!(
        r#"SELECT status::text AS "status!", closes_at, extended_until
             FROM event.calls_for_proposals WHERE id = $1"#,
        call_id.as_uuid()
    )
    .fetch_optional(&mut *conn)
    .await?;

    Ok(ligne.map(|l| EtatAvant {
        status: l.status,
        closes_at: l.closes_at,
        extended_until: l.extended_until,
    }))
}

/// Création. `created_by` vient du **contexte de session**, jamais de la charge
/// utile.
///
/// **L'erreur est rendue BRUTE** : seul le service sait à quel champ du
/// formulaire une contrainte se rapporte, et la traduire ici perdrait le nom de
/// la contrainte — la seule chose sur laquelle on ait le droit de brancher.
///
/// Les heures d'accueil du pavillon sont écrites **telles que le formulaire les
/// compose**, en heure locale de l'édition : `time` n'est pas un instant, il n'y
/// a donc aucun fuseau à appliquer.
pub async fn inserer(
    conn: &mut PgConnection,
    event_id: EventId,
    p: &EditionCallPayload,
    created_by: Uuid,
) -> std::result::Result<CallId, sqlx::Error> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, description, status, opens_at, closes_at,
                extended_until, results_expected_at, max_proposals_per_organization,
                requires_verified_organization, min_speakers, max_speakers,
                default_duration_minutes, min_duration_minutes, max_duration_minutes,
                daily_start_time, daily_end_time, allowed_formats,
                required_reviews, blind_review, guidelines_url, created_by)
           VALUES ($1, $2, $3::jsonb, $4::jsonb, $5::text::event.call_status, $6, $7,
                   $8, $9, $10, $11, $12, $13, $14, $15, $16,
                   $17::text::time, $18::text::time,
                   $19::text[]::event.participation_mode[],
                   $20, $21, $22::text::platform.url, $23)
        RETURNING id"#,
        event_id.as_uuid(),
        p.code,
        p.title,
        p.description,
        p.status,
        p.opens_at,
        p.closes_at,
        p.extended_until,
        p.results_expected_at,
        p.max_proposals_per_organization,
        p.requires_verified_organization,
        p.min_speakers,
        p.max_speakers,
        p.default_duration_minutes,
        p.min_duration_minutes,
        p.max_duration_minutes,
        p.daily_start_time,
        p.daily_end_time,
        &p.allowed_formats,
        p.required_reviews,
        p.blind_review,
        p.guidelines_url,
        created_by
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(CallId::from(id))
}

/// Modification **totale** — même parti qu'une édition (research.md § R13) :
/// tous les champs modifiables sont réécrits, y compris à `NULL`, ce qui permet
/// de retirer une prolongation ou une adresse de consignes.
///
/// `created_by` et `event_id` ne s'y trouvent pas : ni l'auteur ni l'édition
/// d'un appel ne se déplacent.
pub async fn modifier(
    conn: &mut PgConnection,
    call_id: CallId,
    p: &EditionCallPayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.calls_for_proposals SET
               code                           = $2,
               title                          = $3::jsonb,
               description                    = $4::jsonb,
               status                         = $5::text::event.call_status,
               opens_at                       = $6,
               closes_at                      = $7,
               extended_until                 = $8,
               results_expected_at            = $9,
               max_proposals_per_organization = $10,
               requires_verified_organization = $11,
               min_speakers                   = $12,
               max_speakers                   = $13,
               default_duration_minutes       = $14,
               min_duration_minutes           = $15,
               max_duration_minutes           = $16,
               daily_start_time               = $17::text::time,
               daily_end_time                 = $18::text::time,
               allowed_formats                = $19::text[]::event.participation_mode[],
               required_reviews               = $20,
               blind_review                   = $21,
               guidelines_url                 = $22::text::platform.url
         WHERE id = $1"#,
        call_id.as_uuid(),
        p.code,
        p.title,
        p.description,
        p.status,
        p.opens_at,
        p.closes_at,
        p.extended_until,
        p.results_expected_at,
        p.max_proposals_per_organization,
        p.requires_verified_organization,
        p.min_speakers,
        p.max_speakers,
        p.default_duration_minutes,
        p.min_duration_minutes,
        p.max_duration_minutes,
        p.daily_start_time,
        p.daily_end_time,
        &p.allowed_formats,
        p.required_reviews,
        p.blind_review,
        p.guidelines_url
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// L'échéance effective d'un appel, **par la fonction du modèle**. Elle est
/// appelée et jamais recalculée : deux définitions de l'échéance, c'est la
/// seconde qui finit par se tromper.
pub async fn echeance_effective(
    conn: &mut PgConnection,
    call_id: CallId,
) -> Result<OffsetDateTime> {
    let echeance = sqlx::query_scalar!(
        r#"SELECT event.effective_deadline($1) AS "echeance!""#,
        call_id.as_uuid()
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(echeance)
}
