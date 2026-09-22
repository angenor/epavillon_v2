//! Qui est entré avec un code, et si son accès tient encore.
//!
//! **Tout passe par `negotiation.v_invitation_code_uses`.** La table des usages
//! ne porte aucun état d'accès — ce serait une seconde vérité à côté du RBAC —,
//! et c'est la vue qui joint l'attribution de rôle correspondant à la portée du
//! code. Refaire cette jointure ici la ferait diverger le jour où la portée
//! d'un code changerait de forme.
//!
//! # RÉVOQUER UN CODE ET RETIRER UN ACCÈS SONT DEUX GESTES
//!
//! ADR-006. Une fuite de code se referme en le révoquant, sans couper l'accès
//! de tout un réseau déjà entré ; un abus se sanctionne en retirant l'accès
//! d'une personne, sans invalider le code du groupe. Les confondre ferait de
//! chaque révocation une exclusion collective.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::admin::InvitationCodeUseRow;

/// Les usages d'un code, les accès encore ouverts d'abord.
pub async fn du_code(
    conn: &mut PgConnection,
    code_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<(Vec<InvitationCodeUseRow>, i64, i64)> {
    let lignes = sqlx::query!(
        r#"SELECT u.person_id             AS "person_id!",
                  u.display_name          AS "display_name!",
                  u.email::text           AS "email!",
                  u.used_at               AS "used_at!",
                  COALESCE(u.access_active, false) AS "access_active!",
                  u.access_revoked_at,
                  u.access_revoked_reason,
                  count(*) OVER ()                                  AS "total!",
                  count(*) FILTER (WHERE u.access_active) OVER ()    AS "ouverts!"
             FROM negotiation.v_invitation_code_uses u
            WHERE u.code_id = $1
            ORDER BY u.access_active DESC, u.used_at DESC
            LIMIT $2 OFFSET $3"#,
        code_id,
        limit,
        offset
    )
    .fetch_all(conn)
    .await?;

    let total = lignes.first().map(|l| l.total).unwrap_or(0);
    let ouverts = lignes.first().map(|l| l.ouverts).unwrap_or(0);

    let rows = lignes
        .into_iter()
        .map(|l| InvitationCodeUseRow {
            person_id: l.person_id,
            display_name: l.display_name,
            email: l.email,
            used_at: l.used_at,
            access_active: l.access_active,
            access_revoked_at: l.access_revoked_at,
            access_revoked_reason: l.access_revoked_reason,
        })
        .collect();

    Ok((rows, total, ouverts))
}

/// Combien de personnes entrées par ce code ont **encore** leur accès.
///
/// C'est ce que la fiche affiche à côté du bouton de révocation : révoquer le
/// code n'en retirera aucun, et l'administrateur doit le savoir avant, pas
/// après.
pub async fn acces_ouverts(conn: &mut PgConnection, code_id: Uuid) -> Result<i64> {
    let compte = sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!"
             FROM negotiation.v_invitation_code_uses u
            WHERE u.code_id = $1 AND u.access_active"#,
        code_id
    )
    .fetch_one(conn)
    .await?;

    Ok(compte)
}

/// Une attribution retirée, telle que l'événement la décrit.
#[derive(Debug, Clone)]
pub struct AccesRetire {
    /// L'attribution retirée : c'est elle l'agrégat de l'événement
    /// `space_access`, comme l'attribution posée l'était à l'octroi.
    pub role_assignment_id: Uuid,
    pub person_id: Uuid,
    pub scope_type: String,
    pub space_id: Option<Uuid>,
}

/// Retire l'accès d'une personne entrée par ce code.
///
/// **La portée retirée est celle du code**, et non « toutes celles de la
/// personne » : une négociatrice admise sur la COP31 par un code et sur
/// Guide Négo entier par un autre ne perd que ce que ce code lui avait ouvert.
///
/// `None` : elle n'a pas d'accès en cours sur cette portée — déjà retiré, ou
/// jamais accordé. L'opération est idempotente.
pub async fn retirer(
    conn: &mut PgConnection,
    code_id: Uuid,
    person_id: Uuid,
    acteur: Uuid,
    motif: Option<&str>,
) -> Result<Option<AccesRetire>> {
    let ligne = sqlx::query!(
        r#"UPDATE identity.role_assignments ra
              SET revoked_at = now(), revoked_by = $3, revoked_reason = $4
             FROM negotiation.invitation_codes c
            WHERE c.id = $1
              AND ra.person_id = $2
              AND ra.role_code = 'negotiator'
              AND ra.scope_type = c.scope_type
              AND ra.scope_id IS NOT DISTINCT FROM c.space_id
              AND ra.revoked_at IS NULL
        RETURNING ra.id AS "role_assignment_id!",
                  ra.scope_type::text AS "scope_type!", ra.scope_id"#,
        code_id,
        person_id,
        acteur,
        motif
    )
    .fetch_optional(&mut *conn)
    .await?;

    let Some(ligne) = ligne else {
        return Ok(None);
    };

    sortir_de_lespace(conn, person_id, ligne.scope_id).await?;

    Ok(Some(AccesRetire {
        role_assignment_id: ligne.role_assignment_id,
        person_id,
        scope_type: ligne.scope_type,
        space_id: ligne.scope_id,
    }))
}

/// Retire l'accès de **toutes** les personnes entrées par ce code.
///
/// Le geste d'un code compromis : il suit une révocation, il ne la remplace
/// pas. Les personnes déjà retirées ne sont pas touchées, et le nombre rendu
/// est celui des accès qui viennent réellement de tomber.
pub async fn retirer_tous(
    conn: &mut PgConnection,
    code_id: Uuid,
    acteur: Uuid,
    motif: Option<&str>,
) -> Result<Vec<AccesRetire>> {
    let lignes = sqlx::query!(
        r#"UPDATE identity.role_assignments ra
              SET revoked_at = now(), revoked_by = $2, revoked_reason = $3
             FROM negotiation.invitation_codes c
             JOIN negotiation.invitation_code_uses u ON u.code_id = c.id
            WHERE c.id = $1
              AND ra.person_id = u.person_id
              AND ra.role_code = 'negotiator'
              AND ra.scope_type = c.scope_type
              AND ra.scope_id IS NOT DISTINCT FROM c.space_id
              AND ra.revoked_at IS NULL
        RETURNING ra.id AS "role_assignment_id!",
                  ra.person_id AS "person_id!",
                  ra.scope_type::text AS "scope_type!",
                  ra.scope_id"#,
        code_id,
        acteur,
        motif
    )
    .fetch_all(&mut *conn)
    .await?;

    let retires: Vec<AccesRetire> = lignes
        .into_iter()
        .map(|l| AccesRetire {
            role_assignment_id: l.role_assignment_id,
            person_id: l.person_id,
            scope_type: l.scope_type,
            space_id: l.scope_id,
        })
        .collect();

    for retire in &retires {
        sortir_de_lespace(conn, retire.person_id, retire.space_id).await?;
    }

    Ok(retires)
}

/// L'annuaire de l'espace suit le droit : il n'accorde rien, mais laisser
/// quelqu'un dans la liste des membres d'un espace dont l'accès vient d'être
/// retiré ferait mentir l'écran qui la montre. Une portée globale n'a pas
/// d'espace à quitter.
async fn sortir_de_lespace(
    conn: &mut PgConnection,
    person_id: Uuid,
    space_id: Option<Uuid>,
) -> Result<()> {
    let Some(space_id) = space_id else {
        return Ok(());
    };

    sqlx::query!(
        "UPDATE negotiation.space_members
            SET left_at = now()
          WHERE space_id = $1 AND person_id = $2 AND left_at IS NULL",
        space_id,
        person_id
    )
    .execute(conn)
    .await?;

    Ok(())
}
