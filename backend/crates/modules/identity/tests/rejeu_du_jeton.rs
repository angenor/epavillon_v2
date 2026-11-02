//! FR-031 : un jeton de rafraîchissement rejoué révoque **toutes** les sessions
//! de la personne.
//!
//! Passé la minute qui suit la rotation (onglets, réponse perdue : ADR-024,
//! `rejeu_reponse_perdue.rs`), un jeton présenté deux fois n'a aucune
//! explication innocente : volé, ou copié.
//! La seule réponse sûre est de tout couper — y compris les sessions ouvertes sur
//! d'autres appareils, qui sont précisément celles qu'un vol vise ensuite.

mod commun;

use commun::{connexion, semer, Bac, Compte};
use identity::service::session::{self, Device, RefreshOutcome};
use kernel::error::ErrorCode;

const ADRESSE: &str = "awa.diallo@example.org";

#[tokio::test]
async fn rejeu_du_jeton_revoque_tout() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;

    // Deux appareils : le second n'a rien fait de mal, et tombe quand même.
    let vole = connexion(&bac, ADRESSE).await;
    let autre_appareil = connexion(&bac, ADRESSE).await;
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 2);

    let premier = session::refresh(
        &bac.state,
        &bac.ctx(),
        &vole.refresh_token,
        Device::default(),
    )
    .await
    .expect("premier renouvellement");
    assert!(matches!(premier, RefreshOutcome::Renewed(_)));

    // Au-delà de la tolérance : la réponse perdue n'explique plus rien.
    commun::vieillir_la_rotation(&bac, personne, 120).await;

    let rejeu = session::refresh(
        &bac.state,
        &bac.ctx(),
        &vole.refresh_token,
        Device::default(),
    )
    .await
    .expect_err("le rejeu doit être refusé");

    assert_eq!(rejeu.code, ErrorCode::IdentityRefreshReused);
    assert_eq!(
        commun::sessions_vivantes(&bac, personne).await,
        0,
        "toutes les sessions de la personne doivent tomber, pas seulement la rejouée"
    );
    assert_eq!(
        commun::acteur_resolu(&bac, &autre_appareil.access_token).await,
        None
    );

    let motifs: Vec<Option<String>> = commun::sessions(&bac, personne)
        .await
        .into_iter()
        .map(|(_, motif)| motif)
        .collect();
    assert!(motifs
        .iter()
        .any(|m| m.as_deref() == Some("reuse_detected")));
}

/// Le rejeu se distingue d'un jeton simplement périmé : l'un coupe tout et
/// s'annonce, l'autre demande seulement de se reconnecter.
#[tokio::test]
async fn une_session_expiree_nest_pas_un_rejeu() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    sqlx::query!(
        "UPDATE identity.sessions SET expires_at = now() - interval '1 minute'
          WHERE person_id = $1",
        personne
    )
    .execute(bac.base.pool())
    .await
    .expect("péremption de la session");

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

/// La course : deux onglets partagent les cookies et rafraîchissent ensemble
/// avec le même jeton (incident du 02/11/2026, toutes les sessions coupées).
/// Les deux réussissent, et aucune ne coupe l'autre (ADR-024).
#[tokio::test]
async fn deux_renouvellements_simultanes_reussissent_tous_deux() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    let ctx = bac.ctx();
    let (un, deux) = tokio::join!(
        session::refresh(&bac.state, &ctx, &ouverte.refresh_token, Device::default()),
        session::refresh(&bac.state, &ctx, &ouverte.refresh_token, Device::default()),
    );

    for issue in [un, deux] {
        let neuve = match issue {
            Ok(RefreshOutcome::Renewed(neuve)) => neuve,
            autre => panic!("deux onglets ne se déconnectent pas l'un l'autre : {autre:?}"),
        };
        assert_eq!(
            commun::acteur_resolu(&bac, &neuve.access_token).await,
            Some(personne)
        );
    }
    let motifs: Vec<Option<String>> = commun::sessions(&bac, personne)
        .await
        .into_iter()
        .map(|(_, motif)| motif)
        .collect();
    assert!(!motifs
        .iter()
        .any(|m| m.as_deref() == Some("reuse_detected")));
}
