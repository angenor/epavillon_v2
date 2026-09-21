//! Les demandes d'accès. **Une seule en attente par personne et par portée**,
//! et c'est la base qui le tient : `ux_access_requests_pending_space` et
//! `ux_access_requests_pending_global`.
//!
//! Ce fichier ne porte à cette étape que ce dont le code d'invitation a besoin —
//! ouvrir une demande quand le mode l'exige. La file du back-office et les deux
//! décisions arrivent avec le récit qui les demande.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::access::{AccessRequestView, RequestStatus};

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
