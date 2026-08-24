//! Les adhésions : un seul ordre pour demander, deux lectures pour les deux
//! files.
//!
//! **`ux_memberships (organization_id, person_id)` ne connaît pas le statut.**
//! Une adhésion révoquée occupe la place : lire puis écrire laisserait une
//! fenêtre où deux demandes simultanées produiraient une violation de contrainte
//! au lieu d'une réponse propre. La demande est donc **un unique ordre** avec
//! reprise conditionnelle, et c'est la base qui tranche (research.md § R7,
//! écart n° 72).
//!
//! **`is_primary` n'est jamais calculé ici** : `tg_default_primary_membership`
//! attribue la primauté à la première adhésion active, et
//! `tg_sync_primary_organization` la répercute sur la personne.

use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::domain::ids::{MembershipId, OrganizationId, PersonId};
use crate::domain::membership::{
    MemberEntry, MemberPerson, Membership, MembershipRole, MembershipStatus,
};

/// Ce que la demande de rattachement produit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// Ligne créée, ou adhésion révoquée reprise.
    Opened,
    /// Une adhésion vivante existait déjà.
    AlreadyThere,
}

/// Demande de rattachement — **un seul ordre**.
///
/// `DO UPDATE … WHERE status = 'revoked'` : une adhésion révoquée est reprise,
/// une adhésion vivante ne bouge pas et l'ordre ne rend rien. Ce qui repart à
/// zéro : l'état, le rôle demandé, la fonction déclarée, la date de révocation.
/// Ce qui ne bouge pas : la date de création — l'histoire de l'adhésion se lit
/// dans le journal d'audit, pas dans une ligne réécrite.
///
/// `approved_at` et `approved_by` sont posés quand le domaine a tranché : un
/// rattachement automatique **est** une approbation, et laisser ces colonnes
/// nulles ferait croire à une adhésion active que personne n'a jamais validée.
pub async fn request(
    conn: &mut PgConnection,
    organization_id: OrganizationId,
    person_id: PersonId,
    role: MembershipRole,
    job_title: Option<&str>,
    auto_join: bool,
) -> Result<(RequestOutcome, Membership)> {
    let statut = if auto_join { "active" } else { "pending" };
    let approbateur = auto_join.then(|| person_id.as_uuid());

    let ligne = sqlx::query_as!(
        Ligne,
        r#"INSERT INTO org.memberships
               (organization_id, person_id, role, status, job_title, approved_by, approved_at)
           VALUES ($1, $2, $3::text::org.membership_role,
                   $4::text::org.membership_status, $5, $6::uuid,
                   CASE WHEN $6::uuid IS NULL THEN NULL ELSE now() END)
           ON CONFLICT (organization_id, person_id) DO UPDATE
              SET status      = EXCLUDED.status,
                  role        = EXCLUDED.role,
                  job_title   = EXCLUDED.job_title,
                  approved_by = EXCLUDED.approved_by,
                  approved_at = EXCLUDED.approved_at,
                  revoked_at  = NULL,
                  invited_by  = NULL,
                  invited_at  = NULL
            WHERE org.memberships.status = 'revoked'
        RETURNING id, organization_id, person_id, role::text AS "role!",
                  status::text AS "statut!", is_primary, job_title,
                  invited_by, invited_at, approved_by, approved_at, revoked_at,
                  created_at, updated_at"#,
        organization_id.as_uuid(),
        person_id.as_uuid(),
        role.as_str(),
        statut,
        job_title,
        approbateur
    )
    .fetch_optional(&mut *conn)
    .await?;

    match ligne {
        Some(l) => Ok((RequestOutcome::Opened, Membership::try_from(l)?)),
        // Rien n'en est sorti : une adhésion vivante occupe la place. On la
        // relit — c'est ce que « déjà membre » doit porter.
        None => {
            let existante = by_couple(&mut *conn, organization_id, person_id)
                .await?
                .ok_or_else(|| {
                    ApiError::internal(
                        "l'insertion n'a rien rendu et aucune adhésion n'existe : \
                         la ligne a disparu entre les deux ordres",
                    )
                })?;
            Ok((RequestOutcome::AlreadyThere, existante))
        }
    }
}

pub async fn by_couple<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: OrganizationId,
    person_id: PersonId,
) -> Result<Option<Membership>> {
    let ligne = sqlx::query_as!(
        Ligne,
        r#"SELECT id, organization_id, person_id, role::text AS "role!",
                  status::text AS "statut!", is_primary, job_title,
                  invited_by, invited_at, approved_by, approved_at, revoked_at,
                  created_at, updated_at
             FROM org.memberships
            WHERE organization_id = $1 AND person_id = $2"#,
        organization_id.as_uuid(),
        person_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    ligne.map(Membership::try_from).transpose()
}

pub async fn by_id<'e>(
    executor: impl PgExecutor<'e>,
    id: MembershipId,
) -> Result<Option<Membership>> {
    let ligne = sqlx::query_as!(
        Ligne,
        r#"SELECT id, organization_id, person_id, role::text AS "role!",
                  status::text AS "statut!", is_primary, job_title,
                  invited_by, invited_at, approved_by, approved_at, revoked_at,
                  created_at, updated_at
             FROM org.memberships
            WHERE id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    ligne.map(Membership::try_from).transpose()
}

/// Les adhésions **vivantes** d'une personne : actives et en attente. Les
/// révoquées ne s'affichent pas — elles n'ont d'intérêt que pour l'audit.
pub async fn of_person(pool: &PgPool, person_id: PersonId) -> Result<Vec<Membership>> {
    let lignes = sqlx::query_as!(
        Ligne,
        r#"SELECT id, organization_id, person_id, role::text AS "role!",
                  status::text AS "statut!", is_primary, job_title,
                  invited_by, invited_at, approved_by, approved_at, revoked_at,
                  created_at, updated_at
             FROM org.memberships
            WHERE person_id = $1 AND status <> 'revoked'
            ORDER BY is_primary DESC, created_at"#,
        person_id.as_uuid()
    )
    .fetch_all(pool)
    .await?;

    lignes.into_iter().map(Membership::try_from).collect()
}

/// L'appelant est-il **référent actif** de cette organisation ?
///
/// Ce n'est pas une permission mais une **qualité**, lue en base à chaque
/// écriture. Un rôle d'organisation existe bien dans le modèle, mais rien ne
/// l'attribue et il ne porte que deux permissions étrangères à ce module : le
/// tester serait tester un nom de rôle, ce que le principe V interdit.
pub async fn is_manager<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: OrganizationId,
    person_id: PersonId,
) -> Result<bool> {
    let referent = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM org.memberships
                WHERE organization_id = $1 AND person_id = $2
                  AND role = 'manager' AND status = 'active'
           ) AS "referent!""#,
        organization_id.as_uuid(),
        person_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(referent)
}

/// Combien de référents **actifs** restent, hors celui qu'on retire.
///
/// C'est ce qui décide de `ORG_LAST_MANAGER` : une organisation sans référent
/// n'a plus personne pour accepter une demande, et se retrouve close sans que
/// quiconque l'ait voulu.
pub async fn other_active_managers(
    conn: &mut PgConnection,
    organization_id: OrganizationId,
    sauf: MembershipId,
) -> Result<i64> {
    let restants = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.memberships
            WHERE organization_id = $1 AND id <> $2
              AND role = 'manager' AND status = 'active'"#,
        organization_id.as_uuid(),
        sauf.as_uuid()
    )
    .fetch_one(conn)
    .await?;

    Ok(restants)
}

/// Crée l'adhésion d'une **invitation** : `invited_by` et `invited_at` posés
/// ensemble — `ck_memberships_invitation` interdit d'en renseigner une seule, et
/// c'est ce couple qui porte la direction de l'attente.
pub async fn invite(
    conn: &mut PgConnection,
    organization_id: OrganizationId,
    person_id: PersonId,
    role: MembershipRole,
    job_title: Option<&str>,
    invited_by: PersonId,
) -> Result<Option<Membership>> {
    let ligne = sqlx::query_as!(
        Ligne,
        r#"INSERT INTO org.memberships
               (organization_id, person_id, role, status, job_title, invited_by, invited_at)
           VALUES ($1, $2, $3::text::org.membership_role, 'pending', $4, $5, now())
           ON CONFLICT (organization_id, person_id) DO UPDATE
              SET status     = 'pending',
                  role       = EXCLUDED.role,
                  job_title  = EXCLUDED.job_title,
                  invited_by = EXCLUDED.invited_by,
                  invited_at = now(),
                  revoked_at = NULL
            WHERE org.memberships.status = 'revoked'
        RETURNING id, organization_id, person_id, role::text AS "role!",
                  status::text AS "statut!", is_primary, job_title,
                  invited_by, invited_at, approved_by, approved_at, revoked_at,
                  created_at, updated_at"#,
        organization_id.as_uuid(),
        person_id.as_uuid(),
        role.as_str(),
        job_title,
        invited_by.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    ligne.map(Membership::try_from).transpose()
}

/// Approuve une adhésion : l'auteur et la date sont posés, et c'est la base qui
/// décide de la primauté.
/// `fonction` nulle = celle déjà posée sur la ligne est conservée.
///
/// Les deux appelants n'ont pas la même situation. Le référent qui approuve une
/// DEMANDE ne touche à rien : la personne a déclaré sa fonction en demandant.
/// La personne qui accepte une INVITATION, elle, l'apporte — c'est le moment où
/// elle parle d'elle-même, et l'adhésion ne peut pas devenir active sans
/// (`ck_memberships_job_title`).
pub async fn approve(
    conn: &mut PgConnection,
    id: MembershipId,
    approuve_par: PersonId,
    fonction: Option<&str>,
) -> Result<Option<Membership>> {
    let ligne = sqlx::query_as!(
        Ligne,
        r#"UPDATE org.memberships
              SET status = 'active', approved_by = $2, approved_at = now(), revoked_at = NULL,
                  job_title = COALESCE($3, job_title)
            WHERE id = $1 AND status = 'pending'
        RETURNING id, organization_id, person_id, role::text AS "role!",
                  status::text AS "statut!", is_primary, job_title,
                  invited_by, invited_at, approved_by, approved_at, revoked_at,
                  created_at, updated_at"#,
        id.as_uuid(),
        approuve_par.as_uuid(),
        fonction
    )
    .fetch_optional(conn)
    .await?;

    ligne.map(Membership::try_from).transpose()
}

/// Révoque une adhésion — **la ligne n'est jamais supprimée**.
///
/// Supprimer effacerait l'histoire, et la personne pourrait redemander comme si
/// rien ne s'était passé. La révocation, elle, se relit et se reprend.
///
/// `is_primary` retombe : `ux_memberships_primary` ne porte que sur les
/// adhésions actives, mais laisser le drapeau posé ferait réapparaître la
/// primauté à la première reprise.
pub async fn revoke(conn: &mut PgConnection, id: MembershipId) -> Result<Option<Membership>> {
    let ligne = sqlx::query_as!(
        Ligne,
        r#"UPDATE org.memberships
              SET status = 'revoked', revoked_at = now(), is_primary = false
            WHERE id = $1 AND status <> 'revoked'
        RETURNING id, organization_id, person_id, role::text AS "role!",
                  status::text AS "statut!", is_primary, job_title,
                  invited_by, invited_at, approved_by, approved_at, revoked_at,
                  created_at, updated_at"#,
        id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    ligne.map(Membership::try_from).transpose()
}

/// La file d'un **référent** : les demandes reçues, jamais ses propres
/// invitations. `invited_at IS NULL` est le filtre, et c'est l'index
/// `ix_memberships_requests` qui le sert.
pub async fn requests_for_organization<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: OrganizationId,
) -> Result<Vec<MemberEntry>> {
    let lignes = sqlx::query_as!(
        LigneEntree,
        r#"SELECT m.id, m.organization_id, m.person_id, m.role::text AS "role!",
                  m.status::text AS "statut!", m.is_primary, m.job_title,
                  m.invited_by, m.invited_at, m.approved_by, m.approved_at, m.revoked_at,
                  m.created_at, m.updated_at,
                  p.display_name AS "display_name!",
                  p.primary_email::text AS "primary_email!",
                  p.first_name AS "first_name!", p.last_name AS "last_name!",
                  p.preferred_locale AS "preferred_locale!"
             FROM org.memberships m
             JOIN identity.people p ON p.id = m.person_id
            WHERE m.organization_id = $1 AND m.status = 'pending' AND m.invited_at IS NULL
            ORDER BY m.created_at DESC"#,
        organization_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    lignes.into_iter().map(MemberEntry::try_from).collect()
}

/// La file d'une **personne** : les invitations reçues, jamais ses propres
/// demandes. `invited_at IS NOT NULL`, servi par `ix_memberships_invitations`.
pub async fn invitations_for_person<'e>(
    executor: impl PgExecutor<'e>,
    person_id: PersonId,
) -> Result<Vec<MemberEntry>> {
    let lignes = sqlx::query_as!(
        LigneEntree,
        r#"SELECT m.id, m.organization_id, m.person_id, m.role::text AS "role!",
                  m.status::text AS "statut!", m.is_primary, m.job_title,
                  m.invited_by, m.invited_at, m.approved_by, m.approved_at, m.revoked_at,
                  m.created_at, m.updated_at,
                  p.display_name AS "display_name!",
                  p.primary_email::text AS "primary_email!",
                  p.first_name AS "first_name!", p.last_name AS "last_name!",
                  p.preferred_locale AS "preferred_locale!"
             FROM org.memberships m
             JOIN identity.people p ON p.id = m.person_id
            WHERE m.person_id = $1 AND m.status = 'pending' AND m.invited_at IS NOT NULL
            ORDER BY m.invited_at DESC"#,
        person_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    lignes.into_iter().map(MemberEntry::try_from).collect()
}

/// Tous les membres d'une organisation, révoqués compris : c'est la fiche du
/// back-office qui les demande, et une adhésion retirée fait partie de
/// l'histoire de la fiche.
pub async fn all_of_organization<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: OrganizationId,
) -> Result<Vec<MemberEntry>> {
    let lignes = sqlx::query_as!(
        LigneEntree,
        r#"SELECT m.id, m.organization_id, m.person_id, m.role::text AS "role!",
                  m.status::text AS "statut!", m.is_primary, m.job_title,
                  m.invited_by, m.invited_at, m.approved_by, m.approved_at, m.revoked_at,
                  m.created_at, m.updated_at,
                  p.display_name AS "display_name!",
                  p.primary_email::text AS "primary_email!",
                  p.first_name AS "first_name!", p.last_name AS "last_name!",
                  p.preferred_locale AS "preferred_locale!"
             FROM org.memberships m
             JOIN identity.people p ON p.id = m.person_id
            WHERE m.organization_id = $1
            ORDER BY m.status, m.role, p.display_name"#,
        organization_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    lignes.into_iter().map(MemberEntry::try_from).collect()
}

/// L'entrée d'un membre, personne comprise — ce que la file affiche.
pub async fn entry_of<'e>(
    executor: impl PgExecutor<'e>,
    id: MembershipId,
) -> Result<Option<MemberEntry>> {
    let ligne = sqlx::query_as!(
        LigneEntree,
        r#"SELECT m.id, m.organization_id, m.person_id, m.role::text AS "role!",
                  m.status::text AS "statut!", m.is_primary, m.job_title,
                  m.invited_by, m.invited_at, m.approved_by, m.approved_at, m.revoked_at,
                  m.created_at, m.updated_at,
                  p.display_name AS "display_name!",
                  p.primary_email::text AS "primary_email!",
                  p.first_name AS "first_name!", p.last_name AS "last_name!",
                  p.preferred_locale AS "preferred_locale!"
             FROM org.memberships m
             JOIN identity.people p ON p.id = m.person_id
            WHERE m.id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    ligne.map(MemberEntry::try_from).transpose()
}

// -----------------------------------------------------------------------------

/// Les quatorze colonnes de `org.memberships`.
pub(crate) struct Ligne {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub person_id: Uuid,
    pub role: String,
    pub statut: String,
    pub is_primary: bool,
    pub job_title: Option<String>,
    pub invited_by: Option<Uuid>,
    pub invited_at: Option<time::OffsetDateTime>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<time::OffsetDateTime>,
    pub revoked_at: Option<time::OffsetDateTime>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

impl TryFrom<Ligne> for Membership {
    type Error = ApiError;

    fn try_from(l: Ligne) -> Result<Self> {
        Ok(Self {
            id: MembershipId(l.id),
            organization_id: OrganizationId(l.organization_id),
            person_id: PersonId(l.person_id),
            role: role(&l.role)?,
            status: statut(&l.statut)?,
            is_primary: l.is_primary,
            job_title: l.job_title,
            invited_by: l.invited_by.map(PersonId),
            invited_at: l.invited_at,
            approved_by: l.approved_by.map(PersonId),
            approved_at: l.approved_at,
            revoked_at: l.revoked_at,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
    }
}

struct LigneEntree {
    id: Uuid,
    organization_id: Uuid,
    person_id: Uuid,
    role: String,
    statut: String,
    is_primary: bool,
    job_title: Option<String>,
    invited_by: Option<Uuid>,
    invited_at: Option<time::OffsetDateTime>,
    approved_by: Option<Uuid>,
    approved_at: Option<time::OffsetDateTime>,
    revoked_at: Option<time::OffsetDateTime>,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
    display_name: String,
    primary_email: String,
    first_name: String,
    last_name: String,
    preferred_locale: String,
}

impl TryFrom<LigneEntree> for MemberEntry {
    type Error = ApiError;

    fn try_from(l: LigneEntree) -> Result<Self> {
        let membership = Membership::try_from(Ligne {
            id: l.id,
            organization_id: l.organization_id,
            person_id: l.person_id,
            role: l.role,
            statut: l.statut,
            is_primary: l.is_primary,
            job_title: l.job_title,
            invited_by: l.invited_by,
            invited_at: l.invited_at,
            approved_by: l.approved_by,
            approved_at: l.approved_at,
            revoked_at: l.revoked_at,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })?;

        Ok(Self {
            is_invitation: membership.is_invitation(),
            person: MemberPerson {
                id: PersonId(l.person_id),
                display_name: l.display_name,
                primary_email: l.primary_email,
                first_name: l.first_name,
                last_name: l.last_name,
                preferred_locale: l.preferred_locale,
            },
            membership,
        })
    }
}

pub(crate) fn role(valeur: &str) -> Result<MembershipRole> {
    MembershipRole::from_db(valeur)
        .ok_or_else(|| ApiError::internal(format!("rôle d'adhésion inconnu : {valeur}")))
}

pub(crate) fn statut(valeur: &str) -> Result<MembershipStatus> {
    MembershipStatus::from_db(valeur)
        .ok_or_else(|| ApiError::internal(format!("statut d'adhésion inconnu : {valeur}")))
}
