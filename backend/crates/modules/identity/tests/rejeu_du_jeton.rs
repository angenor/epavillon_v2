//! FR-031 : un jeton de rafraîchissement rejoué révoque **toutes** les sessions
//! de la personne.
//!
//! Hors d'une réponse de rotation perdue — la minute qui suit la rotation, tant
//! que la remplaçante n'a pas servi (ADR-020, `rejeu_reponse_perdue.rs`) —, un
//! jeton présenté deux fois n'a aucune explication innocente : volé, ou copié.
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

/// La course : deux renouvellements partis ensemble avec le même jeton.
///
/// Le client n'en envoie qu'un à la fois. Ce que le test tient, c'est
/// l'invariant qui compte quand cette hypothèse est fausse : **jamais deux
/// sessions vivantes.** Selon l'ordre d'arrivée, le second est pris pour un
/// rejeu — tout est coupé — ou pour une réponse perdue — la première remplaçante
/// tombe (ADR-020) ; dans les deux cas, aucune session orpheline ne survit.
#[tokio::test]
async fn deux_renouvellements_simultanes_nouvrent_quune_session() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    let ctx = bac.ctx();
    let (un, deux) = tokio::join!(
        session::refresh(&bac.state, &ctx, &ouverte.refresh_token, Device::default()),
        session::refresh(&bac.state, &ctx, &ouverte.refresh_token, Device::default()),
    );

    let renouvelees = [&un, &deux]
        .iter()
        .filter(|issue| matches!(issue, Ok(RefreshOutcome::Renewed(_))))
        .count();
    assert!(renouvelees >= 1);

    for refus in [un, deux].into_iter().filter_map(Result::err) {
        assert_eq!(refus.code, ErrorCode::IdentityRefreshReused);
    }
    assert!(
        commun::sessions_vivantes(&bac, personne).await <= 1,
        "jamais deux sessions vivantes nées d'un même jeton"
    );
}
