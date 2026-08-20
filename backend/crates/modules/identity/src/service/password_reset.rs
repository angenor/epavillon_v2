//! Réinitialisation du mot de passe : la demande, le contrôle, l'enregistrement.
//!
//! **La demande rend toujours `sent`** (FR-036). Distinguer l'adresse connue de
//! l'inconnue ferait du formulaire « mot de passe oublié » l'annuaire des
//! comptes que la connexion refuse d'être — et il suffirait d'une boucle sur un
//! carnet d'adresses pour le lire.
//!
//! **Ce que la réponse tait, le temps ne le dit pas non plus — dans la mesure du
//! possible.** À l'inscription, l'écart tenait au hachage, dix à cent
//! millisecondes qu'on paie donc des deux côtés. Ici l'écart est de tout autre
//! nature : trois écritures brèves contre une lecture. Il ne se comble pas de la
//! même façon — écrire un jeton factice pour une adresse inconnue mettrait en
//! file un courriel vers un destinataire qui n'existe pas, et le remède serait
//! pire. Ce qui est fait à la place : la transaction d'écriture est ouverte
//! **avant** de savoir si l'adresse est connue, de sorte que les deux chemins
//! paient le même aller-retour d'ouverture et de validation, et que l'écart
//! restant tienne aux seuls trois `INSERT`. Quelques millisecondes sur un
//! réseau public ne se distinguent pas ; les cinquante d'Argon2id, si.
//!
//! **Le jeton est revérifié à l'enregistrement** (FR-042). Le contrôle préalable
//! sert à ne pas faire composer un mot de passe pour rien ; il ne vaut aucune
//! garantie. Un onglet ouvert la veille au soir et validé le lendemain matin est
//! un cas ordinaire, pas une bizarrerie.

use contracts::identity as evenements;
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self, DomainEvent};
use kernel::jobs::{self, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;

use crate::domain::password;
use crate::domain::token::{
    PasswordResetOutcome, PasswordResetRequestOutcome, TokenCheckOutcome, TokenPurpose,
    TokenRejection,
};
use crate::jobs::emails;
use crate::repo::{accounts, people, tokens};
use crate::service::session;
use crate::state::IdentityState;

/// Demande de lien. Le statut de la personne n'entre pas dans la décision : un
/// compte suspendu ne se reconnecte pas de toute façon (FR-024, FR-033), et le
/// filtrer ici ferait varier le courriel reçu selon un état qu'on ne divulgue
/// pas.
pub async fn request(
    state: &IdentityState,
    ctx: &RequestContext,
    email: &str,
) -> Result<PasswordResetRequestOutcome> {
    let mut tx = state.db().write(ctx).await?;

    if let Some(personne) = people::find_by_email(&mut tx, email).await? {
        envoyer_le_lien(state, &mut tx, &personne).await?;
    }

    tx.commit().await?;
    Ok(PasswordResetRequestOutcome::sent())
}

/// Contrôle **sans consommer**, pour décider d'afficher le formulaire. L'adresse
/// revient avec la réponse valide : l'écran dit quel compte est en train d'être
/// repris, ce qui n'apprend rien à qui tient déjà le lien.
pub async fn check(state: &IdentityState, jeton: &str) -> Result<TokenCheckOutcome> {
    let person_id = match tokens::check(state.pool(), jeton, TokenPurpose::PasswordReset).await? {
        Ok(id) => id,
        Err(refus) => return Ok(TokenCheckOutcome::Rejected { reason: refus }),
    };

    let mut conn = state.pool().acquire().await?;
    Ok(match people::email_of(&mut conn, person_id).await? {
        Some(email) => TokenCheckOutcome::Valid { email },
        // La personne a été effacée entre l'envoi du lien et le clic : le jeton
        // ne mène plus nulle part, et « invalide » est le seul refus honnête.
        None => TokenCheckOutcome::Rejected {
            reason: TokenRejection::Invalid,
        },
    })
}

/// Enregistrement du nouveau mot de passe.
///
/// L'ordre compte : le mot de passe est jugé **avant** que le jeton ne soit
/// consommé. Une saisie trop courte ne doit pas brûler le lien — la personne
/// corrige et renvoie, sans repasser par sa boîte aux lettres.
pub async fn confirm(
    state: &IdentityState,
    ctx: &RequestContext,
    jeton: &str,
    mot_de_passe: &str,
) -> Result<PasswordResetOutcome> {
    password::exiger(mot_de_passe)?;
    let empreinte = hacher(state, mot_de_passe).await?;

    let mut tx = state.db().write(ctx).await?;

    let consomme = match tokens::consume(&mut tx, jeton, TokenPurpose::PasswordReset).await? {
        Ok(consomme) => consomme,
        Err(refus) => return Ok(PasswordResetOutcome::Rejected { reason: refus }),
    };

    let Some(person_id) = consomme.person_id else {
        return Ok(PasswordResetOutcome::Rejected {
            reason: TokenRejection::Invalid,
        });
    };

    // L'acteur ne se connaissait pas à l'ouverture : la personne n'a pas de
    // session, et son identifiant sort du jeton qu'on vient de consommer. On le
    // pose ici, avant les écritures — sinon l'événement porterait un acteur nul
    // pour un changement qu'elle a bel et bien fait.
    kernel::db::set_actor(&mut tx, person_id.as_uuid()).await?;

    let account_id = accounts::set_password(&mut tx, person_id, &empreinte).await?;
    session::cut_on_password_change(&mut tx, person_id).await?;

    events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_ACCOUNT,
            aggregate_id: account_id.as_uuid(),
            event_type: evenements::ACCOUNT_PASSWORD_CHANGED,
            payload: serde_json::to_value(evenements::AccountPasswordChanged {
                person_id: person_id.as_uuid(),
                channel: evenements::PasswordChangeChannel::Reset,
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;

    let adresse = people::email_of(&mut tx, person_id).await?;
    tx.commit().await?;

    Ok(match adresse {
        Some(email) => PasswordResetOutcome::Reset { email },
        None => PasswordResetOutcome::Rejected {
            reason: TokenRejection::Invalid,
        },
    })
}

// -----------------------------------------------------------------------------

/// Crée le jeton et met le courriel en file, **dans la transaction en cours** :
/// ni l'un ni l'autre ne survit à un `ROLLBACK`.
///
/// Les liens de réinitialisation encore en vie tombent d'abord (FR-040) : deux
/// liens valides pour le même compte, c'est une surface d'attaque de plus pour
/// aucun service rendu. Qui clique l'ancien lit « périmé », ce qui est le
/// message juste — un plus récent vient d'arriver.
async fn envoyer_le_lien(
    state: &IdentityState,
    tx: &mut PgConnection,
    personne: &people::RegistrationTarget,
) -> Result<()> {
    tokens::invalidate_pending(tx, personne.person_id, TokenPurpose::PasswordReset).await?;

    let jeton = tokens::create(
        tx,
        &state.config().auth.token_ttl,
        personne.person_id,
        TokenPurpose::PasswordReset,
        json!({ "email": personne.email }),
    )
    .await?;

    jobs::enqueue(
        tx,
        NewJob::new(
            emails::SEND_PASSWORD_RESET_EMAIL,
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

/// Le hachage part sur un fil dédié : cinquante à cent millisecondes de calcul
/// sur le fil du réacteur y bloqueraient toutes les autres requêtes.
async fn hacher(state: &IdentityState, mot_de_passe: &str) -> Result<String> {
    let passwords = state.shared_passwords();
    let mot_de_passe = mot_de_passe.to_owned();

    tokio::task::spawn_blocking(move || passwords.hash(&mot_de_passe))
        .await
        .map_err(ApiError::internal)?
}
