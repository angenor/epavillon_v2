//! **Une configuration dont l'intervalle n'excède pas l'anti-rebond refuse le
//! démarrage, en nommant le réglage.**
//!
//! C'est le seul défaut du jalon qui serait **entièrement silencieux** : la
//! chaîne se dédoublonnerait contre elle-même, s'arrêterait, et rien — ni
//! erreur, ni trace, ni message à l'écran — ne le dirait. Seule la fraîcheur
//! affichée cesserait d'avancer, ce que personne ne surveille.

use figment::providers::Serialized;
use figment::Figment;
use kernel::config::Config;

/// Le socle minimal qu'exige `Config::from_figment`, plus les deux réglages
/// éprouvés.
fn configuration(intervalle: &str, anti_rebond: &str) -> Figment {
    Figment::from(Serialized::defaults(serde_json::json!({
        "database_url": "postgres://postgres:dev@localhost:5442/epavillon",
        "auth_signing_key": "cle-de-test-suffisamment-longue-pour-passer",
        "app_public_url": "http://localhost:3000",
        "mail_transport": "smtp",
        "s3_access_key_id": "cle",
        "s3_secret_access_key": "secret",
        "analytics_refresh_interval": intervalle,
        "analytics_refresh_debounce": anti_rebond,
    })))
}

#[test]
fn un_intervalle_egal_a_lanti_rebond_refuse_le_demarrage() {
    let erreur = Config::from_figment(configuration("5m", "5m"))
        .expect_err("à durée égale, la chaîne se dédoublonne contre elle-même");

    let texte = erreur.to_string();
    assert!(
        texte.contains("ANALYTICS_REFRESH_INTERVAL")
            && texte.contains("ANALYTICS_REFRESH_DEBOUNCE"),
        "le refus NOMME les deux réglages : {texte}"
    );
}

#[test]
fn un_intervalle_plus_court_que_lanti_rebond_refuse_aussi() {
    let erreur = Config::from_figment(configuration("1m", "5m")).expect_err("plus court encore");
    assert!(erreur.to_string().contains("ANALYTICS_REFRESH_INTERVAL"));
}

#[test]
fn un_intervalle_nul_refuse_le_demarrage() {
    let erreur = Config::from_figment(configuration("0s", "5m"))
        .expect_err("une cadence nulle replanifierait sans fin");
    assert!(erreur.to_string().contains("ANALYTICS_REFRESH_INTERVAL"));
}

#[test]
fn les_valeurs_par_defaut_du_depot_sont_acceptees() {
    Config::from_figment(configuration("15m", "5m"))
        .expect("quinze minutes contre cinq : la marge d'un rattrapage sans allumer l'alerte");
}
