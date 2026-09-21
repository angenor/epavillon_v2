//! Le lien du courriel ramène là d'où la demande vient.
//!
//! **Le défaut corrigé** : une personne qui crée son compte dans Guide Négo
//! ouvre son courriel, touche le lien, et se retrouve sur l'ePavillon — autre
//! apparence, autre logique, et aucun chemin de retour. Le parcours commencé est
//! perdu.
//!
//! Le client se retient **avec le jeton**, jamais avec une session : au moment
//! où le lien s'ouvre, la personne n'en a pas encore.

mod commun;

use commun::{semer, Bac, Compte, MOT_DE_PASSE};
use identity::repo::sessions::ClientKind;
use identity::service::registration::RegisterRequest;
use identity::service::{password_reset, registration};

const ADRESSE: &str = "awa.diallo@example.org";

fn demande(client: ClientKind) -> RegisterRequest<'static> {
    RegisterRequest {
        first_name: "Awa",
        last_name: "Diallo",
        email: ADRESSE,
        country_id: None,
        password: MOT_DE_PASSE,
        preferred_locale: "fr",
        timezone: "Africa/Dakar",
        client,
    }
}

/// Ce que le travail d'envoi porte : le client, à côté du jeton. C'est lui que
/// le composeur relit pour choisir l'écran.
async fn client_du_dernier_envoi(bac: &Bac, tache: &str) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT payload ->> 'client' FROM platform.jobs
          WHERE task = $1 ORDER BY created_at DESC LIMIT 1",
        tache
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi")
}

#[tokio::test]
async fn linscription_depuis_lapplication_retient_son_client() {
    let bac = Bac::monter().await;

    registration::register(&bac.state, &bac.ctx(), demande(ClientKind::App))
        .await
        .expect("inscription");

    assert_eq!(
        client_du_dernier_envoi(&bac, "identity.send_verification_email").await,
        Some("app".to_owned())
    );

    // Et le jeton lui-même le porte : c'est là qu'il survit à l'attente entre
    // l'envoi du courriel et le clic.
    let sur_le_jeton = sqlx::query_scalar!(
        "SELECT payload ->> 'client' FROM identity.one_time_tokens
          WHERE purpose = 'email_verification' ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du jeton");
    assert_eq!(sur_le_jeton, Some("app".to_owned()));
}

#[tokio::test]
async fn linscription_depuis_le_site_ne_change_pas() {
    let bac = Bac::monter().await;

    registration::register(&bac.state, &bac.ctx(), demande(ClientKind::Web))
        .await
        .expect("inscription");

    assert_eq!(
        client_du_dernier_envoi(&bac, "identity.send_verification_email").await,
        Some("web".to_owned())
    );
}

/// **Le client du RENVOI, pas celui de l'inscription.** Une personne qui s'est
/// inscrite sur le site mais redemande son lien depuis l'application doit
/// revenir dans l'application : c'est là qu'elle attend.
#[tokio::test]
async fn le_renvoi_suit_le_client_qui_redemande() {
    let bac = Bac::monter().await;
    registration::register(&bac.state, &bac.ctx(), demande(ClientKind::Web))
        .await
        .expect("inscription");

    registration::resend_verification(&bac.state, &bac.ctx(), ADRESSE, ClientKind::App)
        .await
        .expect("renvoi");

    assert_eq!(
        client_du_dernier_envoi(&bac, "identity.send_verification_email").await,
        Some("app".to_owned())
    );
}

#[tokio::test]
async fn le_mot_de_passe_oublie_retient_aussi_son_client() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    password_reset::request(&bac.state, &bac.ctx(), ADRESSE, ClientKind::App)
        .await
        .expect("demande de lien");

    assert_eq!(
        client_du_dernier_envoi(&bac, "identity.send_password_reset_email").await,
        Some("app".to_owned())
    );
}
