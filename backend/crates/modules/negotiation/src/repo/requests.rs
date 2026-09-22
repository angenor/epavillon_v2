//! Les demandes d'accès. **Une seule en attente par personne et par portée**,
//! et c'est la base qui le tient : `ux_access_requests_pending_space` et
//! `ux_access_requests_pending_global`.
//!
//! Ce fichier ne porte à cette étape que ce dont le code d'invitation a besoin —
//! ouvrir une demande quand le mode l'exige. La file du back-office et les deux
//! décisions arrivent avec le récit qui les demande.

use kernel::auth::ScopeType;
use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::access::{AccessRequestView, AccessScopeView, RequestStatus};
use crate::domain::requests::AccessRequestRow;

/// La demande en attente pour cette portée, s'il y en a une.
///
/// **Ce n'est pas une vérification préalable d'invariant** : l'unicité reste
/// celle des deux index partiels, et deux envois simultanés sortent en conflit
/// traduit. Cette lecture sert à rendre la saisie d'un code **idempotente** —
/// la même personne qui ressaisit son code en mode « approbation » doit
/// retrouver sa demande, pas un refus.
pub async fn en_attente(
    conn: &mut PgConnection,
    person_id: Uuid,
    scope_type: &str,
    scope_id: Option<Uuid>,
) -> Result<Option<AccessRequestView>> {
    let ligne = sqlx::query!(
        r#"SELECT r.id           AS "id!",
                  r.submitted_at AS "submitted_at!"
             FROM negotiation.access_requests r
            WHERE r.person_id = $1
              AND r.status = 'pending'
              AND r.scope_type = $2::text::identity.scope_type
              AND r.space_id IS NOT DISTINCT FROM $3"#,
        person_id,
        scope_type,
        scope_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| AccessRequestView {
        id: l.id,
        status: RequestStatus::Pending,
        submitted_at: l.submitted_at,
        decided_at: None,
        decision_reason: None,
    }))
}

/// Ouvre une demande. `invitation_code_id` porte le code reconnu mais
/// insuffisant à ouvrir — c'est ce que l'administrateur lit pour trancher.
///
/// L'événement `negotiation.access_request.submitted` part **de la base**, dans
/// cette transaction : `tg_access_request_event()` s'en charge, et l'émettre
/// ici le doublerait.
pub async fn ouvrir(
    conn: &mut PgConnection,
    person_id: Uuid,
    scope_type: &str,
    scope_id: Option<Uuid>,
    invitation_code_id: Option<Uuid>,
    message: Option<&str>,
) -> Result<AccessRequestView> {
    let ligne = sqlx::query!(
        r#"INSERT INTO negotiation.access_requests
               (person_id, scope_type, space_id, invitation_code_id, message)
           VALUES ($1, $2::text::identity.scope_type, $3, $4, $5)
           RETURNING id, submitted_at"#,
        person_id,
        scope_type,
        scope_id,
        invitation_code_id,
        message
    )
    .fetch_one(conn)
    .await?;

    Ok(AccessRequestView {
        id: ligne.id,
        status: RequestStatus::Pending,
        submitted_at: ligne.submitted_at,
        decided_at: None,
        decision_reason: None,
    })
}

// ---------------------------------------------------------------------------
// Le back-office — la file, et les deux décisions
// ---------------------------------------------------------------------------

/// Ce que le filtre d'URL de la file porte.
#[derive(Debug, Clone, Default)]
pub struct FiltreDemandes<'a> {
    /// `pending`, `approved`, `rejected` ou `cancelled`. Absent : toutes.
    pub etat: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

/// La file, les plus anciennes demandes en attente d'abord — une file se traite
/// dans l'ordre où elle s'est formée.
pub async fn lister(
    conn: &mut PgConnection,
    filtre: &FiltreDemandes<'_>,
    locale: &str,
) -> Result<(Vec<AccessRequestRow>, i64, i64)> {
    let lignes = sqlx::query!(
        r#"SELECT r.id                            AS "id!",
                  r.status::text                  AS "status!",
                  r.person_id                     AS "person_id!",
                  p.display_name                  AS "display_name!",
                  p.primary_email::text           AS "email!",
                  platform.t(pays.name, $4)       AS country,
                  r.scope_type::text              AS "scope_type!",
                  r.space_id,
                  platform.t(s.name, $4)          AS space_name,
                  -- Le « ? » force la nullabilité : la jointure est externe,
                  -- mais ces deux colonnes sont NOT NULL dans leur table et
                  -- sqlx en conclut que la colonne jointe l'est aussi.
                  c.code                          AS "invitation_code?",
                  c.label                         AS "invitation_code_label?",
                  r.message,
                  r.submitted_at                  AS "submitted_at!",
                  r.decided_at,
                  d.display_name                  AS decided_by_name,
                  r.decision_reason,
                  count(*) OVER ()                AS "total!",
                  (SELECT count(*) FROM negotiation.access_requests q
                    WHERE q.status = 'pending')   AS "pending!"
             FROM negotiation.access_requests r
             JOIN identity.people p               ON p.id = r.person_id
             LEFT JOIN reference.countries pays   ON pays.id = p.country_id
             LEFT JOIN negotiation.spaces s       ON s.id = r.space_id
             LEFT JOIN negotiation.invitation_codes c ON c.id = r.invitation_code_id
             LEFT JOIN identity.people d          ON d.id = r.decided_by
            WHERE ($1::text IS NULL OR r.status::text = $1)
            ORDER BY (r.status = 'pending') DESC, r.submitted_at
            LIMIT $2 OFFSET $3"#,
        filtre.etat,
        filtre.limit,
        filtre.offset,
        locale
    )
    .fetch_all(&mut *conn)
    .await?;

    let total = lignes.first().map(|l| l.total).unwrap_or(0);

    // **Le compte des demandes en attente ne suit pas le filtre** : c'est la
    // pastille du menu, et elle ne doit pas tomber à zéro parce qu'on regarde
    // les refusées. Il est relu séparément quand la page est vide.
    let pending = match lignes.first() {
        Some(l) => l.pending,
        None => en_attente_total(&mut *conn).await?,
    };

    let rows = lignes
        .into_iter()
        .map(|l| {
            let status = RequestStatus::from_db(&l.status).ok_or_else(|| {
                ApiError::internal(format!("état de demande « {} » inconnu", l.status))
            })?;
            let kind = ScopeType::from_db(&l.scope_type).ok_or_else(|| {
                ApiError::internal(format!("portée « {} » inconnue", l.scope_type))
            })?;

            Ok(AccessRequestRow {
                id: l.id,
                status,
                person_id: l.person_id,
                display_name: l.display_name,
                email: l.email,
                country: l.country,
                scope: AccessScopeView {
                    kind,
                    id: l.space_id,
                    name: l.space_name,
                },
                invitation_code: l.invitation_code,
                invitation_code_label: l.invitation_code_label,
                message: l.message,
                submitted_at: l.submitted_at,
                decided_at: l.decided_at,
                decided_by_name: l.decided_by_name,
                decision_reason: l.decision_reason,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok((rows, total, pending))
}

pub async fn en_attente_total(conn: &mut PgConnection) -> Result<i64> {
    let compte = sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!" FROM negotiation.access_requests WHERE status = 'pending'"#
    )
    .fetch_one(conn)
    .await?;

    Ok(compte)
}

/// Ce qu'une demande porte au moment d'être tranchée : de quoi accorder
/// l'accès, rejoindre le réseau, et écrire le courriel.
#[derive(Debug, Clone)]
pub struct DemandeATrancher {
    pub id: Uuid,
    pub person_id: Uuid,
    pub scope_type: String,
    pub space_id: Option<Uuid>,
    pub invitation_code_id: Option<Uuid>,
    /// Réseau que portait le code de la demande, s'il y en avait un (FR-027).
    pub network_term_id: Option<Uuid>,
    pub email: String,
    pub first_name: String,
    pub locale: String,
    /// Nom de l'espace, résolu : le courriel dit ce qui s'ouvre.
    pub space_name: Option<String>,
}

/// La demande, **verrouillée pour la décision**.
///
/// `FOR UPDATE OF r` sérialise deux administrateurs qui trancheraient à la même
/// seconde : le second lira l'état écrit par le premier, et le trigger de
/// transition lui refusera sa décision. Sans ce verrou, les deux liraient
/// `pending` et la seconde écriture échouerait plus loin, après avoir déjà
/// accordé l'accès.
pub async fn a_trancher(
    conn: &mut PgConnection,
    request_id: Uuid,
    locale: &str,
) -> Result<Option<(DemandeATrancher, RequestStatus)>> {
    let ligne = sqlx::query!(
        r#"SELECT r.id                      AS "id!",
                  r.status::text            AS "status!",
                  r.person_id               AS "person_id!",
                  r.scope_type::text        AS "scope_type!",
                  r.space_id,
                  r.invitation_code_id,
                  c.grants_network_term_id,
                  p.primary_email::text     AS "email!",
                  p.first_name              AS "first_name!",
                  p.preferred_locale        AS "preferred_locale!",
                  platform.t(s.name, $2)    AS space_name
             FROM negotiation.access_requests r
             JOIN identity.people p ON p.id = r.person_id
             LEFT JOIN negotiation.spaces s ON s.id = r.space_id
             LEFT JOIN negotiation.invitation_codes c ON c.id = r.invitation_code_id
            WHERE r.id = $1
              FOR UPDATE OF r"#,
        request_id,
        locale
    )
    .fetch_optional(conn)
    .await?;

    let Some(l) = ligne else {
        return Ok(None);
    };

    let status = RequestStatus::from_db(&l.status)
        .ok_or_else(|| ApiError::internal(format!("état de demande « {} » inconnu", l.status)))?;

    Ok(Some((
        DemandeATrancher {
            id: l.id,
            person_id: l.person_id,
            scope_type: l.scope_type,
            space_id: l.space_id,
            invitation_code_id: l.invitation_code_id,
            network_term_id: l.grants_network_term_id,
            email: l.email,
            first_name: l.first_name,
            locale: l.preferred_locale,
            space_name: l.space_name,
        },
        status,
    )))
}

/// Écrit la décision. L'événement `negotiation.access_request.approved` ou
/// `.rejected` part **de la base**, par `tg_access_request_event()`, dans cette
/// transaction : l'émettre ici le doublerait.
///
/// La transition d'un état final est refusée **par le trigger**, et l'appelant
/// traduit ce refus (principe VIII).
pub async fn trancher(
    conn: &mut PgConnection,
    request_id: Uuid,
    issue: RequestStatus,
    acteur: Uuid,
    motif: Option<&str>,
) -> Result<()> {
    let etat = match issue {
        RequestStatus::Approved => "approved",
        RequestStatus::Rejected => "rejected",
        RequestStatus::Cancelled => "cancelled",
        RequestStatus::Pending => {
            return Err(ApiError::internal(
                "une décision ne remet jamais une demande en attente",
            ))
        }
    };

    sqlx::query!(
        "UPDATE negotiation.access_requests
            SET status = $2::text::negotiation.access_request_status,
                decided_at = now(), decided_by = $3, decision_reason = $4
          WHERE id = $1",
        request_id,
        etat,
        acteur,
        motif
    )
    .execute(conn)
    .await
    .map_err(traduire_la_transition)?;

    Ok(())
}

/// La personne retire sa demande. `false` : elle n'était plus en attente.
///
/// **Aucun courriel, aucun événement** : `tg_access_request_event()` n'émet que
/// pour `approved` et `rejected`, et c'est voulu — personne n'a rien à recevoir
/// pour une demande que son auteur vient de refermer.
pub async fn annuler(conn: &mut PgConnection, request_id: Uuid, person_id: Uuid) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE negotiation.access_requests
            SET status = 'cancelled', decided_at = now()
          WHERE id = $1 AND person_id = $2 AND status = 'pending'",
        request_id,
        person_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees > 0)
}

/// Existe-t-elle, et appartient-elle à cette personne ? C'est ce qui distingue
/// « déjà tranchée » de « demande d'un autre compte », sans que la réponse
/// révèle la seconde.
pub async fn appartient_a(
    conn: &mut PgConnection,
    request_id: Uuid,
    person_id: Uuid,
) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM negotiation.access_requests
                WHERE id = $1 AND person_id = $2
           ) AS "existe!""#,
        request_id,
        person_id
    )
    .fetch_one(conn)
    .await?;

    Ok(existe)
}

/// `tg_access_request_transition()` lève `integrity_constraint_violation` — et
/// le message français est déjà écrit dans le modèle. L'API lui substitue son
/// code stable, que le client range sous son discriminant.
fn traduire_la_transition(erreur: sqlx::Error) -> ApiError {
    if kernel::pg_error::sqlstate(&erreur).as_deref() == Some("23000") {
        ApiError::new(kernel::error::ErrorCode::NegotiationAccessRequestDecided)
    } else {
        erreur.into()
    }
}
