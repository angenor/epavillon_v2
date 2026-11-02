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
    /// Écrit par la reprise d'ADR-020, plus depuis le 02/11/2026 (ADR-024) : les
    /// lignes qui le portent restent lues, et leur jeton qui revient est un vol.
    ResponseLost,
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
            Self::ResponseLost => "response_lost",
            Self::Logout => "logout",
            Self::LogoutAll => "logout_all",
            Self::ReuseDetected => "reuse_detected",
            Self::PasswordChanged => "password_changed",
            Self::StatusChanged => "status_changed",
        }
    }
}

/// D'où vient la session. ENUM fermé côté base (`identity.session_client`), et
/// l'API n'en accepte pas d'autre valeur — un `kind` inconnu est refusé, jamais
/// « corrigé » en silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClientKind {
    #[default]
    Web,
    App,
}

impl ClientKind {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::App => "app",
        }
    }

    /// `None` sur une valeur inconnue : c'est l'appelant qui décide du refus, et
    /// il désigne alors le champ fautif.
    pub fn parse(valeur: &str) -> Option<Self> {
        match valeur {
            "web" => Some(Self::Web),
            "app" => Some(Self::App),
            _ => None,
        }
    }

    pub fn est_app(self) -> bool {
        self == Self::App
    }
}

pub struct NewSession<'a> {
    pub person_id: PersonId,
    pub account_id: Option<AccountId>,
    pub refresh_token_hash: &'a [u8],
    pub user_agent: Option<&'a str>,
    pub ip_address: Option<std::net::IpAddr>,
    pub client_kind: ClientKind,
    pub device_id: Option<&'a str>,
    pub device_label: Option<&'a str>,
    pub device_platform: Option<&'a str>,
    pub expires_at: OffsetDateTime,
}

/// `None` signale une collision d'empreinte : l'appelant régénère et rejoue une
/// fois. `ON CONFLICT DO NOTHING` plutôt qu'une erreur rattrapée, parce qu'un
/// échec d'insertion avorterait la transaction entière — et l'ouverture de
/// session partage la sienne avec la mise à jour du compte.
pub async fn create(conn: &mut PgConnection, session: NewSession<'_>) -> Result<Option<SessionId>> {
    let id = sqlx::query_scalar!(
        "INSERT INTO identity.sessions
             (person_id, account_id, refresh_token_hash, user_agent, ip_address,
              client_kind, device_id, device_label, device_platform, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6::text::identity.session_client, $7, $8, $9, $10)
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
        session.client_kind.as_db(),
        session.device_id,
        session.device_label,
        session.device_platform,
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
    /// **Relu pour être RECOPIÉ par la rotation.** Sans lui, toute session de
    /// l'application redeviendrait « web » au premier renouvellement : rien
    /// n'échouerait, et le compte des personnes qui utilisent l'application
    /// mentirait sans que personne ne s'en aperçoive.
    pub client_kind: ClientKind,
    pub device_id: Option<String>,
    pub device_label: Option<String>,
    pub device_platform: Option<String>,
    /// La dernière session ouverte depuis le jeton de celle-ci.
    pub replaced_by: Option<SessionId>,
}

/// Rend la session **même révoquée** : c'est ce qui permet de distinguer un
/// jeton périmé d'un jeton rejoué. Lue hors transaction, puis relue dans celle
/// du renouvellement quand un autre onglet vient de la tourner.
pub async fn find_by_refresh_hash<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    empreinte: &[u8],
) -> Result<Option<SessionRecord>> {
    let ligne = sqlx::query!(
        r#"SELECT id, person_id, account_id, expires_at, revoked_at, revoked_reason,
                  client_kind::text AS "client_kind!", device_id, device_label, device_platform,
                  replaced_by
             FROM identity.sessions
            WHERE refresh_token_hash = $1"#,
        empreinte
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| SessionRecord {
        id: SessionId(l.id),
        person_id: PersonId(l.person_id),
        account_id: l.account_id.map(AccountId),
        expires_at: l.expires_at,
        revoked_at: l.revoked_at,
        revoked_reason: l.revoked_reason,
        // Une valeur que la base rendrait et que l'API ne connaîtrait pas
        // retomberait sur « web » : c'est le seul repli qui ne donne rien de
        // plus que ce qui est déjà accordé.
        client_kind: ClientKind::parse(&l.client_kind).unwrap_or_default(),
        device_id: l.device_id,
        device_label: l.device_label,
        device_platform: l.device_platform,
        replaced_by: l.replaced_by.map(SessionId),
    }))
}

/// Nomme la dernière session ouverte depuis le jeton de `remplacee`.
pub async fn set_replaced_by(
    conn: &mut PgConnection,
    remplacee: SessionId,
    remplacante: SessionId,
) -> Result<()> {
    sqlx::query!(
        "UPDATE identity.sessions SET replaced_by = $2 WHERE id = $1",
        remplacee.as_uuid(),
        remplacante.as_uuid()
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// La lignée ouverte depuis une session tournée vit-elle encore ? On suit
/// `replaced_by` tant que le maillon a été tourné ; le bout est vivant, ou il a
/// été fermé (déconnexion, mot de passe changé, suspension, vol détecté).
/// Bornée à 32 maillons : dans la minute de tolérance, on en voit deux ou trois.
pub async fn lineage_alive(conn: &mut PgConnection, tournee: SessionId) -> Result<bool> {
    let vivante = sqlx::query_scalar!(
        r#"WITH RECURSIVE lignee AS (
               SELECT id, replaced_by, revoked_at, revoked_reason, expires_at, 0 AS rang
                 FROM identity.sessions
                WHERE id = $1
               UNION ALL
               SELECT s.id, s.replaced_by, s.revoked_at, s.revoked_reason, s.expires_at, l.rang + 1
                 FROM lignee l
                 JOIN identity.sessions s ON s.id = l.replaced_by
                WHERE l.revoked_reason = $2 AND l.rang < 32
           )
           SELECT EXISTS (
               SELECT 1 FROM lignee WHERE revoked_at IS NULL AND expires_at > now()
           ) AS "vivante!""#,
        tournee.as_uuid(),
        RevokeReason::Rotated.as_db()
    )
    .fetch_one(conn)
    .await?;

    Ok(vivante)
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

/// Ce que `GET /auth/me` ajoute à la personne : de quel appareil la session
/// courante vient, et depuis quand. Une requête de plus n'est pas nécessaire —
/// l'identifiant de session sort déjà du jeton d'accès.
#[derive(Debug, Clone)]
pub struct CurrentSession {
    pub client_kind: ClientKind,
    pub device_label: Option<String>,
    pub issued_at: OffsetDateTime,
}

pub async fn describe(pool: &PgPool, session_id: SessionId) -> Result<Option<CurrentSession>> {
    let ligne = sqlx::query!(
        r#"SELECT client_kind::text AS "client_kind!", device_label, issued_at
             FROM identity.sessions
            WHERE id = $1"#,
        session_id.as_uuid()
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| CurrentSession {
        client_kind: ClientKind::parse(&l.client_kind).unwrap_or_default(),
        device_label: l.device_label,
        issued_at: l.issued_at,
    }))
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
