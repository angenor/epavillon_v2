//! Inscription, vérification d'adresse, renvoi de lien.
//!
//! **La réponse est invariable, adresse libre ou déjà prise** (FR-035) : c'est
//! le courriel envoyé qui diffère — un lien de vérification d'un côté, un
//! rappel « vous avez déjà un compte » de l'autre. Rendre une erreur de
//! conflit ferait du formulaire d'inscription l'annuaire des comptes que la
//! connexion refuse déjà d'être.
//!
//! **Le mot de passe est haché dans les deux cas**, y compris quand rien ne
//! sera créé. C'est la seconde moitié de la même règle, apprise en phase 3 :
//! un message identique ne suffit pas, le temps parle aussi. Ne hacher que sur
//! adresse libre rendrait l'inscription dix à cent fois plus rapide sur une
//! adresse déjà connue, et le formulaire redirait ce qu'on vient de taire.
//!
//! **Une personne connue mais SANS compte obtient un compte**, et non un rappel :
//! l'invitation par adresse crée une personne sans compte, et brancher sur la
//! seule existence de l'adresse la laissait sans issue. La réponse ne change pas
//! de forme pour autant — le formulaire d'inscription ne devient pas l'annuaire
//! des personnes.
//!
//! **Le jeton en clair et son travail d'envoi naissent dans la transaction du
//! changement d'état** (research.md § R8) : ni l'un ni l'autre ne survit à un
//! `ROLLBACK`, et l'événement de domaine, lui, ne porte aucun secret.

use contracts::identity as evenements;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::{self, DomainEvent};
use kernel::jobs::{self, NewJob};
use kernel::tokens::{self, TokenPurpose, TokenRejection};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::PersonId;
use crate::domain::password;
use crate::domain::token::{RegisterOutcome, ResendOutcome, VerifyEmailOutcome};
use crate::jobs::emails;
use crate::repo::people;
use crate::state::IdentityState;

pub struct RegisterRequest<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: &'a str,
    pub country_id: Option<Uuid>,
    pub password: &'a str,
    pub preferred_locale: &'a str,
    pub timezone: &'a str,
}

pub async fn register(
    state: &IdentityState,
    ctx: &RequestContext,
    demande: RegisterRequest<'_>,
) -> Result<RegisterOutcome> {
    // Le refus d'un mot de passe trop court est une faute de saisie visible à
    // l'écran : il ne divulgue rien et sort avant tout le reste.
    password::exiger(demande.password)?;

    let empreinte = hacher(state, demande.password).await?;

    let mut tx = state.db().write(ctx).await?;

    match people::find_by_email(&mut tx, demande.email).await? {
        // Connue AVEC un compte : rien n'est créé, un rappel part.
        Some(connue) if connue.has_password_account => {
            rappeler_le_compte_existant(&mut tx, &connue).await?;
        }
        // Connue SANS compte — une personne créée par une invitation. Elle
        // obtient son compte et son lien de vérification : sans cela, l'invitée
        // ne pourrait jamais s'inscrire et l'invitation resterait une moitié de
        // fonctionnalité (specs/002-organisations/research.md § R9).
        //
        // Ce n'est pas une brèche : l'adresse est prouvée par le lien avant que
        // le compte ne serve à quoi que ce soit, et une personne sans compte n'a
        // par définition aucun secret à voler.
        Some(sans_compte) => {
            match doter_dun_compte(state, &mut tx, &sans_compte, &empreinte).await {
                Ok(()) => {}
                // Deux inscriptions simultanées sur la même personne sans
                // compte : l'unicité tranche, et la perdante rend la même
                // réponse. Le lien est déjà parti par l'autre requête.
                Err(e) if e.code == ErrorCode::IdentityAccountAlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        None => match creer(state, &mut tx, &demande, &empreinte).await {
            Ok(()) => {}
            // Deux inscriptions simultanées sur la même adresse : l'unicité de
            // la base tranche, et la perdante rend **la même réponse** que la
            // gagnante. Le rappel de compte existant est déjà parti par l'autre
            // requête.
            Err(e) if e.code == ErrorCode::IdentityEmailAlreadyUsed => {}
            Err(e) => return Err(e),
        },
    }

    tx.commit().await?;
    Ok(RegisterOutcome::verification_sent(demande.email))
}

/// Vérification d'adresse. Le jeton est consommé **atomiquement** : deux clics
/// simultanés n'aboutissent qu'une fois, et le second lit « déjà utilisé ».
pub async fn verify_email(
    state: &IdentityState,
    ctx: &RequestContext,
    jeton: &str,
) -> Result<VerifyEmailOutcome> {
    let mut tx = state.db().write(ctx).await?;

    let consomme = match tokens::consume(&mut tx, jeton, TokenPurpose::EmailVerification).await? {
        Ok(consomme) => consomme,
        Err(refus) => return Ok(VerifyEmailOutcome::Rejected { reason: refus }),
    };

    let Some(person_id) = consomme.person_id.map(PersonId) else {
        return Ok(VerifyEmailOutcome::Rejected {
            reason: TokenRejection::Invalid,
        });
    };

    // `email_verified_at` porte à elle seule l'état « en attente de
    // vérification » : la poser une seconde fois ferait mentir la date, et
    // l'événement annoncerait un changement qui n'a pas eu lieu.
    let verifiee = people::mark_email_verified(&mut tx, person_id).await?;
    let adresse = people::email_of(&mut tx, person_id).await?;

    if let Some(instant) = verifiee {
        events::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: evenements::AGGREGATE_SCHEMA,
                aggregate_type: evenements::AGGREGATE_PERSON,
                aggregate_id: person_id.as_uuid(),
                event_type: evenements::PERSON_EMAIL_VERIFIED,
                payload: serde_json::to_value(evenements::PersonEmailVerified {
                    person_id: person_id.as_uuid(),
                    verified_at: instant,
                })
                .map_err(ApiError::internal)?,
            },
        )
        .await?;
    }

    tx.commit().await?;

    Ok(match adresse {
        Some(email) => VerifyEmailOutcome::Verified { email },
        None => VerifyEmailOutcome::Rejected {
            reason: TokenRejection::Invalid,
        },
    })
}

/// Renvoi du lien. **Issue unique** : une adresse inconnue, déjà vérifiée ou en
/// attente rendent toutes `sent`. Distinguer les trois cas dirait au premier
/// venu si une adresse est inscrite, et si son propriétaire a cliqué.
pub async fn resend_verification(
    state: &IdentityState,
    ctx: &RequestContext,
    email: &str,
) -> Result<ResendOutcome> {
    let mut tx = state.db().write(ctx).await?;

    if let Some(personne) = people::find_by_email(&mut tx, email).await? {
        if personne.email_verified_at.is_none() {
            envoyer_la_verification(state, &mut tx, &personne).await?;
        }
    }

    tx.commit().await?;
    Ok(ResendOutcome::sent())
}

// -----------------------------------------------------------------------------

async fn creer(
    state: &IdentityState,
    tx: &mut PgConnection,
    demande: &RegisterRequest<'_>,
    empreinte: &str,
) -> Result<()> {
    let personne = people::create(
        tx,
        people::NewPerson {
            first_name: demande.first_name,
            last_name: demande.last_name,
            email: demande.email,
            country_id: demande.country_id,
            preferred_locale: demande.preferred_locale,
            timezone: demande.timezone,
        },
    )
    .await?;

    people::create_password_account(tx, personne.person_id, empreinte).await?;
    envoyer_la_verification(state, tx, &personne).await?;

    events::emit(
        tx,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_PERSON,
            aggregate_id: personne.person_id.as_uuid(),
            event_type: evenements::PERSON_REGISTERED,
            payload: serde_json::to_value(evenements::PersonRegistered {
                person_id: personne.person_id.as_uuid(),
                preferred_locale: personne.preferred_locale.clone(),
                country_id: demande.country_id,
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;

    Ok(())
}

/// Une personne connue mais sans compte : on lui en crée un, et le lien de
/// vérification part comme pour une inscription ordinaire.
async fn doter_dun_compte(
    state: &IdentityState,
    tx: &mut PgConnection,
    personne: &people::RegistrationTarget,
    empreinte: &str,
) -> Result<()> {
    people::create_password_account(tx, personne.person_id, empreinte).await?;
    envoyer_la_verification(state, tx, personne).await
}

/// Crée le jeton et met le courriel en file, **dans la transaction en cours**.
///
/// Les jetons de vérification encore en vie sont invalidés d'abord (FR-040) :
/// deux liens valides pour la même adresse, c'est une surface d'attaque de plus
/// pour aucun service rendu.
async fn envoyer_la_verification(
    state: &IdentityState,
    tx: &mut PgConnection,
    personne: &people::RegistrationTarget,
) -> Result<()> {
    tokens::invalidate_pending(
        tx,
        personne.person_id.as_uuid(),
        TokenPurpose::EmailVerification,
    )
    .await?;

    let jeton = tokens::create(
        tx,
        &state.config().auth.token_ttl,
        personne.person_id.as_uuid(),
        TokenPurpose::EmailVerification,
        json!({ "email": personne.email }),
    )
    .await?;

    // La clé d'unicité porte l'identifiant du jeton : deux demandes du même
    // courriel n'en envoient qu'un.
    jobs::enqueue(
        tx,
        NewJob::new(
            emails::SEND_VERIFICATION_EMAIL,
            json!({
                "to": personne.email,
                "locale": personne.preferred_locale,
                "first_name": personne.first_name,
                "token": jeton.clear,
            }),
        )
        .idempotent(jeton.id.to_string()),
    )
    .await?;

    Ok(())
}

/// L'autre moitié de la réponse invariable : rien n'est créé, rien n'est
/// modifié, et un rappel part — **sans lien de vérification**. Un tiers ne doit
/// pas pouvoir provoquer l'envoi d'un lien vers un compte qui n'est pas le sien.
///
/// La clé d'unicité porte la personne **et le jour** : réessayer dix fois dans
/// l'après-midi n'inonde pas la boîte de quelqu'un, et le rappel repart demain.
async fn rappeler_le_compte_existant(
    tx: &mut PgConnection,
    personne: &people::RegistrationTarget,
) -> Result<()> {
    let jour = OffsetDateTime::now_utc().date();

    jobs::enqueue(
        tx,
        NewJob::new(
            emails::SEND_EXISTING_ACCOUNT_NOTICE,
            json!({
                "to": personne.email,
                "locale": personne.preferred_locale,
                "first_name": personne.first_name,
            }),
        )
        .idempotent(format!("{}:{jour}", personne.person_id)),
    )
    .await?;

    Ok(())
}

/// Le hachage part sur un fil dédié : cinquante à cent millisecondes de calcul
/// sur le fil du réacteur y bloqueraient toutes les autres requêtes.
async fn hacher(state: &IdentityState, mot_de_passe: &str) -> Result<String> {
    let passwords = state.shared_passwords();
    let mot_de_passe = mot_de_passe.to_owned();

    tokio::task::spawn_blocking(move || passwords.hash(&mot_de_passe))
        .await
        .map_err(ApiError::internal)?
}
