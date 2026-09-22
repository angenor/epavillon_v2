//! **La réponse de rotation perdue** — ADR-020, qui nuance R3.
//!
//! Le serveur a tourné le jeton ; la réponse ne revient pas — réseau saturé,
//! passage du Wi-Fi à la 4G, téléphone verrouillé au mauvais moment. Le
//! navigateur représente l'ancien jeton. Dans la minute, et tant que la
//! remplaçante n'a jamais été renouvelée, la session continue — **une seule
//! vivante**. Hors de ces bornes, tout est coupé.

mod commun;

use commun::{connexion, semer, Bac, Compte};
use identity::service::session::{self, Device, IssuedSession, RefreshOutcome};
use kernel::error::ErrorCode;

const ADRESSE: &str = "awa.diallo@example.org";

async fn renouveler(bac: &Bac, jeton: &str) -> kernel::error::Result<RefreshOutcome> {
    session::refresh(&bac.state, &bac.ctx(), jeton, Device::default()).await
}

fn neuve(issue: RefreshOutcome) -> IssuedSession {
    match issue {
        RefreshOutcome::Renewed(neuve) => *neuve,
        RefreshOutcome::Expired => panic!("la session devait continuer"),
    }
}

#[tokio::test]
async fn reponse_perdue_puis_nouvel_essai_la_session_continue() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    // Le serveur tourne ; la réponse — et le nouveau cookie — se perdent.
    let perdue = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("rotation"),
    );

    // Le navigateur représente l'ancien jeton.
    let reprise = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("une réponse perdue n'est pas un vol"),
    );

    assert_eq!(
        commun::sessions_vivantes(&bac, personne).await,
        1,
        "une seule session vivante"
    );
    assert_eq!(
        commun::acteur_resolu(&bac, &reprise.access_token).await,
        Some(personne)
    );
    assert_eq!(
        commun::acteur_resolu(&bac, &perdue.access_token).await,
        None,
        "la remplaçante jamais reçue est révoquée"
    );

    let motifs: Vec<Option<String>> = commun::sessions(&bac, personne)
        .await
        .into_iter()
        .map(|(_, m)| m)
        .collect();
    assert!(motifs.iter().any(|m| m.as_deref() == Some("response_lost")));
    assert!(!motifs
        .iter()
        .any(|m| m.as_deref() == Some("reuse_detected")));

    // La session reprise se renouvelle ensuite normalement.
    assert!(matches!(
        renouveler(&bac, &reprise.refresh_token)
            .await
            .expect("renouvellement suivant"),
        RefreshOutcome::Renewed(_)
    ));
}

/// Deux réponses perdues de suite : chaque essai reprend, et une seule vit.
#[tokio::test]
async fn deux_reponses_perdues_de_suite() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    renouveler(&bac, &ouverte.refresh_token)
        .await
        .expect("rotation");
    renouveler(&bac, &ouverte.refresh_token)
        .await
        .expect("premier essai");
    let dernier = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("second essai"),
    );

    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 1);
    assert_eq!(
        commun::acteur_resolu(&bac, &dernier.access_token).await,
        Some(personne)
    );
}

#[tokio::test]
async fn rejeu_apres_la_minute_tout_est_coupe() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;
    let autre_appareil = connexion(&bac, ADRESSE).await;

    renouveler(&bac, &ouverte.refresh_token)
        .await
        .expect("rotation");
    commun::vieillir_la_rotation(&bac, personne, 61).await;

    let refus = renouveler(&bac, &ouverte.refresh_token)
        .await
        .expect_err("hors de la fenêtre");
    assert_eq!(refus.code, ErrorCode::IdentityRefreshReused);
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
    assert_eq!(
        commun::acteur_resolu(&bac, &autre_appareil.access_token).await,
        None
    );
}

/// La remplaçante a servi : le navigateur l'avait donc reçue, et l'ancien jeton
/// qui revient ne peut plus être une réponse perdue.
#[tokio::test]
async fn rejeu_apres_usage_de_la_remplacante_tout_est_coupe() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    let remplacante = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("rotation"),
    );
    renouveler(&bac, &remplacante.refresh_token)
        .await
        .expect("la remplaçante sert");

    let refus = renouveler(&bac, &ouverte.refresh_token)
        .await
        .expect_err("la remplaçante a servi");
    assert_eq!(refus.code, ErrorCode::IdentityRefreshReused);
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
}

/// Le jeton d'une remplaçante révoquée pour réponse perdue n'a jamais été remis
/// au navigateur : s'il se présente, c'est un vol.
#[tokio::test]
async fn le_jeton_dune_remplacante_ecartee_est_un_vol() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    let ecartee = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("rotation"),
    );
    renouveler(&bac, &ouverte.refresh_token)
        .await
        .expect("reprise");

    let refus = renouveler(&bac, &ecartee.refresh_token)
        .await
        .expect_err("jeton jamais remis");
    assert_eq!(refus.code, ErrorCode::IdentityRefreshReused);
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
}
