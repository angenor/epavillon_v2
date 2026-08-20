//! **`/events/public` rend une liste, jamais le `null` d'une adresse inconnue**
//! (research.md § R11).
//!
//! Actix retient la **première** route dont le motif correspond. Déclarée après
//! `GET /events/{slug}`, la liste des éditions publiques serait lue comme
//! l'adresse d'URL « public » — et rendrait `null` avec un statut 200. Rien ne
//! le signalerait : ni la compilation, ni les tests du module, qui appellent les
//! services et ne traversent aucun routeur.
//!
//! C'est exactement la forme du défaut de B2, où trois routes sur vingt et une
//! étaient muettes. Ici le symptôme serait plus discret encore : une page
//! d'accueil vide, sans erreur.
//!
//! Ce test frappe donc la **vraie application**, intergiciels compris, et
//! vérifie la **forme** de la réponse — un tableau, pas `null`.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;

/// Monter la vraie application sur une base jetable.
///
/// Une macro et non une fonction : le type que rend `init_service` est opaque et
/// mentionne `actix_http::Request`, un crate que l'API ne déclare pas. L'écrire
/// en signature demanderait une dépendance de plus pour trois lignes.
macro_rules! application {
    () => {{
        let base = TestDb::new().await;
        let config = kernel::testing::test_config(base.url());
        let etat = api::state::AppState::new(base.db(), config)
            .await
            .expect("état de l'application");
        // La base jetable doit survivre à l'application qui s'en sert : on la
        // confie à l'exécutable du test. Elle est détruite avec le serveur du
        // bac à sable.
        std::mem::forget(base);
        test::init_service(api::build_app(&etat)).await
    }};
}

#[actix_web::test]
async fn events_public_rend_une_liste_et_non_le_null_dune_adresse_inconnue() {
    let app = application!();

    let reponse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/events/public")
            .to_request(),
    )
    .await;

    assert_eq!(
        reponse.status(),
        StatusCode::OK,
        "la liste publique ne demande aucune session"
    );

    let corps: serde_json::Value = test::read_body_json(reponse).await;
    assert!(
        corps.is_array(),
        "« public » a été capturé comme une adresse d'URL : la réponse est {corps}"
    );
}

/// L'autre moitié du même piège : le chemin paramétré **fonctionne toujours**.
/// Déplacer une route littérale devant lui ne doit pas l'éteindre.
#[actix_web::test]
async fn une_adresse_inconnue_rend_bien_null() {
    let app = application!();

    let reponse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/events/cette-edition-nexiste-pas")
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::OK);

    let corps: serde_json::Value = test::read_body_json(reponse).await;
    assert!(
        corps.is_null(),
        "une adresse inconnue rend null, indiscernable d'un brouillon : {corps}"
    );
}

/// **Le sélecteur du back-office reste distinct de la liste publique.** Les deux
/// vivent sous `/events` ; l'un exige une session, l'autre non.
#[actix_web::test]
async fn le_selecteur_du_back_office_exige_toujours_une_session() {
    let app = application!();

    let reponse = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/events").to_request(),
    )
    .await;

    assert_eq!(
        reponse.status(),
        StatusCode::UNAUTHORIZED,
        "le sélecteur d'édition est borné par le périmètre, donc par la session"
    );
}
