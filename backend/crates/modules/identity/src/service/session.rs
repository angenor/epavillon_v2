//! Ouverture, rotation, révocation d'une session.
//!
//! **La rotation chaîne des lignes** : chaque renouvellement révoque la session
//! courante avec le motif `rotated` et en ouvre une nouvelle. Présenter un jeton
//! dont la session porte déjà ce motif n'a aucune explication innocente — soit
//! il a été volé, soit une copie de la session circule — et fait révoquer
//! **toutes** les sessions vivantes de la personne (FR-031, research.md § R3).
//!
//! L'échéance ne glisse pas. La session neuve hérite de celle qu'elle remplace :
//! une durée repoussée à chaque renouvellement ferait des douze heures de
//! `FR-030` une session éternelle.

use kernel::context::RequestContext;
use kernel::crypto::{random_token, token_hash};
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;

use crate::domain::ids::{AccountId, PersonId, SessionId};
use crate::repo::sessions::{self, NewSession, RevokeReason};
use crate::state::IdentityState;

/// Ce que la connexion et le renouvellement rendent au client. Le jeton de
/// rafraîchissement en clair ne sort d'ici que pour entrer dans un cookie.
pub struct IssuedSession {
    pub session_id: SessionId,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: OffsetDateTime,
}

/// Deux jetons en clair : `Debug` ne les rend pas. Une trace de mise au point
/// qui les afficherait vaudrait une session volée dans les journaux.
impl std::fmt::Debug for IssuedSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IssuedSession")
            .field("session_id", &self.session_id)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Device<'a> {
    pub user_agent: Option<&'a str>,
    pub ip: Option<std::net::IpAddr>,
}

#[derive(Debug)]
pub enum RefreshOutcome {
    Renewed(Box<IssuedSession>),
    Expired,
}

/// Douze heures, ou trente jours avec « rester connecté » — les valeurs que le
/// site tient déjà (FR-030).
pub fn expiry(state: &IdentityState, remember_me: bool) -> OffsetDateTime {
    let duree = if remember_me {
        state.config().auth.session_ttl_remembered
    } else {
        state.config().auth.session_ttl
    };
    OffsetDateTime::now_utc() + time::Duration::seconds(duree.as_secs() as i64)
}

pub async fn open(
    state: &IdentityState,
    conn: &mut PgConnection,
    person_id: PersonId,
    account_id: Option<AccountId>,
    expires_at: OffsetDateTime,
    device: Device<'_>,
) -> Result<IssuedSession> {
    // Une collision sur 256 bits d'aléa n'arrivera pas ; le contrat demande
    // quand même qu'on régénère une fois avant d'abandonner.
    for _ in 0..2 {
        let jeton = random_token();
        let empreinte = token_hash(&jeton);

        let cree = sessions::create(
            conn,
            NewSession {
                person_id,
                account_id,
                refresh_token_hash: &empreinte,
                user_agent: device.user_agent,
                ip_address: device.ip,
                expires_at,
            },
        )
        .await?;

        if let Some(session_id) = cree {
            let access_token = state.tokens().issue(person_id, session_id)?;
            return Ok(IssuedSession {
                session_id,
                access_token,
                refresh_token: jeton,
                expires_at,
            });
        }
    }

    Err(ApiError::internal(
        "deux collisions d'empreinte de jeton de rafraîchissement d'affilée",
    ))
}

pub async fn refresh(
    state: &IdentityState,
    ctx: &RequestContext,
    refresh_token: &str,
    device: Device<'_>,
) -> Result<RefreshOutcome> {
    let empreinte = token_hash(refresh_token);
    let Some(session) = sessions::find_by_refresh_hash(state.pool(), &empreinte).await? else {
        return Ok(RefreshOutcome::Expired);
    };

    let contexte = ctx.with_actor(session.person_id.as_uuid());

    if session.est_un_rejeu() {
        let mut tx = state.db().write(&contexte).await?;
        let erreur = couper_tout_pour_rejeu(&mut tx, session.person_id).await?;
        tx.commit().await?;
        return Err(erreur);
    }

    if !session.est_vivante() {
        return Ok(RefreshOutcome::Expired);
    }

    let mut tx = state.db().write(&contexte).await?;

    // **La révocation est le verrou.** La session a été lue hors transaction ;
    // si l'UPDATE ne touche aucune ligne, un autre renouvellement l'a prise
    // entre-temps. Deux appels portant le même jeton, c'est la définition du
    // rejeu — R3 écarte explicitement toute fenêtre de tolérance —, et surtout
    // on ne peut pas en ouvrir deux : ce serait une session orpheline vivante,
    // née d'un jeton déjà consommé.
    if !sessions::revoke(&mut tx, session.id, RevokeReason::Rotated).await? {
        let erreur = couper_tout_pour_rejeu(&mut tx, session.person_id).await?;
        tx.commit().await?;
        return Err(erreur);
    }

    let neuve = open(
        state,
        &mut tx,
        session.person_id,
        session.account_id,
        session.expires_at,
        device,
    )
    .await?;
    tx.commit().await?;

    Ok(RefreshOutcome::Renewed(Box::new(neuve)))
}

/// Un jeton présenté deux fois n'a aucune explication innocente : soit il a été
/// volé, soit une copie de la session circule. On coupe tout, y compris les
/// appareils qui n'ont rien fait — ce sont eux que le vol vise ensuite.
async fn couper_tout_pour_rejeu(conn: &mut PgConnection, person_id: PersonId) -> Result<ApiError> {
    let coupees = sessions::revoke_all(conn, person_id, RevokeReason::ReuseDetected).await?;
    tracing::warn!(
        %person_id,
        sessions_revoquees = coupees,
        "jeton de rafraîchissement rejoué : toutes les sessions sont coupées"
    );
    Ok(ApiError::new(ErrorCode::IdentityRefreshReused))
}

/// Se déconnecter deux fois n'est pas une erreur : sans jeton, sans session
/// connue, ou sur une session déjà révoquée, la route réussit quand même.
pub async fn logout(
    state: &IdentityState,
    ctx: &RequestContext,
    refresh_token: Option<&str>,
) -> Result<()> {
    let Some(jeton) = refresh_token else {
        return Ok(());
    };
    let empreinte = token_hash(jeton);
    let Some(session) = sessions::find_by_refresh_hash(state.pool(), &empreinte).await? else {
        return Ok(());
    };

    let contexte = ctx.with_actor(session.person_id.as_uuid());
    let mut tx = state.db().write(&contexte).await?;
    sessions::revoke(&mut tx, session.id, RevokeReason::Logout).await?;
    tx.commit().await?;

    Ok(())
}

/// Compromission : tout couper, d'un coup.
pub async fn revoke_all(conn: &mut PgConnection, person_id: PersonId) -> Result<u64> {
    sessions::revoke_all(conn, person_id, RevokeReason::LogoutAll).await
}

/// Suspension, exclusion : les sessions cessent de valoir **dans la transaction
/// du changement de statut**, sans attendre leur échéance (FR-033). Appelée par
/// l'écriture qui change le statut, jamais après coup — un intervalle entre les
/// deux serait exactement la fenêtre qu'on ferme.
pub async fn cut_on_status_change(conn: &mut PgConnection, person_id: PersonId) -> Result<u64> {
    sessions::revoke_all(conn, person_id, RevokeReason::StatusChanged).await
}

/// Même règle pour un changement de mot de passe : les sessions ouvertes
/// ailleurs tombent avec l'ancien secret.
pub async fn cut_on_password_change(conn: &mut PgConnection, person_id: PersonId) -> Result<u64> {
    sessions::revoke_all(conn, person_id, RevokeReason::PasswordChanged).await
}
