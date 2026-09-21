//! La session dit d'où elle vient, et de quel appareil (ADR-001, FR-005).
//!
//! `user_agent` ne distingue pas le site de l'application : le navigateur d'un
//! téléphone annonce la même chose dans les deux cas. Sans ces colonnes, compter
//! les personnes qui utilisent Guide Négo est impossible, et la durée de session
//! ne peut pas suivre le client.

mod commun;

use commun::{appareil_de_lapplication, appareil_du_site, connexion_avec, semer, Bac, Compte};

const ADRESSE: &str = "awa.diallo@example.org";

#[tokio::test]
async fn une_session_ouverte_depuis_lapplication_porte_son_appareil() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    let ouverte = connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;
    let ligne = commun::ligne_de_session(&bac, ouverte.session_id.as_uuid()).await;

    assert_eq!(ligne.client_kind, "app");
    assert_eq!(ligne.device_id.as_deref(), Some("9f2c-appareil-de-test"));
    assert_eq!(ligne.device_label.as_deref(), Some("Android · Chrome"));
    assert_eq!(ligne.device_platform.as_deref(), Some("android"));
}

/// **Non-régression du site.** Il n'envoie aucun objet `client` : la colonne
/// prend son défaut, et rien de ses appels ne change.
#[tokio::test]
async fn une_session_du_site_reste_web_sans_appareil() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    let ouverte = connexion_avec(&bac, ADRESSE, appareil_du_site(), false).await;
    let ligne = commun::ligne_de_session(&bac, ouverte.session_id.as_uuid()).await;

    assert_eq!(ligne.client_kind, "web");
    assert_eq!(ligne.device_id, None);
    assert_eq!(ligne.device_label, None);
    assert_eq!(ligne.device_platform, None);
}

/// Le décompte que FR-005 demande : combien de personnes se servent de
/// l'application, **sans en dédoubler aucune**. Deux téléphones d'une même
/// personne font deux sessions et une personne.
#[tokio::test]
async fn les_sessions_dapplication_se_comptent_sans_dedoubler_personne() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;
    semer(&bac, Compte::actif("boureima.ouedraogo@example.org")).await;

    connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;
    connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;
    connexion_avec(
        &bac,
        "boureima.ouedraogo@example.org",
        appareil_de_lapplication(),
        false,
    )
    .await;
    connexion_avec(&bac, ADRESSE, appareil_du_site(), false).await;

    let personnes = sqlx::query_scalar!(
        r#"SELECT count(DISTINCT person_id) AS "compte!"
             FROM identity.sessions WHERE client_kind = 'app'"#
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");

    assert_eq!(personnes, 2);
}
