//! Renouvellement, rotation, déconnexion.
//!
//! Ce que le test cherche : qu'une session vive, se renouvelle en **changeant de
//! jeton**, et se coupe pour de bon. Un renouvellement qui rendrait le même
//! jeton annulerait la détection de rejeu, sans rien casser de visible.

mod commun;

use commun::{appareil_de_lapplication, connexion, connexion_avec, semer, Bac, Compte};
use identity::service::session::{self, Device, RefreshOutcome};

const ADRESSE: &str = "awa.diallo@example.org";

#[tokio::test]
async fn le_renouvellement_fait_tourner_le_jeton() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    let issue = session::refresh(
        &bac.state,
        &bac.ctx(),
        &ouverte.refresh_token,
        Device::default(),
    )
    .await
    .expect("renouvellement");

    let RefreshOutcome::Renewed(neuve) = issue else {
        panic!("le renouvellement devait aboutir");
    };

    assert_ne!(
        neuve.refresh_token, ouverte.refresh_token,
        "le jeton de rafraîchissement doit changer à chaque tour"
    );
    assert_ne!(neuve.session_id, ouverte.session_id);

    // L'échéance ne glisse pas : la session neuve hérite de celle qu'elle
    // remplace, sinon douze heures deviendraient une session éternelle.
    assert_eq!(neuve.expires_at, ouverte.expires_at);

    let sessions = commun::sessions(&bac, personne).await;
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].1.as_deref(), Some("rotated"));
    assert_eq!(sessions[1].1, None);
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 1);
}

#[tokio::test]
async fn le_jeton_dacces_de_la_session_tournee_ne_vaut_plus() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    assert_eq!(
        commun::acteur_resolu(&bac, &ouverte.access_token).await,
        Some(personne)
    );

    let RefreshOutcome::Renewed(neuve) = session::refresh(
        &bac.state,
        &bac.ctx(),
        &ouverte.refresh_token,
        Device::default(),
    )
    .await
    .expect("renouvellement") else {
        panic!("le renouvellement devait aboutir");
    };

    // La signature du premier jeton reste bonne quinze minutes ; sa session,
    // elle, est révoquée. C'est la relecture en base qui le refuse.
    assert_eq!(
        commun::acteur_resolu(&bac, &ouverte.access_token).await,
        None
    );
    assert_eq!(
        commun::acteur_resolu(&bac, &neuve.access_token).await,
        Some(personne)
    );
}

#[tokio::test]
async fn la_deconnexion_coupe_la_session() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    session::logout(&bac.state, &bac.ctx(), Some(&ouverte.refresh_token))
        .await
        .expect("déconnexion");

    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
    assert_eq!(
        commun::sessions(&bac, personne).await[0].1.as_deref(),
        Some("logout")
    );
    assert_eq!(
        commun::acteur_resolu(&bac, &ouverte.access_token).await,
        None
    );

    let issue = session::refresh(
        &bac.state,
        &bac.ctx(),
        &ouverte.refresh_token,
        Device::default(),
    )
    .await
    .expect("renouvellement");
    assert!(matches!(issue, RefreshOutcome::Expired));
}

/// Se déconnecter deux fois n'est pas une erreur, et se déconnecter sans session
/// non plus.
#[tokio::test]
async fn la_deconnexion_reussit_meme_sans_session() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    for _ in 0..2 {
        session::logout(&bac.state, &bac.ctx(), Some(&ouverte.refresh_token))
            .await
            .expect("déconnexion répétée");
    }
    session::logout(&bac.state, &bac.ctx(), None)
        .await
        .expect("déconnexion sans jeton");
    session::logout(&bac.state, &bac.ctx(), Some("un-jeton-qui-nexiste-pas"))
        .await
        .expect("déconnexion sur un jeton inconnu");
}

#[tokio::test]
async fn un_jeton_inconnu_ne_renouvelle_rien() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    let issue = session::refresh(
        &bac.state,
        &bac.ctx(),
        "un-jeton-qui-nexiste-pas",
        Device::default(),
    )
    .await
    .expect("renouvellement");

    assert!(matches!(issue, RefreshOutcome::Expired));
}

/// **Le piège n° 1 de l'étape 0b, et il est muet.** La rotation ouvre une
/// session neuve à partir de la remplacée ; si elle n'en recopie pas le client,
/// toute session de l'application redevient « web » au premier renouvellement.
/// Rien n'échoue, personne n'est déconnecté, et le décompte des téléphones ment.
///
/// Le renouvellement **ne déclare rien** : `POST /auth/refresh` ne porte aucun
/// objet `client`, et le test le prouve en passant un appareil vide.
#[tokio::test]
async fn la_rotation_recopie_le_client_et_lappareil() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;

    let RefreshOutcome::Renewed(neuve) = session::refresh(
        &bac.state,
        &bac.ctx(),
        &ouverte.refresh_token,
        Device::default(),
    )
    .await
    .expect("renouvellement") else {
        panic!("le renouvellement devait aboutir");
    };

    let ligne = commun::ligne_de_session(&bac, neuve.session_id.as_uuid()).await;
    assert_eq!(
        ligne.client_kind, "app",
        "une session d'application qui redevient « web » ne fait échouer aucun test : celui-ci existe pour ça"
    );
    assert_eq!(ligne.device_id.as_deref(), Some("9f2c-appareil-de-test"));
    assert_eq!(ligne.device_label.as_deref(), Some("Android · Chrome"));
    assert_eq!(ligne.device_platform.as_deref(), Some("android"));
}

/// Deux tours d'affilée : le client se recopie de proche en proche, et ne se
/// perd pas au second. Un seul renouvellement ne le prouverait pas.
#[tokio::test]
async fn le_client_survit_a_deux_rotations() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;
    let mut jeton = connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false)
        .await
        .refresh_token;

    for tour in 1..=2 {
        let RefreshOutcome::Renewed(neuve) =
            session::refresh(&bac.state, &bac.ctx(), &jeton, Device::default())
                .await
                .expect("renouvellement")
        else {
            panic!("le renouvellement devait aboutir");
        };

        let ligne = commun::ligne_de_session(&bac, neuve.session_id.as_uuid()).await;
        assert_eq!(ligne.client_kind, "app", "tour {tour}");
        jeton = neuve.refresh_token;
    }
}
