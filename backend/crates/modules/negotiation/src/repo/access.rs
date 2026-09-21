//! L'accès d'une personne — **lu dans le RBAC, jamais dans ce module**.
//!
//! # AUCUNE REQUÊTE ICI NE NOMME UN RÔLE
//!
//! Les lectures partent de la **permission** `negotiation.space.access` et
//! remontent aux attributions par `identity.role_permissions` : c'est la règle
//! métier n° 8, et elle vaut aussi pour afficher un état, pas seulement pour
//! autoriser. Le jour où un second rôle ouvrira l'espace réservé — un
//! facilitateur, un expert —, ces écrans le verront sans qu'on y touche.
//!
//! **L'attribution, elle, doit nommer le rôle** : accorder un droit, c'est
//! choisir un rôle, et le modèle a tranché lequel — `negotiator`, dont les
//! `allowed_scopes` valent exactement `{global, negotiation_space}`, les deux
//! portées qu'un code peut ouvrir.

use kernel::auth::ScopeType;
use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::access::{
    AccessRequestView, AccessScopeView, GrantedAccess, NetworkView, RequestStatus,
};
use crate::domain::permissions::SPACE_ACCESS;

/// Le rôle que l'admission attribue. **Le seul endroit de ce crate qui le
/// nomme** : tout le reste raisonne en permission et en portée.
const ROLE_ADMIS: &str = "negotiator";

/// L'accès en cours, s'il y en a un — avec ce qu'il ouvre, depuis quand, et par
/// quel code il est venu.
///
/// L'ordre de choix quand une personne en détient plusieurs : **la portée la
/// plus large d'abord**. Une attribution globale ouvre tout Guide Négo ;
/// l'annoncer comme « COP31 » serait dire moins que la vérité.
pub async fn accorde(
    conn: &mut PgConnection,
    person_id: Uuid,
    locale: &str,
) -> Result<Option<GrantedAccess>> {
    let ligne = sqlx::query!(
        r#"SELECT ra.scope_type::text        AS "scope_type!",
                  ra.scope_id,
                  ra.granted_at             AS "granted_at!",
                  platform.t(s.name, $3)    AS space_name,
                  -- Le « ? » force la nullabilité : la jointure latérale est
                  -- externe, mais `c.label` est NOT NULL et sqlx en conclut
                  -- que la colonne jointe l'est aussi.
                  u.code_label              AS "code_label?"
             FROM identity.role_assignments ra
             JOIN identity.role_permissions rp ON rp.role_code = ra.role_code
             LEFT JOIN negotiation.spaces s ON s.id = ra.scope_id
             LEFT JOIN LATERAL (
                 SELECT c.label AS code_label
                   FROM negotiation.invitation_code_uses cu
                   JOIN negotiation.invitation_codes c ON c.id = cu.code_id
                  WHERE cu.person_id = ra.person_id
                    AND c.scope_type = ra.scope_type
                    AND c.space_id IS NOT DISTINCT FROM ra.scope_id
                  ORDER BY cu.used_at DESC
                  LIMIT 1
             ) u ON true
            WHERE ra.person_id = $1
              AND rp.permission_code = $2
              AND ra.revoked_at IS NULL
              AND ra.valid_from <= now()
              AND (ra.valid_until IS NULL OR ra.valid_until > now())
            ORDER BY (ra.scope_type = 'global') DESC, ra.granted_at
            LIMIT 1"#,
        person_id,
        SPACE_ACCESS,
        locale
    )
    .fetch_optional(conn)
    .await?;

    let Some(ligne) = ligne else {
        return Ok(None);
    };

    let kind = ScopeType::from_db(&ligne.scope_type).ok_or_else(|| {
        ApiError::internal(format!(
            "portée « {} » inconnue du code : le modèle et l'énuméré ont divergé",
            ligne.scope_type
        ))
    })?;

    Ok(Some(GrantedAccess {
        scope: AccessScopeView {
            kind,
            id: ligne.scope_id,
            name: ligne.space_name,
        },
        granted_at: ligne.granted_at,
        source_code_label: ligne.code_label,
    }))
}

/// A-t-elle eu un accès **qu'on lui a retiré** ? C'est ce qui distingue
/// « visiteuse » de « accès retiré » dans « Mon accès » (FR-033).
pub async fn a_ete_retire(conn: &mut PgConnection, person_id: Uuid) -> Result<bool> {
    let retire = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1
                 FROM identity.role_assignments ra
                 JOIN identity.role_permissions rp ON rp.role_code = ra.role_code
                WHERE ra.person_id = $1
                  AND rp.permission_code = $2
                  AND ra.revoked_at IS NOT NULL
           ) AS "retire!""#,
        person_id,
        SPACE_ACCESS
    )
    .fetch_one(conn)
    .await?;

    Ok(retire)
}

/// Les appartenances en cours. Elles n'ouvrent aucun droit (FR-014) : le
/// libellé vient de la taxonomie, et non d'un fichier de traduction.
pub async fn reseaux(
    conn: &mut PgConnection,
    person_id: Uuid,
    locale: &str,
) -> Result<Vec<NetworkView>> {
    let lignes = sqlx::query!(
        r#"SELECT t.code                                       AS "code!",
                  COALESCE(platform.t(t.label, $2), t.code)     AS "label!"
             FROM negotiation.network_memberships m
             JOIN reference.taxonomy_terms t ON t.id = m.network_term_id
            WHERE m.person_id = $1 AND m.left_at IS NULL
            ORDER BY m.joined_at"#,
        person_id,
        locale
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| NetworkView {
            code: l.code,
            label: l.label,
        })
        .collect())
}

/// La demande la plus récente, **quelle que soit son issue et sa portée**.
///
/// À cette étape l'espace réservé n'en a qu'un, et l'écran « Mon accès » n'en
/// montre qu'une : distinguer les portées produirait un second état à afficher
/// sans qu'aucun écran le demande.
pub async fn derniere_demande(
    conn: &mut PgConnection,
    person_id: Uuid,
) -> Result<Option<AccessRequestView>> {
    let ligne = sqlx::query!(
        r#"SELECT r.id                  AS "id!",
                  r.status::text        AS "status!",
                  r.submitted_at        AS "submitted_at!",
                  r.decided_at,
                  r.decision_reason
             FROM negotiation.access_requests r
            WHERE r.person_id = $1
            ORDER BY r.submitted_at DESC
            LIMIT 1"#,
        person_id
    )
    .fetch_optional(conn)
    .await?;

    let Some(ligne) = ligne else {
        return Ok(None);
    };

    let status = RequestStatus::from_db(&ligne.status).ok_or_else(|| {
        ApiError::internal(format!("état de demande « {} » inconnu", ligne.status))
    })?;

    Ok(Some(AccessRequestView {
        id: ligne.id,
        status,
        submitted_at: ligne.submitted_at,
        decided_at: ligne.decided_at,
        decision_reason: ligne.decision_reason,
    }))
}

// ---------------------------------------------------------------------------
// Écritures — toutes dans la transaction de l'appelant
// ---------------------------------------------------------------------------

/// Attribue l'accès, avec la portée du code.
///
/// `granted_by` reste nul : personne ne l'a accordé, un code l'a ouvert.
/// L'auteur du geste est dans `platform.audit_log`, posé par `Db::write`.
///
/// `None` en retour : une attribution vivante existait déjà — deux requêtes
/// concurrentes, ou une portée déjà couverte. C'est l'index partiel
/// `ux_role_assignments_active` qui l'a dit, pas un `SELECT` préalable.
pub async fn accorder(
    conn: &mut PgConnection,
    person_id: Uuid,
    scope_type: &str,
    scope_id: Option<Uuid>,
    note: &str,
) -> Result<Option<Uuid>> {
    let insere = sqlx::query_scalar!(
        "INSERT INTO identity.role_assignments
             (person_id, role_code, scope_type, scope_id, note)
         VALUES ($1, $2, $3::text::identity.scope_type, $4, $5)
         ON CONFLICT (person_id, role_code, scope_type,
                      COALESCE(scope_id, '00000000-0000-0000-0000-000000000000'::uuid))
             WHERE revoked_at IS NULL
             DO NOTHING
         RETURNING id",
        person_id,
        ROLE_ADMIS,
        scope_type,
        scope_id,
        note
    )
    .fetch_optional(conn)
    .await?;

    Ok(insere)
}

/// Inscrit la personne à l'annuaire de l'espace.
///
/// **L'annuaire n'accorde rien** : le droit vient de l'attribution ci-dessus.
/// Les deux s'écrivent dans la même transaction, et en cas de divergence le
/// RBAC fait foi — c'est le commentaire de la table qui le dit.
pub async fn inscrire_a_lespace(
    conn: &mut PgConnection,
    space_id: Uuid,
    person_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.space_members (space_id, person_id, role)
         VALUES ($1, $2, 'negotiator')
         ON CONFLICT (space_id, person_id) WHERE left_at IS NULL DO NOTHING",
        space_id,
        person_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// L'appartenance au réseau, venue du code. `true` si elle vient de s'ajouter —
/// c'est ce que le message dit à une personne qui avait déjà l'accès (FR-018).
pub async fn rejoindre_le_reseau(
    conn: &mut PgConnection,
    person_id: Uuid,
    network_term_id: Uuid,
    source_code_id: Uuid,
) -> Result<bool> {
    let insere = sqlx::query_scalar!(
        "INSERT INTO negotiation.network_memberships
             (person_id, network_term_id, source_code_id)
         VALUES ($1, $2, $3)
         ON CONFLICT (person_id, network_term_id) WHERE left_at IS NULL DO NOTHING
         RETURNING id",
        person_id,
        network_term_id,
        source_code_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(insere.is_some())
}

/// La demande en attente **s'annule** quand un code l'a devancée (US4, scénario
/// 6 ; FR-026). « Annulée » est le fait de la personne : elle est entrée
/// autrement. « Révoquée » ne qualifie jamais une demande.
///
/// Aucun événement n'en part : `tg_access_request_event` n'émet que pour
/// `approved` et `rejected`, et c'est voulu — personne n'a de courriel à
/// recevoir pour une demande que son propre code a rendue inutile.
pub async fn annuler_la_demande_en_attente(
    conn: &mut PgConnection,
    person_id: Uuid,
    scope_type: &str,
    scope_id: Option<Uuid>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.access_requests
            SET status = 'cancelled', decided_at = now()
          WHERE person_id = $1
            AND status = 'pending'
            AND scope_type = $2::text::identity.scope_type
            AND space_id IS NOT DISTINCT FROM $3",
        person_id,
        scope_type,
        scope_id
    )
    .execute(conn)
    .await?;

    Ok(())
}
