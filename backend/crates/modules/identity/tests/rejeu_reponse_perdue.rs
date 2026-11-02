//! **Le jeton tourné qui revient dans la minute** : ADR-024, qui remplace la
//! reprise d'ADR-020.
//!
//! Deux onglets qui rafraîchissent ensemble, ou une réponse perdue au retour :
//! le navigateur représente l'ancien jeton. Dans la minute, une session sœur
//! s'ouvre et **la première reste vivante**, car rien ne dit quel cookie le
//! navigateur garde. Hors de la minute, tout est coupé ; une lignée fermée
//! entre-temps ne se rouvre pas.

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

    let premiere = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("rotation"),
    );
    let soeur = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("dans la minute, ce n'est pas un vol"),
    );

    assert_eq!(
        commun::acteur_resolu(&bac, &soeur.access_token).await,
        Some(personne)
    );
    assert_eq!(
        commun::acteur_resolu(&bac, &premiere.access_token).await,
        Some(personne),
        "la première remplaçante reste vivante : c'est peut-être son cookie que le navigateur garde"
    );

    let motifs: Vec<Option<String>> = commun::sessions(&bac, personne)
        .await
        .into_iter()
        .map(|(_, m)| m)
        .collect();
    assert!(!motifs
        .iter()
        .any(|m| m.as_deref() == Some("reuse_detected")));

    // Les deux se renouvellent ensuite normalement.
    for jeton in [&premiere.refresh_token, &soeur.refresh_token] {
        assert!(matches!(
            renouveler(&bac, jeton)
                .await
                .expect("renouvellement suivant"),
            RefreshOutcome::Renewed(_)
        ));
    }
}

/// Six onglets qui présentent le même jeton à la suite : aucun n'est déconnecté.
#[tokio::test]
async fn six_onglets_dans_la_minute() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    for _ in 0..6 {
        let issue = neuve(
            renouveler(&bac, &ouverte.refresh_token)
                .await
                .expect("onglet"),
        );
        assert_eq!(
            commun::acteur_resolu(&bac, &issue.access_token).await,
            Some(personne)
        );
    }
}

/// La remplaçante a déjà servi : l'ancien jeton dans la minute reste un onglet
/// en retard, pas un vol.
#[tokio::test]
async fn apres_usage_de_la_remplacante_la_minute_tient() {
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

    let soeur = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("dans la minute"),
    );
    assert_eq!(
        commun::acteur_resolu(&bac, &soeur.access_token).await,
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

/// Déconnexion dans un onglet, puis un onglet en retard présente l'ancien jeton
/// dans la minute : rien ne se rouvre, et l'autre appareil n'est pas coupé.
#[tokio::test]
async fn une_lignee_fermee_ne_se_rouvre_pas() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;
    let autre_appareil = connexion(&bac, ADRESSE).await;

    let remplacante = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("rotation"),
    );
    session::logout(
        &bac.state,
        &bac.ctx(),
        Some(remplacante.refresh_token.as_str()),
    )
    .await
    .expect("déconnexion");

    let issue = renouveler(&bac, &ouverte.refresh_token)
        .await
        .expect("une lignée close n'est pas un vol");
    assert!(matches!(issue, RefreshOutcome::Expired));
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 1);
    assert_eq!(
        commun::acteur_resolu(&bac, &autre_appareil.access_token).await,
        Some(personne)
    );
}

/// Le jeton d'une remplaçante écartée par l'ancienne reprise (ADR-020) n'a
/// jamais été remis au navigateur : s'il se présente, c'est un vol.
#[tokio::test]
async fn le_jeton_dune_remplacante_ecartee_reste_un_vol() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    let ecartee = neuve(
        renouveler(&bac, &ouverte.refresh_token)
            .await
            .expect("rotation"),
    );
    sqlx::query!(
        "UPDATE identity.sessions SET revoked_at = now(), revoked_reason = 'response_lost'
          WHERE id = $1",
        ecartee.session_id.as_uuid()
    )
    .execute(bac.base.pool())
    .await
    .expect("écartée comme le faisait ADR-020");

    let refus = renouveler(&bac, &ecartee.refresh_token)
        .await
        .expect_err("jeton jamais remis");
    assert_eq!(refus.code, ErrorCode::IdentityRefreshReused);
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
}
