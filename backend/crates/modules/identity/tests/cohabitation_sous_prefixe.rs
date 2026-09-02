//! **Les cookies suivent le chemin sous lequel le site est servi.**
//!
//! Le défaut que ce test ferme est entièrement silencieux : servi sous `/v2`,
//! le navigateur appelle `/v2/api/auth/refresh`, mais un cookie de
//! rafraîchissement posé sur `/api/auth` ne lui est **jamais** renvoyé. La
//! connexion réussit, la navigation fonctionne quinze minutes — la durée du
//! jeton d'accès —, puis tout le monde se retrouve déconnecté. Aucune erreur,
//! aucune trace en journal, rien à l'écran qui l'explique.
//!
//! Il vérifie aussi l'autre moitié du même réglage : l'origine autorisée ne
//! porte PAS le préfixe, parce qu'un navigateur n'en met jamais dans son
//! en-tête `Origin`. Les deux valeurs viennent d'`APP_PUBLIC_URL` et se
//! déduisent en sens contraire ; les tenir séparément les ferait diverger.

use kernel::config::Config;

fn configuration(url_publique: &str) -> Config {
    Config::from_figment(figment::Figment::from(
        figment::providers::Serialized::defaults(serde_json::json!({
            "database_url": "postgres://postgres:dev@localhost:5432/epavillon",
            "auth_signing_key": "cle-de-test-suffisamment-longue-pour-passer",
            "app_public_url": url_publique,
            // Le courriel n'a rien à voir avec ce test, mais le transport par
            // défaut exige son relais : sans ces deux clés, c'est lui qui
            // refuserait la configuration.
            "mail_transport": "smtp",
            "smtp_host": "localhost",
            "smtp_from": "ne-pas-repondre@epavillon.local",
            "s3_access_key_id": "cle",
            "s3_secret_access_key": "secret",
        })),
    ))
    .expect("configuration valide")
}

#[test]
fn sous_un_prefixe_les_deux_cookies_le_portent() {
    let config = configuration("https://epavillonclimatique.francophonie.org/v2");

    let acces = identity::routes::cookies::acces(
        &config,
        "jeton".into(),
        std::time::Duration::from_secs(60),
    );
    let rafraichissement = identity::routes::cookies::rafraichissement(
        &config,
        "jeton".into(),
        time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    );

    assert_eq!(acces.path(), Some("/v2/"));
    assert_eq!(rafraichissement.path(), Some("/v2/api/auth"));
}

/// L'effacement doit viser **la même portée** que la pose : un chemin différent
/// laisserait un second cookie, invisible et vivant.
#[test]
fn leffacement_vise_la_meme_portee() {
    let config = configuration("https://epavillonclimatique.francophonie.org/v2");
    let [acces, rafraichissement] = identity::routes::cookies::effacer(&config);

    assert_eq!(acces.path(), Some("/v2/"));
    assert_eq!(rafraichissement.path(), Some("/v2/api/auth"));
}

/// À la racine, rien ne change — la cohabitation ne doit rien coûter au jour où
/// elle prendra fin.
#[test]
fn a_la_racine_les_chemins_sont_inchanges() {
    let config = configuration("https://epavillonclimatique.francophonie.org");
    let [acces, rafraichissement] = identity::routes::cookies::effacer(&config);

    assert_eq!(acces.path(), Some("/"));
    assert_eq!(rafraichissement.path(), Some("/api/auth"));
}

/// Un navigateur n'annonce jamais de chemin dans son en-tête `Origin` : la
/// comparer à l'URL complète refuserait **toute écriture**, sans rien dire de
/// plus qu'un refus d'origine.
#[test]
fn lorigine_autorisee_ne_porte_pas_le_prefixe() {
    let config = configuration("https://epavillonclimatique.francophonie.org/v2/");

    assert_eq!(
        config.app_public_origin,
        "https://epavillonclimatique.francophonie.org"
    );
    assert_eq!(config.app_base_path, "/v2");
    // Les liens des courriels, eux, gardent le préfixe : sans lui, un lien de
    // vérification d'adresse mènerait à la v1.
    assert_eq!(
        config.app_public_url,
        "https://epavillonclimatique.francophonie.org/v2"
    );
}
