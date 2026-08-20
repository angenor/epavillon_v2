//! Connexion : l'ordre des contrôles, et la discrétion.
//!
//! **Le mot de passe se vérifie en premier, et toujours** — même quand l'adresse
//! est inconnue, contre une empreinte factice calculée au démarrage. Un message
//! identique ne suffit pas à tenir la règle de discrétion : le temps parle
//! aussi. Abandonner avant de calculer Argon2id rend la réponse dix à cent fois
//! plus rapide, et le formulaire de connexion redevient l'annuaire des comptes
//! qu'on voulait fermer (FR-019, FR-020, research.md § R5).
//!
//! Verrou, suspension, adresse non vérifiée, second facteur : dans cet ordre, et
//! seulement après un mot de passe correct (FR-021). Chacun suppose l'identité
//! prouvée — les rendre plus tôt renseignerait qui ne connaît pas le mot de passe.
//!
//! **Le compteur d'échecs s'écrit PENDANT le hachage, pas après.** C'est la
//! seconde moitié de la même règle, et elle a été trouvée en la mesurant : la
//! base répond en une milliseconde là où le hachage en prend une dizaine, mais
//! une adresse connue écrit et une adresse inconnue n'écrit rien — l'écart se
//! lisait à la montre. Lancée avant le hachage, l'écriture disparaît entièrement
//! derrière lui. L'incrément est donc **optimiste** : si le mot de passe se
//! révèle juste, le compteur retombe, ce que FR-015 demande de toute façon.

use contracts::identity as evenements;
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self, DomainEvent};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::AccountId;
use crate::domain::login::{LoginOutcome, MfaMethod};
use crate::repo::{accounts, people};
use crate::service::session::{self, Device, IssuedSession};
use crate::state::IdentityState;

pub struct LoginRequest<'a> {
    pub email: &'a str,
    pub password: &'a str,
    pub remember_me: bool,
    pub device: Device<'a>,
}

pub struct LoginResponse {
    pub outcome: LoginOutcome,
    /// Renseignée pour la seule issue « authentifié » : aucune des cinq autres
    /// ne pose de cookie.
    pub session: Option<IssuedSession>,
}

impl LoginResponse {
    fn refus(outcome: LoginOutcome) -> Self {
        Self {
            outcome,
            session: None,
        }
    }
}

pub async fn login(
    state: &IdentityState,
    ctx: &RequestContext,
    requete: LoginRequest<'_>,
) -> Result<LoginResponse> {
    // Adresse inconnue, ou personne connue sans compte mot de passe : on paie
    // quand même le coût de vérification, et on rend la seule issue possible.
    let Some(candidat) = people::find_for_login(state.pool(), requete.email).await? else {
        verifier(state, requete.password, None).await?;
        return Ok(LoginResponse::refus(LoginOutcome::InvalidCredentials));
    };

    let (Some(account_id), Some(empreinte)) = (candidat.account_id, candidat.password_hash.clone())
    else {
        verifier(state, requete.password, None).await?;
        return Ok(LoginResponse::refus(LoginOutcome::InvalidCredentials));
    };

    let maintenant = OffsetDateTime::now_utc();
    let deja_verrouille = candidat.locked_until.is_some_and(|fin| fin > maintenant);

    // Les deux partent ensemble ; l'écriture se replie derrière le hachage.
    // L'acteur n'est pas encore prouvé : la transaction s'ouvre sans lui, et
    // c'est exact — on ne sait pas qui frappe à la porte.
    let hachage = lancer_verification(state, requete.password, Some(empreinte));
    let tentatives = {
        let mut tx = state.db().write(ctx).await?;
        let compte = accounts::bump_failed_attempts(&mut tx, account_id).await?;
        tx.commit().await?;
        compte
    };
    let correct = hachage.await.map_err(ApiError::internal)?;

    if !correct {
        if !deja_verrouille && i32::from(tentatives) >= state.config().auth.lockout_threshold as i32
        {
            verrouiller(state, ctx, &candidat, account_id).await?;
        }
        return Ok(LoginResponse::refus(LoginOutcome::InvalidCredentials));
    }

    // Le mot de passe est prouvé : l'acteur est connu à partir d'ici, et
    // l'incrément optimiste se défait (FR-015).
    let contexte = ctx.with_actor(candidat.person_id.as_uuid());
    let mut tx = state.db().write(&contexte).await?;
    accounts::clear_attempts(&mut tx, account_id).await?;
    tx.commit().await?;

    // Un verrou échu, lui, a déjà été purgé par l'incrément : le compte repart
    // avec un compteur neuf, quel que soit le verdict du mot de passe.
    if let Some(fin) = candidat.locked_until {
        if fin > maintenant {
            return Ok(LoginResponse::refus(LoginOutcome::Locked { until: fin }));
        }
    }

    if !candidat.status.peut_se_connecter() {
        return Ok(LoginResponse::refus(LoginOutcome::Suspended {
            until: candidat.suspended_until,
        }));
    }

    // Écart n° 20 : l'état « en attente de vérification » est porté par la seule
    // date de vérification. Aucun statut de personne n'est ajouté pour cela —
    // deux sources pour un même fait divergent toujours (FR-024).
    if candidat.email_verified_at.is_none() {
        return Ok(LoginResponse::refus(LoginOutcome::EmailUnverified {
            email: candidat.email.clone(),
        }));
    }

    // Emplacement réservé : aucune route ne complète le défi dans ce jalon.
    if candidat.mfa_enabled_at.is_some() {
        return Ok(LoginResponse::refus(LoginOutcome::MfaRequired {
            challenge_id: Uuid::now_v7(),
            method: MfaMethod::Totp,
        }));
    }

    let mut tx = state.db().write(&contexte).await?;
    accounts::mark_login(&mut tx, account_id).await?;
    let ouverte = session::open(
        state,
        &mut tx,
        candidat.person_id,
        Some(account_id),
        session::expiry(state, requete.remember_me),
        requete.device,
    )
    .await?;
    tx.commit().await?;

    let personne = people::view(state.pool(), candidat.person_id)
        .await?
        .ok_or_else(|| {
            ApiError::internal("personne disparue entre l'authentification et sa lecture")
        })?;

    Ok(LoginResponse {
        outcome: LoginOutcome::Authenticated {
            person: Box::new(personne),
        },
        session: Some(ouverte),
    })
}

/// `None` en empreinte : l'adresse est inconnue, on vérifie contre l'empreinte
/// factice du noyau. Le résultat est jeté ; seul le temps compte.
fn lancer_verification(
    state: &IdentityState,
    mot_de_passe: &str,
    empreinte: Option<String>,
) -> tokio::task::JoinHandle<bool> {
    let passwords = state.shared_passwords();
    let mot_de_passe = mot_de_passe.to_owned();

    tokio::task::spawn_blocking(move || match empreinte {
        Some(empreinte) => passwords.verify(&mot_de_passe, &empreinte),
        None => {
            passwords.burn(&mot_de_passe);
            false
        }
    })
}

async fn verifier(
    state: &IdentityState,
    mot_de_passe: &str,
    empreinte: Option<String>,
) -> Result<bool> {
    lancer_verification(state, mot_de_passe, empreinte)
        .await
        .map_err(ApiError::internal)
}

/// Le seuil atteint : le verrou est posé pour la durée configurée, et son
/// événement part dans la même transaction. Si un autre échec concurrent l'a
/// posé d'abord, la base le dit et rien n'est émis.
async fn verrouiller(
    state: &IdentityState,
    ctx: &RequestContext,
    candidat: &people::AuthCandidate,
    account_id: AccountId,
) -> Result<()> {
    let jusqu_a = OffsetDateTime::now_utc()
        + time::Duration::seconds(state.config().auth.lockout_duration.as_secs() as i64);

    let charge = serde_json::to_value(evenements::AccountLocked {
        person_id: candidat.person_id.as_uuid(),
        locked_until: jusqu_a,
    })
    .map_err(ApiError::internal)?;

    let mut tx = state.db().write(ctx).await?;
    if accounts::lock(&mut tx, account_id, jusqu_a).await? {
        events::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: evenements::AGGREGATE_SCHEMA,
                aggregate_type: evenements::AGGREGATE_ACCOUNT,
                aggregate_id: account_id.as_uuid(),
                event_type: evenements::ACCOUNT_LOCKED,
                payload: charge,
            },
        )
        .await?;
    }
    tx.commit().await?;

    Ok(())
}
