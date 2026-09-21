//! La durée d'une session suit le client — et pour l'application, elle glisse.
//!
//! **Le constat** : douze heures, ou trente jours avec « se souvenir de moi ».
//! L'écran de connexion de Guide Négo n'a pas cette case, et une négociatrice
//! déconnectée toutes les douze heures dans une salle sans réseau ne peut plus
//! compter sur l'application — c'est précisément là qu'elle en a besoin.
//!
//! **Glissante, et c'est le point qui compte.** Une durée fixe de quatre-vingt-dix
//! jours posée à la connexion ferait expirer le 14 novembre un compte ouvert le
//! 15 octobre, au milieu de la COP, après des semaines de préparation.

mod commun;

use commun::{appareil_de_lapplication, appareil_du_site, connexion_avec, semer, Bac, Compte};
use identity::service::session::{self, Device, RefreshOutcome};
use time::OffsetDateTime;

const ADRESSE: &str = "awa.diallo@example.org";

/// Les durées se comparent **en jours**, à un jour près : l'instant de référence
/// n'est pas le même des deux côtés, et comparer à la seconde ferait échouer le
/// test pour la seule raison que la machine a mis du temps.
fn jours_restants(echeance: OffsetDateTime) -> i64 {
    (echeance - OffsetDateTime::now_utc()).whole_days()
}

#[tokio::test]
async fn lapplication_dure_quatre_vingt_dix_jours_sans_case_a_cocher() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    // `remember_me` vaut `false` : l'écran de l'application n'a pas la case, et
    // c'est exactement ce que le test vérifie — la durée ne vient pas de là.
    let ouverte = connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;

    assert_eq!(jours_restants(ouverte.expires_at), 89);
}

#[tokio::test]
async fn le_site_garde_ses_douze_heures_et_ses_trente_jours() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    let courte = connexion_avec(&bac, ADRESSE, appareil_du_site(), false).await;
    assert_eq!(jours_restants(courte.expires_at), 0);
    assert!((courte.expires_at - OffsetDateTime::now_utc()).whole_hours() >= 11);

    let longue = connexion_avec(&bac, ADRESSE, appareil_du_site(), true).await;
    assert_eq!(jours_restants(longue.expires_at), 29);
}

/// **La durée repart à chaque tour pour l'application, et jamais pour le site.**
/// Les deux moitiés se tiennent : glisser partout ferait des douze heures du
/// site une session éternelle, ne glisser nulle part ferait expirer un compte
/// d'application au milieu de la COP.
#[tokio::test]
async fn lecheance_de_lapplication_repart_a_chaque_rotation() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    let ouverte = connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;

    // On vieillit la session de trente jours : sans glissement, le renouvellement
    // hériterait de cette échéance rapprochée.
    sqlx::query!(
        "UPDATE identity.sessions SET expires_at = expires_at - interval '30 days' WHERE id = $1",
        ouverte.session_id.as_uuid()
    )
    .execute(bac.base.pool())
    .await
    .expect("vieillissement de la session");

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

    assert_eq!(
        jours_restants(neuve.expires_at),
        89,
        "la session d'application repart de quatre-vingt-dix jours"
    );
}

#[tokio::test]
async fn lecheance_du_site_ne_glisse_pas() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    let ouverte = connexion_avec(&bac, ADRESSE, appareil_du_site(), false).await;

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

    assert_eq!(neuve.expires_at, ouverte.expires_at);
}
