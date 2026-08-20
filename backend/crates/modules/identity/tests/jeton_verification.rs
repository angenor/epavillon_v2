//! Les trois refus d'un jeton, **leur ordre**, et la consommation concurrente.
//!
//! « Déjà utilisé » l'emporte sur « périmé » : un jeton consommé puis périmé
//! dit que le travail est fait, là où « le lien a expiré » enverrait redemander
//! un courriel inutile. L'ordre inverse ne casse rien — c'est pourquoi il se
//! teste.

mod commun;

use commun::{Bac, MOT_DE_PASSE};
use identity::domain::token::{TokenRejection, VerifyEmailOutcome};
use identity::service::registration::{self, RegisterRequest};

const ADRESSE: &str = "awa.diallo@example.org";

/// Inscrit quelqu'un et rend le jeton en clair, tel qu'il partirait dans le
/// courriel — la file est le seul endroit où il existe.
async fn inscrire(bac: &Bac, email: &str) -> String {
    registration::register(
        &bac.state,
        &bac.ctx(),
        RegisterRequest {
            first_name: "Awa",
            last_name: "Diallo",
            email,
            country_id: None,
            password: MOT_DE_PASSE,
            preferred_locale: "fr",
            timezone: "Africa/Dakar",
        },
    )
    .await
    .expect("inscription");

    sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_verification_email'
          ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi")
    .expect("le travail porte le jeton en clair")
}

fn refus(issue: &VerifyEmailOutcome) -> TokenRejection {
    match issue {
        VerifyEmailOutcome::Rejected { reason } => *reason,
        VerifyEmailOutcome::Verified { .. } => panic!("le jeton devait être refusé"),
    }
}

#[tokio::test]
async fn un_jeton_valide_verifie_ladresse_une_fois() {
    let bac = Bac::monter().await;
    let jeton = inscrire(&bac, ADRESSE).await;

    let issue = registration::verify_email(&bac.state, &bac.ctx(), &jeton)
        .await
        .expect("vérification");

    match issue {
        VerifyEmailOutcome::Verified { email } => assert_eq!(email, ADRESSE),
        autre => panic!("issue inattendue : {autre:?}"),
    }

    let verifiee = sqlx::query_scalar!(
        "SELECT email_verified_at FROM identity.people
          WHERE primary_email = $1::text::platform.email",
        ADRESSE
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("relecture de la personne");
    assert!(verifiee.is_some());

    let evenements = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM platform.outbox_events
          WHERE event_type = 'identity.person.email_verified'"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(evenements, 1);
}

#[tokio::test]
async fn un_jeton_inconnu_est_invalide() {
    let bac = Bac::monter().await;

    let issue = registration::verify_email(&bac.state, &bac.ctx(), "jeton-qui-nexiste-pas")
        .await
        .expect("vérification");
    assert_eq!(refus(&issue), TokenRejection::Invalid);
}

#[tokio::test]
async fn un_jeton_perime_est_perime() {
    let bac = Bac::monter().await;
    let jeton = inscrire(&bac, ADRESSE).await;

    sqlx::query!("UPDATE identity.one_time_tokens SET expires_at = now() - interval '1 hour'")
        .execute(bac.base.pool())
        .await
        .expect("péremption forcée");

    let issue = registration::verify_email(&bac.state, &bac.ctx(), &jeton)
        .await
        .expect("vérification");
    assert_eq!(refus(&issue), TokenRejection::Expired);
}

/// L'ordre du contrat, éprouvé sur le seul cas où il se voit : un jeton
/// consommé **et** périmé.
#[tokio::test]
async fn deja_utilise_lemporte_sur_perime() {
    let bac = Bac::monter().await;
    let jeton = inscrire(&bac, ADRESSE).await;

    registration::verify_email(&bac.state, &bac.ctx(), &jeton)
        .await
        .expect("première vérification");

    sqlx::query!("UPDATE identity.one_time_tokens SET expires_at = now() - interval '1 hour'")
        .execute(bac.base.pool())
        .await
        .expect("péremption forcée");

    let issue = registration::verify_email(&bac.state, &bac.ctx(), &jeton)
        .await
        .expect("seconde vérification");
    assert_eq!(
        refus(&issue),
        TokenRejection::AlreadyUsed,
        "consommé PUIS périmé dit que le travail est fait"
    );
}

/// FR-041 : la consommation est atomique. Deux clics simultanés — le lien ouvert
/// deux fois, un client qui réémet — n'aboutissent qu'une fois, et c'est la base
/// qui tranche.
#[tokio::test]
async fn deux_consommations_simultanees_naboutissent_quune_fois() {
    let bac = Bac::monter().await;
    let jeton = inscrire(&bac, ADRESSE).await;

    let (ctx_a, ctx_b) = (bac.ctx(), bac.ctx());
    let (a, b) = tokio::join!(
        registration::verify_email(&bac.state, &ctx_a, &jeton),
        registration::verify_email(&bac.state, &ctx_b, &jeton),
    );

    let issues = [a.expect("premier appel"), b.expect("second appel")];
    let verifiees = issues
        .iter()
        .filter(|i| matches!(i, VerifyEmailOutcome::Verified { .. }))
        .count();
    let deja = issues
        .iter()
        .filter(|i| matches!(i, VerifyEmailOutcome::Rejected { reason } if *reason == TokenRejection::AlreadyUsed))
        .count();

    assert_eq!(verifiees, 1, "une seule vérification aboutit : {issues:?}");
    assert_eq!(deja, 1, "l'autre lit « déjà utilisé » : {issues:?}");

    let evenements = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM platform.outbox_events
          WHERE event_type = 'identity.person.email_verified'"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(
        evenements, 1,
        "un seul changement d'état, un seul événement"
    );
}

/// FR-040 : un lien plus récent invalide les précédents. Deux liens valides pour
/// la même adresse, c'est une surface d'attaque de plus pour aucun service
/// rendu — et le refus rendu à l'ancien est « périmé », le message juste puisque
/// le nouveau vient d'arriver.
#[tokio::test]
async fn un_lien_plus_recent_invalide_le_precedent() {
    let bac = Bac::monter().await;
    let ancien = inscrire(&bac, ADRESSE).await;

    registration::resend_verification(&bac.state, &bac.ctx(), ADRESSE)
        .await
        .expect("renvoi du lien");

    let nouveau = sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_verification_email'
          ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du second travail")
    .expect("jeton en clair");
    assert_ne!(ancien, nouveau);

    let issue = registration::verify_email(&bac.state, &bac.ctx(), &ancien)
        .await
        .expect("vérification de l'ancien");
    assert_eq!(refus(&issue), TokenRejection::Expired);

    let issue = registration::verify_email(&bac.state, &bac.ctx(), &nouveau)
        .await
        .expect("vérification du nouveau");
    assert!(matches!(issue, VerifyEmailOutcome::Verified { .. }));
}

/// Le renvoi rend la même chose dans les trois cas — inconnue, en attente, déjà
/// vérifiée —, et n'envoie de courriel que dans un seul.
#[tokio::test]
async fn le_renvoi_rend_toujours_la_meme_chose() {
    let bac = Bac::monter().await;

    let inconnue =
        registration::resend_verification(&bac.state, &bac.ctx(), "personne@example.org")
            .await
            .expect("renvoi sur adresse inconnue");
    assert_eq!(inconnue.status, "sent");

    let jeton = inscrire(&bac, ADRESSE).await;
    registration::verify_email(&bac.state, &bac.ctx(), &jeton)
        .await
        .expect("vérification");

    let deja_verifiee = registration::resend_verification(&bac.state, &bac.ctx(), ADRESSE)
        .await
        .expect("renvoi sur adresse déjà vérifiée");
    assert_eq!(deja_verifiee.status, "sent");

    let envois = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM platform.jobs
          WHERE task = 'identity.send_verification_email'"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des envois");
    assert_eq!(
        envois, 1,
        "seule l'inscription a produit un envoi : ni l'adresse inconnue, ni l'adresse déjà vérifiée"
    );
}
