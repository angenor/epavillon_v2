//! Un seul compte pour le site et pour l'application (FR-003, SC-004).
//!
//! **C'est la règle qui interdit un second système d'identité.** Une personne
//! inscrite sur l'ePavillon se connecte dans Guide Négo avec la même adresse et
//! le même mot de passe ; l'inverse aussi. Un compte de plus, c'est un mot de
//! passe de plus à oublier et un annuaire de plus à réconcilier — le défaut que
//! la v2 corrige partout ailleurs.

mod commun;

use commun::{
    appareil_de_lapplication, appareil_du_site, connexion_avec, semer, Bac, Compte, MOT_DE_PASSE,
};
use identity::repo::sessions::ClientKind;
use identity::service::registration::{self, RegisterRequest};

const ADRESSE: &str = "awa.diallo@example.org";

fn demande(client: ClientKind) -> RegisterRequest<'static> {
    RegisterRequest {
        first_name: "Awa",
        last_name: "Diallo",
        email: ADRESSE,
        country_id: None,
        password: MOT_DE_PASSE,
        preferred_locale: "fr",
        timezone: "Africa/Dakar",
        client,
    }
}

async fn comptes(bac: &Bac) -> (i64, i64) {
    let personnes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.people
            WHERE primary_email = $1::text::platform.email"#,
        ADRESSE
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des personnes");

    let comptes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.accounts a
             JOIN identity.people p ON p.id = a.person_id
            WHERE p.primary_email = $1::text::platform.email"#,
        ADRESSE
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des comptes");

    (personnes, comptes)
}

/// Inscrite sur le site, elle se connecte depuis l'application — **sans second
/// compte**, et sa session porte bien « app ».
#[tokio::test]
async fn inscrite_sur_le_site_elle_se_connecte_depuis_lapplication() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;

    let ouverte = connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;
    let ligne = commun::ligne_de_session(&bac, ouverte.session_id.as_uuid()).await;

    assert_eq!(ligne.client_kind, "app");
    assert_eq!(comptes(&bac).await, (1, 1));
    assert_eq!(
        commun::acteur_resolu(&bac, &ouverte.access_token).await,
        Some(personne)
    );
}

/// L'inverse : inscrite depuis l'application, elle se connecte sur le site.
#[tokio::test]
async fn inscrite_depuis_lapplication_elle_se_connecte_sur_le_site() {
    let bac = Bac::monter().await;

    registration::register(&bac.state, &bac.ctx(), demande(ClientKind::App))
        .await
        .expect("inscription depuis l'application");

    // L'adresse n'est pas encore vérifiée : c'est la seule chose qui la retient,
    // et elle ne dépend pas du client.
    sqlx::query!(
        "UPDATE identity.people SET email_verified_at = now()
          WHERE primary_email = $1::text::platform.email",
        ADRESSE
    )
    .execute(bac.base.pool())
    .await
    .expect("adresse confirmée");

    let ouverte = connexion_avec(&bac, ADRESSE, appareil_du_site(), false).await;
    let ligne = commun::ligne_de_session(&bac, ouverte.session_id.as_uuid()).await;

    assert_eq!(ligne.client_kind, "web");
    assert_eq!(comptes(&bac).await, (1, 1));
}

/// **Deux inscriptions, une seule personne.** S'inscrire depuis l'application
/// avec une adresse déjà connue du site ne crée rien : c'est un rappel qui part,
/// et la réponse ne change pas de forme (FR-002).
#[tokio::test]
async fn sinscrire_depuis_lapplication_avec_une_adresse_du_site_ne_cree_rien() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    registration::register(&bac.state, &bac.ctx(), demande(ClientKind::App))
        .await
        .expect("inscription");

    assert_eq!(comptes(&bac).await, (1, 1));

    let rappels = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = 'identity.send_existing_account_notice'"#
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des rappels");
    assert_eq!(rappels, 1);
}
