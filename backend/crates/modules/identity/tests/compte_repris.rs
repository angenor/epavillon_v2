//! Comptes repris de la v1 (décision D1) : ligne `password` sans empreinte,
//! nouveau mot de passe exigé.
//!
//! Ce qui se vérifie ici : le modèle n'admet une ligne sans empreinte que si
//! l'exigence est posée ; la connexion rend « nouveau mot de passe exigé » quel
//! que soit le mot de passe, sans compter d'échec ni verrouiller (décision du
//! 29/10) ; la réinitialisation reprend **cette** ligne (pas une seconde),
//! efface l'exigence et vaut vérification de l'adresse.

mod commun;

use commun::{semer, Bac, Compte};
use identity::domain::login::LoginOutcome;
use identity::domain::token::PasswordResetOutcome;
use identity::repo::sessions::ClientKind;
use identity::service::auth::{login, LoginRequest};
use identity::service::password_reset;
use identity::service::session::Device;
use time::OffsetDateTime;
use uuid::Uuid;

const ADRESSE: &str = "awa.diallo@example.org";
const NOUVEAU: &str = "Ouagadougou2027";

async fn tenter(bac: &Bac, mot_de_passe: &str) -> LoginOutcome {
    login(
        &bac.state,
        &bac.ctx(),
        LoginRequest {
            email: ADRESSE,
            password: mot_de_passe,
            remember_me: false,
            device: Device::default(),
        },
    )
    .await
    .expect("connexion")
    .outcome
}

async fn reinitialiser(bac: &Bac) {
    password_reset::request(&bac.state, &bac.ctx(), ADRESSE, ClientKind::Web)
        .await
        .expect("demande de réinitialisation");

    let jeton = sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_password_reset_email'
          ORDER BY created_at DESC, id DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi")
    .expect("le travail porte le jeton en clair");

    let issue = password_reset::confirm(&bac.state, &bac.ctx(), &jeton, NOUVEAU)
        .await
        .expect("enregistrement du mot de passe");
    assert!(
        matches!(issue, PasswordResetOutcome::Reset { .. }),
        "issue inattendue : {issue:?}"
    );
}

struct LigneDeCompte {
    id: Uuid,
    a_une_empreinte: bool,
    exigence: Option<OffsetDateTime>,
}

async fn comptes_password(bac: &Bac, person_id: Uuid) -> Vec<LigneDeCompte> {
    sqlx::query!(
        r#"SELECT id, password_hash IS NOT NULL AS "a_une_empreinte!", password_reset_required_at
             FROM identity.accounts
            WHERE person_id = $1 AND provider = 'password'"#,
        person_id
    )
    .fetch_all(bac.base.pool())
    .await
    .expect("lecture des comptes")
    .into_iter()
    .map(|l| LigneDeCompte {
        id: l.id,
        a_une_empreinte: l.a_une_empreinte,
        exigence: l.password_reset_required_at,
    })
    .collect()
}

async fn verifiee_le(bac: &Bac, person_id: Uuid) -> Option<OffsetDateTime> {
    sqlx::query_scalar!(
        "SELECT email_verified_at FROM identity.people WHERE id = $1",
        person_id
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture de la personne")
}

async fn evenements_de_verification(bac: &Bac, person_id: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events
            WHERE event_type = 'identity.person.email_verified'
              AND aggregate_id = $1"#,
        person_id
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage")
}

#[tokio::test]
async fn le_modele_refuse_une_ligne_sans_empreinte_ni_exigence() {
    let bac = Bac::monter().await;
    let person_id = semer(
        &bac,
        Compte {
            avec_mot_de_passe: false,
            ..Compte::actif(ADRESSE)
        },
    )
    .await;

    let refus = sqlx::query!(
        "INSERT INTO identity.accounts (person_id, provider) VALUES ($1, 'password')",
        person_id
    )
    .execute(bac.base.pool())
    .await
    .expect_err("une ligne password sans empreinte ni exigence doit être refusée");
    assert!(
        refus.to_string().contains("ck_accounts_password_shape"),
        "refus inattendu : {refus}"
    );
}

#[tokio::test]
async fn un_compte_repris_est_envoye_vers_la_reinitialisation() {
    let bac = Bac::monter().await;
    let person_id = semer(&bac, Compte::repris(ADRESSE)).await;
    let essais = bac.config.auth.lockout_threshold as usize + 2;

    // Mot de passe vide, quelconque, ou celui qu'elle choisira plus tard : tous
    // rendent la même issue, et bien au-delà du seuil de verrouillage.
    for essai in ["", "Belem2027!", NOUVEAU].iter().cycle().take(essais) {
        let issue = tenter(&bac, essai).await;
        assert!(
            matches!(issue, LoginOutcome::PasswordResetRequired),
            "issue inattendue : {issue:?}"
        );
    }

    let comptes = comptes_password(&bac, person_id).await;
    assert_eq!(comptes.len(), 1);
    assert!(
        comptes[0].exigence.is_some(),
        "la connexion ne lève pas l'exigence"
    );

    let (echecs, verrou): (i16, Option<OffsetDateTime>) =
        sqlx::query_as("SELECT failed_attempts, locked_until FROM identity.accounts WHERE id = $1")
            .bind(comptes[0].id)
            .fetch_one(bac.base.pool())
            .await
            .expect("lecture du compteur");
    assert_eq!(echecs, 0, "aucun échec compté");
    assert!(verrou.is_none(), "aucun verrou posé");
}

#[tokio::test]
async fn un_compte_ordinaire_garde_le_refus_ordinaire() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    assert!(matches!(
        tenter(&bac, "nimportequoi").await,
        LoginOutcome::InvalidCredentials
    ));
}

#[tokio::test]
async fn la_reinitialisation_reprend_la_ligne_et_verifie_l_adresse() {
    let bac = Bac::monter().await;
    let person_id = semer(
        &bac,
        Compte {
            verifie: false,
            ..Compte::repris(ADRESSE)
        },
    )
    .await;
    let avant = comptes_password(&bac, person_id).await;

    reinitialiser(&bac).await;

    let apres = comptes_password(&bac, person_id).await;
    assert_eq!(apres.len(), 1, "aucune seconde ligne password");
    assert_eq!(apres[0].id, avant[0].id, "la ligne reprise est mise à jour");
    assert!(apres[0].a_une_empreinte);
    assert!(apres[0].exigence.is_none(), "l'exigence est effacée");

    assert!(
        verifiee_le(&bac, person_id).await.is_some(),
        "le lien arrivé dans la boîte prouve l'adresse"
    );
    assert_eq!(evenements_de_verification(&bac, person_id).await, 1);

    assert!(
        matches!(
            tenter(&bac, NOUVEAU).await,
            LoginOutcome::Authenticated { .. }
        ),
        "le nouveau mot de passe ouvre la session"
    );
}

#[tokio::test]
async fn la_reinitialisation_garde_une_verification_anterieure() {
    let bac = Bac::monter().await;
    let person_id = semer(&bac, Compte::actif(ADRESSE)).await;
    let avant = verifiee_le(&bac, person_id).await;

    reinitialiser(&bac).await;

    assert_eq!(verifiee_le(&bac, person_id).await, avant);
    assert_eq!(evenements_de_verification(&bac, person_id).await, 0);
}
