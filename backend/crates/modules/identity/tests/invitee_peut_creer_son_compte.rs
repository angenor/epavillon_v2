//! Une personne **créée sans compte** peut s'inscrire, vérifier son adresse et
//! se connecter.
//!
//! C'est le défaut de B1 que B2 corrige (specs/002-organisations/research.md
//! § R9) : l'invitation par adresse crée une personne sans compte, et
//! l'inscription branchait sur la seule existence de l'adresse. L'invitée
//! recevait « vous avez déjà un compte » — pour un compte qui n'existait pas —
//! et n'avait aucune issue.

mod commun;

use commun::{semer, Bac, Compte, MOT_DE_PASSE};
use identity::service::auth::{self, LoginRequest};
use identity::service::registration::{self, RegisterRequest};
use identity::service::session::Device;

const ADRESSE: &str = "b.ouedraogo@osed-sahel.org";

#[tokio::test]
async fn une_personne_creee_sans_compte_sinscrit_verifie_et_se_connecte() {
    let bac = Bac::monter().await;
    // Ce que produit une invitation : la personne existe, son compte non.
    let person_id = semer(&bac, Compte::sans_mot_de_passe(ADRESSE)).await;

    // L'adresse n'est pas vérifiée : l'invitation ne l'a pas encore prouvée.
    sqlx::query!(
        "UPDATE identity.people SET email_verified_at = NULL WHERE id = $1",
        person_id
    )
    .execute(bac.base.pool())
    .await
    .expect("adresse remise en attente de vérification");

    let reponse = registration::register(
        &bac.state,
        &bac.ctx(),
        RegisterRequest {
            first_name: "Boureima",
            last_name: "Ouédraogo",
            email: ADRESSE,
            country_id: None,
            password: MOT_DE_PASSE,
            preferred_locale: "fr",
            timezone: "Africa/Ouagadougou",
        },
    )
    .await
    .expect("inscription d'une personne connue sans compte");
    assert_eq!(reponse.status, "verification_sent");

    let jeton = sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_verification_email'
          ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi")
    .expect("le travail porte le jeton en clair");

    registration::verify_email(&bac.state, &bac.ctx(), &jeton)
        .await
        .expect("vérification de l'adresse");

    let connexion = auth::login(
        &bac.state,
        &bac.ctx(),
        LoginRequest {
            email: ADRESSE,
            password: MOT_DE_PASSE,
            remember_me: false,
            device: Device {
                user_agent: Some("test"),
                ip: None,
            },
        },
    )
    .await
    .expect("connexion");

    assert!(
        connexion.outcome.est_authentifie(),
        "l'invitée doit pouvoir se connecter : {:?}",
        connexion.outcome
    );

    // La personne n'a pas été dupliquée : c'est la ligne de l'invitation qui a
    // reçu son compte.
    let personnes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.people
            WHERE primary_email = $1::text::platform.email"#,
        ADRESSE
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(personnes, 1);
}
