//! Ouverture, rotation, révocation d'une session.
//!
//! **La rotation chaîne des lignes** : chaque renouvellement révoque la session
//! courante avec le motif `rotated` et en ouvre une nouvelle. Un jeton tourné
//! qui revient **au-delà de `AUTH_REFRESH_GRACE`** (60 s) a été volé ou copié,
//! et fait révoquer **toutes** les sessions vivantes de la personne (FR-031,
//! research.md § R3).
//!
//! **En deçà, ce sont des onglets** (ADR-024, qui remplace la reprise
//! d'ADR-020). Ils partagent les cookies : deux qui rafraîchissent ensemble
//! présentent le même jeton, et l'ordre d'arrivée des réponses décide lequel
//! reste dans le navigateur. Le jeton revenu dans la minute ouvre donc une
//! session sœur, **sans fermer la première** : fermer l'une ou l'autre
//! déconnectait l'onglet dont la réponse arrivait en dernier. Une lignée close
//! entre-temps (déconnexion, mot de passe changé, suspension) ne se rouvre pas.
//!
//! L'échéance ne glisse pas **pour le site**. La session neuve hérite de celle
//! qu'elle remplace : une durée repoussée à chaque renouvellement ferait des
//! douze heures de `FR-030` une session éternelle.
//!
//! **L'application, elle, glisse — et c'est délibéré.** Une session ouverte
//! depuis Guide Négo repart de quatre-vingt-dix jours à chaque renouvellement :
//! une durée fixe ferait expirer au milieu de la COP un compte ouvert des
//! semaines plus tôt, et la personne n'aurait alors ni réseau ni patience pour
//! se reconnecter. Ce qui s'éteint, c'est un téléphone qui n'a pas ouvert
//! l'application depuis trois mois.

use kernel::context::RequestContext;
use kernel::crypto::{random_token, token_hash};
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;

use crate::domain::ids::{AccountId, PersonId, SessionId};
use crate::repo::sessions::{self, ClientKind, NewSession, RevokeReason, SessionRecord};
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

/// Ce que le client déclare de lui-même. **Rien ici n'accorde de droit** :
/// `device_id` est engendré par le client et se forge, au même titre que
/// `user_agent`. Il sert à nommer une session dans une liste, jamais à autoriser.
#[derive(Debug, Clone, Copy, Default)]
pub struct Device<'a> {
    pub user_agent: Option<&'a str>,
    pub ip: Option<std::net::IpAddr>,
    pub client_kind: ClientKind,
    pub device_id: Option<&'a str>,
    pub device_label: Option<&'a str>,
    pub device_platform: Option<&'a str>,
}

#[derive(Debug)]
pub enum RefreshOutcome {
    Renewed(Box<IssuedSession>),
    Expired,
}

/// Douze heures, ou trente jours avec « rester connecté » — les valeurs que le
/// site tient déjà (FR-030) —, et **quatre-vingt-dix jours depuis l'application,
/// sans case à cocher** : son écran de connexion n'en a pas, et il n'y a rien à
/// demander à quelqu'un qui installe une application sur son propre téléphone.
///
/// `remember_me` est **ignoré** pour l'application : la durée longue est déjà la
/// sienne, et lire la case ferait croire qu'elle change quelque chose.
pub fn expiry(state: &IdentityState, client: ClientKind, remember_me: bool) -> OffsetDateTime {
    let duree = match (client, remember_me) {
        (ClientKind::App, _) => state.config().auth.session_ttl_app,
        (ClientKind::Web, true) => state.config().auth.session_ttl_remembered,
        (ClientKind::Web, false) => state.config().auth.session_ttl,
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
                client_kind: device.client_kind,
                device_id: device.device_id,
                device_label: device.device_label,
                device_platform: device.device_platform,
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

    let grace = state.config().auth.refresh_grace;
    if lire(&session, grace, OffsetDateTime::now_utc()) == Lecture::Close {
        return Ok(RefreshOutcome::Expired);
    }

    let contexte = ctx.with_actor(session.person_id.as_uuid());
    let mut tx = state.db().write(&contexte).await?;
    let issue = trancher(state, &mut tx, session, &empreinte, device).await?;
    // Validée aussi sur un rejeu : la coupure de toutes les sessions doit rester.
    tx.commit().await?;

    match issue {
        Issue::Renouvelee(neuve) => Ok(RefreshOutcome::Renewed(Box::new(neuve))),
        Issue::Close => Ok(RefreshOutcome::Expired),
        Issue::Rejeu(erreur) => Err(erreur),
    }
}

/// Ce que vaut un jeton présenté, d'après la seule ligne de sa session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lecture {
    /// À tourner.
    Vivante,
    /// Tournée dans la fenêtre de tolérance : un autre onglet, ou une réponse
    /// perdue. Une sœur s'ouvre si la lignée vit encore.
    Soeur,
    Rejeu,
    /// Périmée, ou fermée pour un autre motif que la rotation.
    Close,
}

fn lire(
    session: &SessionRecord,
    grace: std::time::Duration,
    maintenant: OffsetDateTime,
) -> Lecture {
    let echue = session.expires_at <= maintenant;
    let Some(revoquee) = session.revoked_at else {
        return if echue {
            Lecture::Close
        } else {
            Lecture::Vivante
        };
    };
    match session.revoked_reason.as_deref() {
        Some(motif) if motif == RevokeReason::Rotated.as_db() => {
            let tolerance = time::Duration::seconds(grace.as_secs() as i64);
            if grace.is_zero() || maintenant - revoquee >= tolerance {
                Lecture::Rejeu
            } else if echue {
                Lecture::Close
            } else {
                Lecture::Soeur
            }
        }
        // La remplaçante qu'ADR-020 écartait n'a jamais été remise au navigateur.
        Some(motif) if motif == RevokeReason::ResponseLost.as_db() => Lecture::Rejeu,
        _ => Lecture::Close,
    }
}

enum Issue {
    Renouvelee(IssuedSession),
    Close,
    Rejeu(ApiError),
}

async fn trancher(
    state: &IdentityState,
    conn: &mut PgConnection,
    mut session: SessionRecord,
    empreinte: &[u8],
    device: Device<'_>,
) -> Result<Issue> {
    let grace = state.config().auth.refresh_grace;
    // Deux passes : la seconde relit la ligne qu'un renouvellement concurrent
    // vient de tourner, une fois sa transaction validée.
    for _ in 0..2 {
        match lire(&session, grace, OffsetDateTime::now_utc()) {
            Lecture::Close => return Ok(Issue::Close),
            Lecture::Rejeu => {
                return Ok(Issue::Rejeu(
                    couper_tout_pour_rejeu(conn, session.person_id).await?,
                ))
            }
            Lecture::Soeur => return ouvrir_une_soeur(state, conn, &session, device).await,
            Lecture::Vivante => {
                // **La révocation est le verrou** : l'UPDATE attend la transaction
                // concurrente, et ne touche rien si elle a tourné la session.
                if sessions::revoke(conn, session.id, RevokeReason::Rotated).await? {
                    let neuve = remplacer(state, conn, &session, device).await?;
                    return Ok(Issue::Renouvelee(neuve));
                }
                match sessions::find_by_refresh_hash(&mut *conn, empreinte).await? {
                    Some(relue) => session = relue,
                    None => return Ok(Issue::Close),
                }
            }
        }
    }
    Err(ApiError::internal(
        "session vivante à la relecture, et pourtant déjà révoquée",
    ))
}

/// Le jeton d'une session tournée revient dans la minute : un onglet a
/// rafraîchi en même temps qu'un autre, ou la réponse s'est perdue. Une session
/// sœur s'ouvre ; celle déjà émise reste vivante, car c'est peut-être son
/// cookie que le navigateur garde.
async fn ouvrir_une_soeur(
    state: &IdentityState,
    conn: &mut PgConnection,
    session: &SessionRecord,
    device: Device<'_>,
) -> Result<Issue> {
    if !sessions::lineage_alive(conn, session.id).await? {
        return Ok(Issue::Close);
    }
    let neuve = remplacer(state, conn, session, device).await?;
    tracing::info!(
        person_id = %session.person_id,
        "jeton tourné représenté dans la tolérance : session sœur ouverte"
    );
    Ok(Issue::Renouvelee(neuve))
}

/// Ouvre la session qui remplace `session`, et la nomme sur elle.
async fn remplacer(
    state: &IdentityState,
    conn: &mut PgConnection,
    session: &SessionRecord,
    device: Device<'_>,
) -> Result<IssuedSession> {
    // **La rotation recopie le client et l'appareil de la session remplacée**,
    // et n'écoute pas le client sur ce point : `POST /auth/refresh` ne porte
    // aucun objet `client`, et s'il en portait un, il suffirait de l'envoyer
    // pour changer d'appareil déclaré. Sans cette recopie, toute session de
    // l'application redeviendrait « web » au premier renouvellement — sans
    // qu'aucune erreur ne le dise.
    let herite = Device {
        user_agent: device.user_agent,
        ip: device.ip,
        client_kind: session.client_kind,
        device_id: session.device_id.as_deref(),
        device_label: session.device_label.as_deref(),
        device_platform: session.device_platform.as_deref(),
    };

    // L'échéance de l'application **repart** ; celle du site est héritée.
    let echeance = if session.client_kind.est_app() {
        expiry(state, session.client_kind, false)
    } else {
        session.expires_at
    };

    let neuve = open(
        state,
        conn,
        session.person_id,
        session.account_id,
        echeance,
        herite,
    )
    .await?;
    sessions::set_replaced_by(conn, session.id, neuve.session_id).await?;
    Ok(neuve)
}

/// Un jeton tourné qui revient après la tolérance n'a aucune explication
/// innocente : soit il a été volé, soit une copie de la session circule. On
/// coupe tout, y compris les appareils qui n'ont rien fait : ce sont eux que le
/// vol vise ensuite.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use uuid::Uuid;

    const GRACE: Duration = Duration::from_secs(60);

    fn session(revoquee_il_y_a: Option<i64>, motif: Option<&str>, echue: bool) -> SessionRecord {
        let maintenant = OffsetDateTime::now_utc();
        SessionRecord {
            id: SessionId(Uuid::nil()),
            person_id: PersonId(Uuid::nil()),
            account_id: None,
            expires_at: maintenant + time::Duration::hours(if echue { -1 } else { 12 }),
            revoked_at: revoquee_il_y_a.map(|s| maintenant - time::Duration::seconds(s)),
            revoked_reason: motif.map(str::to_owned),
            client_kind: ClientKind::Web,
            device_id: None,
            device_label: None,
            device_platform: None,
            replaced_by: None,
        }
    }

    fn lue(s: &SessionRecord, grace: Duration) -> Lecture {
        lire(s, grace, OffsetDateTime::now_utc())
    }

    #[test]
    fn une_session_vivante_se_tourne() {
        assert_eq!(lue(&session(None, None, false), GRACE), Lecture::Vivante);
    }

    #[test]
    fn une_session_echue_est_close() {
        assert_eq!(lue(&session(None, None, true), GRACE), Lecture::Close);
    }

    #[test]
    fn un_jeton_tourne_dans_la_minute_ouvre_une_soeur() {
        assert_eq!(
            lue(&session(Some(0), Some("rotated"), false), GRACE),
            Lecture::Soeur
        );
        assert_eq!(
            lue(&session(Some(59), Some("rotated"), false), GRACE),
            Lecture::Soeur
        );
    }

    #[test]
    fn un_jeton_tourne_apres_la_minute_est_un_rejeu() {
        assert_eq!(
            lue(&session(Some(61), Some("rotated"), false), GRACE),
            Lecture::Rejeu
        );
        assert_eq!(
            lue(&session(Some(3600), Some("rotated"), false), GRACE),
            Lecture::Rejeu
        );
    }

    #[test]
    fn une_tolerance_nulle_rend_la_regle_stricte() {
        assert_eq!(
            lue(&session(Some(0), Some("rotated"), false), Duration::ZERO),
            Lecture::Rejeu
        );
    }

    #[test]
    fn un_jeton_tourne_dans_la_minute_mais_echu_est_clos() {
        assert_eq!(
            lue(&session(Some(5), Some("rotated"), true), GRACE),
            Lecture::Close
        );
    }

    #[test]
    fn la_remplacante_ecartee_par_adr_020_reste_un_vol() {
        assert_eq!(
            lue(&session(Some(5), Some("response_lost"), false), GRACE),
            Lecture::Rejeu
        );
    }

    #[test]
    fn une_session_fermee_pour_un_autre_motif_est_close() {
        for motif in [
            "logout",
            "logout_all",
            "reuse_detected",
            "password_changed",
            "status_changed",
        ] {
            assert_eq!(
                lue(&session(Some(5), Some(motif), false), GRACE),
                Lecture::Close,
                "{motif}"
            );
        }
    }
}
