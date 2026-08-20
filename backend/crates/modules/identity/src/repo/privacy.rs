//! Consentements et demandes RGPD.
//!
//! **Aucune de ces lectures ne prend de périmètre**, et ce n'est pas un oubli :
//! une demande d'effacement porte sur la plateforme entière, jamais sur une
//! édition. La borner donnerait l'illusion d'un traitement complet à qui ne voit
//! qu'un morceau de la file — c'est la garde d'autorisation qui refuse, en
//! amont, celui qui n'a pas la portée globale.

use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::admin_users::{
    ConsentView, PrivacyRequestStatus, PrivacyRequestType, PrivacyRequestView,
};
use crate::domain::ids::PersonId;

/// L'état courant des consentements. L'historique complet reste en base — c'est
/// la preuve —, la vue n'en rend que la dernière valeur par finalité.
pub async fn consents(pool: &PgPool, person_id: PersonId) -> Result<Vec<ConsentView>> {
    let lignes = sqlx::query!(
        r#"SELECT purpose        AS "purpose!",
                  is_granted     AS "is_granted!",
                  policy_version AS "policy_version!",
                  recorded_at    AS "recorded_at!"
             FROM identity.current_consents
            WHERE person_id = $1
            ORDER BY purpose"#,
        person_id.as_uuid()
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ConsentView {
            purpose: l.purpose,
            is_granted: l.is_granted,
            policy_version: l.policy_version,
            recorded_at: l.recorded_at,
        })
        .collect())
}

/// Les demandes RGPD d'une personne.
///
/// `is_overdue` ne regarde pas seulement la date : une demande close après son
/// échéance n'est plus en retard, elle est traitée. Confondre les deux ferait
/// clignoter une file déjà vidée.
pub async fn of_person(pool: &PgPool, person_id: PersonId) -> Result<Vec<PrivacyRequestView>> {
    let lignes = sqlx::query!(
        r#"SELECT pr.id,
                  pr.person_id,
                  p.display_name        AS "person_name!",
                  p.primary_email::text AS "person_email!",
                  pr.request_type::text AS "request_type!",
                  pr.status::text       AS "statut!",
                  pr.due_at,
                  ceil(extract(epoch FROM (pr.due_at - now())) / 86400)::int AS "days_left!",
                  (pr.due_at < now() AND pr.completed_at IS NULL) AS "is_overdue!",
                  h.display_name        AS "handled_by_name?",
                  pr.resolution,
                  pr.result_asset_id,
                  pr.created_at,
                  pr.completed_at
             FROM identity.privacy_requests pr
             JOIN identity.people p ON p.id = pr.person_id
             LEFT JOIN identity.people h ON h.id = pr.handled_by
            WHERE pr.person_id = $1
            ORDER BY pr.created_at DESC"#,
        person_id.as_uuid()
    )
    .fetch_all(pool)
    .await?;

    lignes
        .into_iter()
        .map(|l| {
            Ok(PrivacyRequestView {
                id: l.id,
                person_id: PersonId(l.person_id),
                person_name: l.person_name,
                person_email: l.person_email,
                request_type: finalite(&l.request_type)?,
                status: etat_de_demande(&l.statut)?,
                due_at: l.due_at,
                days_left: l.days_left,
                is_overdue: l.is_overdue,
                handled_by_name: l.handled_by_name,
                resolution: l.resolution,
                result_asset_id: l.result_asset_id,
                created_at: l.created_at,
                completed_at: l.completed_at,
            })
        })
        .collect()
}

/// **La file entière**, les plus urgentes d'abord : ce qui est encore ouvert
/// vient en tête, par échéance, puis ce qui est clos, du plus récent au plus
/// ancien.
pub async fn queue(pool: &PgPool) -> Result<Vec<PrivacyRequestView>> {
    let lignes = sqlx::query!(
        r#"SELECT pr.id,
                  pr.person_id,
                  p.display_name        AS "person_name!",
                  p.primary_email::text AS "person_email!",
                  pr.request_type::text AS "request_type!",
                  pr.status::text       AS "statut!",
                  pr.due_at,
                  ceil(extract(epoch FROM (pr.due_at - now())) / 86400)::int AS "days_left!",
                  (pr.due_at < now() AND pr.completed_at IS NULL) AS "is_overdue!",
                  h.display_name        AS "handled_by_name?",
                  pr.resolution,
                  pr.result_asset_id,
                  pr.created_at,
                  pr.completed_at
             FROM identity.privacy_requests pr
             JOIN identity.people p ON p.id = pr.person_id
             LEFT JOIN identity.people h ON h.id = pr.handled_by
            ORDER BY (pr.status IN ('received', 'in_progress')) DESC,
                     pr.due_at,
                     pr.created_at DESC"#
    )
    .fetch_all(pool)
    .await?;

    lignes
        .into_iter()
        .map(|l| {
            Ok(PrivacyRequestView {
                id: l.id,
                person_id: PersonId(l.person_id),
                person_name: l.person_name,
                person_email: l.person_email,
                request_type: finalite(&l.request_type)?,
                status: etat_de_demande(&l.statut)?,
                due_at: l.due_at,
                days_left: l.days_left,
                is_overdue: l.is_overdue,
                handled_by_name: l.handled_by_name,
                resolution: l.resolution,
                result_asset_id: l.result_asset_id,
                created_at: l.created_at,
                completed_at: l.completed_at,
            })
        })
        .collect()
}

/// Le nombre de demandes ouvertes, **sans borne d'édition**.
pub async fn open_count(pool: &PgPool) -> Result<i64> {
    let compte = sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!"
             FROM identity.privacy_requests
            WHERE status IN ('received', 'in_progress')"#
    )
    .fetch_one(pool)
    .await?;

    Ok(compte)
}

/// La demande visée, **verrouillée** : deux traitements simultanés se
/// sérialisent, et le second lit l'état que le premier a posé.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub person_id: PersonId,
    pub request_type: PrivacyRequestType,
    pub status: PrivacyRequestStatus,
}

pub async fn lock(conn: &mut PgConnection, request_id: Uuid) -> Result<Option<Target>> {
    let ligne = sqlx::query!(
        r#"SELECT person_id,
                  request_type::text AS "request_type!",
                  status::text       AS "statut!"
             FROM identity.privacy_requests
            WHERE id = $1
              FOR UPDATE"#,
        request_id
    )
    .fetch_optional(conn)
    .await?;

    ligne
        .map(|l| {
            Ok(Target {
                person_id: PersonId(l.person_id),
                request_type: finalite(&l.request_type)?,
                status: etat_de_demande(&l.statut)?,
            })
        })
        .transpose()
}

/// Pose l'état, son auteur et sa résolution.
///
/// `completed_at` suit l'état plutôt qu'un argument : une demande close porte
/// sa date, une demande reprise la perd. Laisser l'appelant la choisir
/// permettrait une demande « en cours » datée de sa clôture.
pub async fn mark(
    conn: &mut PgConnection,
    request_id: Uuid,
    status: PrivacyRequestStatus,
    handled_by: PersonId,
    resolution: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE identity.privacy_requests
            SET status = $2::text::identity.privacy_request_status,
                handled_by = $3,
                resolution = COALESCE($4, resolution),
                completed_at = CASE
                    WHEN $2 IN ('completed', 'rejected') THEN now()
                    ELSE NULL
                END
          WHERE id = $1",
        request_id,
        status.as_db(),
        handled_by.as_uuid(),
        resolution
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Dépôt d'une demande. **`due_at` n'est pas calculée ici** : la table porte
/// l'échéance réglementaire par sa valeur par défaut, et la recopier en Rust
/// ferait deux vérités dont la seconde se périmerait.
pub async fn submit(
    conn: &mut PgConnection,
    person_id: PersonId,
    request_type: PrivacyRequestType,
) -> Result<(Uuid, time::OffsetDateTime)> {
    let ligne = sqlx::query!(
        "INSERT INTO identity.privacy_requests (person_id, request_type)
         VALUES ($1, $2::text::identity.privacy_request_type)
         RETURNING id, due_at",
        person_id.as_uuid(),
        request_type.as_db()
    )
    .fetch_one(conn)
    .await?;

    Ok((ligne.id, ligne.due_at))
}

/// L'ENUM est fermé en base : une valeur inconnue signale que le code et le
/// modèle ont divergé. Public pour la liste d'utilisateurs, qui lit la demande
/// ouverte de chaque personne dans la même passe que sa ligne.
pub fn finalite(valeur: &str) -> Result<PrivacyRequestType> {
    PrivacyRequestType::from_db(valeur)
        .ok_or_else(|| ApiError::internal(format!("type de demande inconnu : {valeur}")))
}

fn etat_de_demande(valeur: &str) -> Result<PrivacyRequestStatus> {
    PrivacyRequestStatus::from_db(valeur)
        .ok_or_else(|| ApiError::internal(format!("état de demande inconnu : {valeur}")))
}
