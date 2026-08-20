//! Les six issues de connexion, une par une — et l'ordre dans lequel elles se
//! produisent, qui est la vraie règle (FR-019 à FR-021, FR-024, FR-027).

mod commun;

use commun::{semer, Bac, Compte, MOT_DE_PASSE};
use identity::domain::login::{LoginOutcome, MfaMethod};
use identity::service::auth::{login, LoginRequest, LoginResponse};
use identity::service::session::Device;
use time::{Duration, OffsetDateTime};

async fn tenter(bac: &Bac, email: &str, mot_de_passe: &str) -> LoginResponse {
    login(
        &bac.state,
        &bac.ctx(),
        LoginRequest {
            email,
            password: mot_de_passe,
            remember_me: false,
            device: Device {
                user_agent: Some("test"),
                ip: "127.0.0.1".parse().ok(),
            },
        },
    )
    .await
    .expect("connexion")
}

#[tokio::test]
async fn issue_1_authentifie() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif("awa.diallo@example.org")).await;

    let reponse = tenter(&bac, "awa.diallo@example.org", MOT_DE_PASSE).await;

    match &reponse.outcome {
        LoginOutcome::Authenticated { person } => {
            assert_eq!(person.id.as_uuid(), personne);
            assert_eq!(person.primary_email, "awa.diallo@example.org");
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }
    assert!(reponse.session.is_some(), "une session doit être ouverte");
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 1);
}

/// `citext` rend la comparaison insensible à la casse : c'est la base qui le
/// porte, pas le service — et ce test le vérifie plutôt que de le supposer.
#[tokio::test]
async fn ladresse_ignore_la_casse() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif("awa.diallo@example.org")).await;

    let reponse = tenter(&bac, "Awa.DIALLO@Example.ORG", MOT_DE_PASSE).await;

    assert!(reponse.outcome.est_authentifie());
}

#[tokio::test]
async fn issue_2_identifiants_invalides() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif("awa.diallo@example.org")).await;
    semer(&bac, Compte::sans_mot_de_passe("invitee@example.org")).await;

    // Adresse inconnue.
    let inconnue = tenter(&bac, "personne.inconnue@example.org", "nimportequoi").await;
    assert!(matches!(inconnue.outcome, LoginOutcome::InvalidCredentials));
    assert!(inconnue.session.is_none());

    // Adresse connue, mot de passe faux.
    let faux = tenter(&bac, "awa.diallo@example.org", "nimportequoi").await;
    assert!(matches!(faux.outcome, LoginOutcome::InvalidCredentials));

    // Personne connue de la plateforme, dépourvue de compte mot de passe.
    let sans_compte = tenter(&bac, "invitee@example.org", MOT_DE_PASSE).await;
    assert!(matches!(
        sans_compte.outcome,
        LoginOutcome::InvalidCredentials
    ));
}

#[tokio::test]
async fn issue_3_verrouille_apres_le_seuil() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif("awa.diallo@example.org")).await;
    let seuil = bac.config.auth.lockout_threshold;

    for _ in 0..seuil {
        let echec = tenter(&bac, "awa.diallo@example.org", "nimportequoi").await;
        assert!(
            matches!(echec.outcome, LoginOutcome::InvalidCredentials),
            "tant que le mot de passe est faux, la seule issue possible est « identifiants invalides »"
        );
    }

    let apres = tenter(&bac, "awa.diallo@example.org", MOT_DE_PASSE).await;
    match apres.outcome {
        LoginOutcome::Locked { until } => {
            assert!(
                until > OffsetDateTime::now_utc(),
                "le verrou porte sa date de fin"
            );
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }
    assert!(apres.session.is_none());

    // L'événement de verrouillage est émis une fois, à la pose du verrou.
    let emis = sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!" FROM platform.outbox_events
            WHERE event_type = 'identity.account.locked'"#
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture de l'outbox");
    assert_eq!(emis, 1);

    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
}

#[tokio::test]
async fn issue_4_suspendu_et_bloque() {
    let bac = Bac::monter().await;
    let fin = OffsetDateTime::now_utc() + Duration::days(7);
    semer(&bac, Compte::suspendu("suspendue@example.org", fin)).await;
    semer(&bac, Compte::bloque("bloque@example.org")).await;

    match tenter(&bac, "suspendue@example.org", MOT_DE_PASSE)
        .await
        .outcome
    {
        LoginOutcome::Suspended { until } => assert!(until.is_some()),
        autre => panic!("issue inattendue : {autre:?}"),
    }

    // Une exclusion n'a pas de date de fin : le contrat porte alors `null`.
    match tenter(&bac, "bloque@example.org", MOT_DE_PASSE)
        .await
        .outcome
    {
        LoginOutcome::Suspended { until } => assert!(until.is_none()),
        autre => panic!("issue inattendue : {autre:?}"),
    }
}

/// Écart n° 20 : l'état « en attente de vérification » est porté par la seule
/// date de vérification, et il interdit la connexion.
#[tokio::test]
async fn connexion_refusee_si_adresse_non_verifiee() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::non_verifie("awa.diallo@example.org")).await;

    let reponse = tenter(&bac, "awa.diallo@example.org", MOT_DE_PASSE).await;

    match &reponse.outcome {
        LoginOutcome::EmailUnverified { email } => {
            assert_eq!(email, "awa.diallo@example.org");
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }
    assert!(
        reponse.session.is_none(),
        "aucun cookie de session n'est posé"
    );
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
}

#[tokio::test]
async fn issue_6_second_facteur_requis() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::avec_second_facteur("awa.diallo@example.org")).await;

    let reponse = tenter(&bac, "awa.diallo@example.org", MOT_DE_PASSE).await;

    match reponse.outcome {
        LoginOutcome::MfaRequired { method, .. } => assert_eq!(method, MfaMethod::Totp),
        autre => panic!("issue inattendue : {autre:?}"),
    }
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
}

/// FR-019 : verrou, suspension, adresse non vérifiée et second facteur ne se
/// signalent qu'APRÈS un mot de passe correct. Un mot de passe faux sur un
/// compte suspendu ne doit rien apprendre de sa suspension.
#[tokio::test]
async fn le_mot_de_passe_passe_avant_tout_le_reste() {
    let bac = Bac::monter().await;
    let fin = OffsetDateTime::now_utc() + Duration::days(7);
    semer(&bac, Compte::suspendu("suspendue@example.org", fin)).await;
    semer(&bac, Compte::non_verifie("attente@example.org")).await;

    assert!(matches!(
        tenter(&bac, "suspendue@example.org", "nimportequoi")
            .await
            .outcome,
        LoginOutcome::InvalidCredentials
    ));
    assert!(matches!(
        tenter(&bac, "attente@example.org", "nimportequoi")
            .await
            .outcome,
        LoginOutcome::InvalidCredentials
    ));
}

/// FR-015, le cas qui manquait : après l'échéance du verrou, le compte doit
/// retrouver **tous** ses essais, pas un seul. Le compteur avait continué de
/// grimper pendant le verrou ; s'il n'est pas purgé à l'échéance, la première
/// faute de frappe suivante repose un verrou d'un quart d'heure — indéfiniment.
#[tokio::test]
async fn un_verrou_echu_rend_tous_ses_essais_au_compte() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif("awa.diallo@example.org")).await;
    let seuil = bac.config.auth.lockout_threshold;

    for _ in 0..seuil {
        tenter(&bac, "awa.diallo@example.org", "nimportequoi").await;
    }
    antidater_le_verrou(&bac, personne).await;

    // Un mot de passe faux de plus : le verrou échu doit avoir été purgé, donc
    // aucun nouveau verrou ne se pose.
    let apres = tenter(&bac, "awa.diallo@example.org", "nimportequoi").await;
    assert!(
        matches!(apres.outcome, LoginOutcome::InvalidCredentials),
        "issue inattendue : {:?}",
        apres.outcome
    );

    let etat = sqlx::query!(
        "SELECT failed_attempts, locked_until FROM identity.accounts WHERE person_id = $1",
        personne
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du compte");

    assert_eq!(
        etat.failed_attempts, 1,
        "le compteur repart de cette tentative"
    );
    assert!(etat.locked_until.is_none(), "aucun verrou n'est reposé");

    // Et le compte se reconnecte, sans avoir eu à attendre un second quart d'heure.
    assert!(tenter(&bac, "awa.diallo@example.org", MOT_DE_PASSE)
        .await
        .outcome
        .est_authentifie());
}

/// FR-015 : un verrou échu ne se traîne pas — le compteur repart de zéro.
#[tokio::test]
async fn un_verrou_echu_libere_le_compte() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif("awa.diallo@example.org")).await;

    for _ in 0..bac.config.auth.lockout_threshold {
        tenter(&bac, "awa.diallo@example.org", "nimportequoi").await;
    }

    antidater_le_verrou(&bac, personne).await;

    let reponse = tenter(&bac, "awa.diallo@example.org", MOT_DE_PASSE).await;
    assert!(reponse.outcome.est_authentifie());

    let compteur = sqlx::query_scalar!(
        "SELECT failed_attempts FROM identity.accounts WHERE person_id = $1",
        personne
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du compteur");
    assert_eq!(compteur, 0);
}

/// Plutôt que d'attendre un quart d'heure.
async fn antidater_le_verrou(bac: &Bac, personne: uuid::Uuid) {
    sqlx::query!(
        "UPDATE identity.accounts SET locked_until = now() - interval '1 minute'
          WHERE person_id = $1",
        personne
    )
    .execute(bac.base.pool())
    .await
    .expect("antidatage du verrou");
}
