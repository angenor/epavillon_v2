//! `identity.sessions` : ouverture, lecture par empreinte, révocation.
//!
//! La table ne garde que l'empreinte SHA-256 du jeton de rafraîchissement — un
//! vol de la base ne donne aucun jeton utilisable. Le clair ne vit que dans le
//! cookie.
//!
//! Une session ne se supprime jamais : la rotation chaîne des lignes, chacune
//! portant son motif de révocation. C'est ce qui rend le rejeu détectable.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::{AccountId, PersonId, SessionId};

/// Les motifs employés par ce module. `anonymization` n'y figure pas : il est
/// écrit par `identity.anonymize_person()`, côté base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeReason {
    Rotated,
    Logout,
    LogoutAll,
    ReuseDetected,
    PasswordChanged,
    StatusChanged,
}

impl RevokeReason {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Rotated => "rotated",
            Self::Logout => "logout",
            Self::LogoutAll => "logout_all",
            Self::ReuseDetected => "reuse_detected",
            Self::PasswordChanged => "password_changed",
            Self::StatusChanged => "status_changed",
        }
    }
}

pub struct NewSession<'a> {
    pub person_id: PersonId,
    pub account_id: Option<AccountId>,
    pub refresh_token_hash: &'a [u8],
    pub user_agent: Option<&'a str>,
    pub ip_address: Option<std::net::IpAddr>,
    pub expires_at: OffsetDateTime,
}

/// `None` signale une collision d'empreinte : l'appelant régénère et rejoue une
/// fois. `ON CONFLICT DO NOTHING` plutôt qu'une erreur rattrapée, parce qu'un
/// échec d'insertion avorterait la transaction entière — et l'ouverture de
/// session partage la sienne avec la mise à jour du compte.
pub async fn create(conn: &mut PgConnection, session: NewSession<'_>) -> Result<Option<SessionId>> {
    let id = sqlx::query_scalar!(
        "INSERT INTO identity.sessions
             (person_id, account_id, refresh_token_hash, user_agent, ip_address, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (refresh_token_hash) DO NOTHING
         RETURNING id",
        session.person_id.as_uuid(),
        session.account_id.map(AccountId::as_uuid),
        session.refresh_token_hash,
        session.user_agent,
        // `inet` se lit en `IpNetwork` côté SQLx ; l'adresse d'un poste en est
        // le cas dégénéré, et la conversion garde la vérification à la
        // compilation là où un `as _` l'aurait écartée.
        session.ip_address.map(IpNetwork::from),
        session.expires_at
    )
    .fetch_optional(conn)
    .await?;

    Ok(id.map(SessionId))
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: SessionId,
    pub person_id: PersonId,
    pub account_id: Option<AccountId>,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
    pub revoked_reason: Option<String>,
}

impl SessionRecord {
    pub fn est_vivante(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at > OffsetDateTime::now_utc()
    }

    /// Une session révoquée **pour cause de rotation** dont le jeton se présente
    /// à nouveau n'a aucune explication innocente : le jeton a été volé, ou une
    /// copie de la session circule.
    pub fn est_un_rejeu(&self) -> bool {
        self.revoked_reason.as_deref() == Some(RevokeReason::Rotated.as_db())
    }
}

/// Rend la session **même révoquée** : c'est ce qui permet de distinguer un
/// jeton périmé d'un jeton rejoué.
pub async fn find_by_refresh_hash(
    pool: &PgPool,
    empreinte: &[u8],
) -> Result<Option<SessionRecord>> {
    let ligne = sqlx::query!(
        "SELECT id, person_id, account_id, expires_at, revoked_at, revoked_reason
           FROM identity.sessions
          WHERE refresh_token_hash = $1",
        empreinte
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| SessionRecord {
        id: SessionId(l.id),
        person_id: PersonId(l.person_id),
        account_id: l.account_id.map(AccountId),
        expires_at: l.expires_at,
        revoked_at: l.revoked_at,
        revoked_reason: l.revoked_reason,
    }))
}

/// Session vivante et personne en état de se connecter, en une seule lecture :
/// l'intergiciel la fait à chaque requête, et deux allers-retours par requête
/// feraient de la table de session le point chaud de la base.
pub async fn resolve_active(pool: &PgPool, session_id: SessionId) -> Result<Option<PersonId>> {
    let ligne = sqlx::query_scalar!(
        "SELECT s.person_id
           FROM identity.sessions s
           JOIN identity.people p ON p.id = s.person_id
          WHERE s.id = $1
            AND s.revoked_at IS NULL
            AND s.expires_at > now()
            AND p.status = 'active'",
        session_id.as_uuid()
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(PersonId))
}

pub async fn revoke(
    conn: &mut PgConnection,
    session_id: SessionId,
    motif: RevokeReason,
) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE identity.sessions
            SET revoked_at = now(), revoked_reason = $2
          WHERE id = $1 AND revoked_at IS NULL",
        session_id.as_uuid(),
        motif.as_db()
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Révocation en masse : compromission, changement de mot de passe, suspension.
pub async fn revoke_all(
    conn: &mut PgConnection,
    person_id: PersonId,
    motif: RevokeReason,
) -> Result<u64> {
    let touchees = sqlx::query!(
        "UPDATE identity.sessions
            SET revoked_at = now(), revoked_reason = $2
          WHERE person_id = $1 AND revoked_at IS NULL",
        person_id.as_uuid(),
        motif.as_db()
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees)
}

/// Sert les tests et le back-office : les sessions vivantes d'une personne.
pub async fn count_active(pool: &PgPool, person_id: Uuid) -> Result<i64> {
    let compte = sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!"
             FROM identity.sessions
            WHERE person_id = $1 AND revoked_at IS NULL AND expires_at > now()"#,
        person_id
    )
    .fetch_one(pool)
    .await?;

    Ok(compte)
}
