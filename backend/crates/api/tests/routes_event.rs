//! **Les trente-sept routes du module Événements sont réellement
//! atteignables.**
//!
//! C'est la leçon de B2, où **trois routes sur vingt et une étaient muettes** :
//! deux `web::scope` du même préfixe ne se complètent pas — Actix retient le
//! premier dont le préfixe correspond et rend 404 si la route n'y figure pas,
//! sans essayer le suivant. Rien ne le signalait : ni la compilation, ni les
//! tests des services, qui appellent directement, ni la documentation OpenAPI,
//! qui décrit ce qu'on annote et non ce qui est monté.
//!
//! Ce module pose **quatre** préfixes partagés ou voisins — `/events`,
//! `/admin/events`, `/admin/calls`, `/admin/planner` —, et ce dernier sera repris
//! en entier par B5. Chaque route est donc frappée sur la **vraie application**,
//! intergiciels compris. On ne vérifie qu'une chose : **le chemin existe**. Le
//! comportement, lui, est éprouvé par les tests du module.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;
use uuid::Uuid;

fn id() -> String {
    Uuid::now_v7().to_string()
}

/// Les dix lectures **publiques** : aucune session, aucun refus attendu.
fn chemins_publics() -> Vec<(&'static str, String)> {
    let evenement = id();

    vec![
        ("GET", "/api/events/public".to_owned()),
        ("GET", "/api/events/cop31-belem".to_owned()),
        ("GET", "/api/event-series".to_owned()),
        ("GET", format!("/api/events/{evenement}/days")),
        ("GET", format!("/api/events/{evenement}/tracks")),
        ("GET", format!("/api/events/{evenement}/venues")),
        ("GET", format!("/api/events/{evenement}/rooms")),
        ("GET", format!("/api/events/{evenement}/channels")),
        ("GET", format!("/api/events/{evenement}/call")),
        ("GET", format!("/api/events/{evenement}/images")),
    ]
}

/// Les vingt-sept routes **qui exigent une session**. Le compte est écrit : une
/// route ajoutée sans être montée fait échouer ce test.
fn chemins_gardes() -> Vec<(&'static str, String)> {
    let evenement = id();
    let journee = id();
    let appel = id();
    let fil = id();
    let lieu = id();
    let salle = id();
    let canal = id();

    vec![
        // Sélecteur du back-office — filtré par le périmètre, jamais refusé.
        ("GET", "/api/events".to_owned()),
        // Éditions.
        ("GET", "/api/admin/events/form-options".to_owned()),
        ("GET", "/api/admin/events".to_owned()),
        ("POST", "/api/admin/events".to_owned()),
        ("GET", format!("/api/admin/events/{evenement}")),
        ("PUT", format!("/api/admin/events/{evenement}")),
        // Journées du calendrier — sous le scope de l'édition.
        ("GET", format!("/api/admin/events/{evenement}/days/plan")),
        ("POST", format!("/api/admin/events/{evenement}/days")),
        (
            "PUT",
            format!("/api/admin/events/{evenement}/days/{journee}"),
        ),
        // Fils de programmation.
        ("POST", "/api/admin/tracks".to_owned()),
        ("PUT", format!("/api/admin/tracks/{fil}")),
        ("DELETE", format!("/api/admin/tracks/{fil}")),
        // Lieux et salles.
        ("POST", "/api/admin/venues".to_owned()),
        ("PUT", format!("/api/admin/venues/{lieu}")),
        ("DELETE", format!("/api/admin/venues/{lieu}")),
        ("POST", "/api/admin/rooms".to_owned()),
        ("PUT", format!("/api/admin/rooms/{salle}")),
        ("DELETE", format!("/api/admin/rooms/{salle}")),
        // Canaux de diffusion.
        ("POST", "/api/admin/channels".to_owned()),
        ("PUT", format!("/api/admin/channels/{canal}")),
        ("DELETE", format!("/api/admin/channels/{canal}")),
        // Appel et grille.
        ("GET", "/api/admin/calls/default-criteria".to_owned()),
        ("POST", "/api/admin/calls".to_owned()),
        ("PUT", format!("/api/admin/calls/{appel}")),
        // Comité.
        ("PUT", format!("/api/admin/calls/{appel}/reviewers")),
        // Publication — préfixe partagé avec B5.
        (
            "GET",
            "/api/admin/planner/readiness?event_id=".to_owned() + &id(),
        ),
        ("POST", "/api/admin/planner/publish".to_owned()),
    ]
}

macro_rules! application {
    () => {{
        let base = TestDb::new().await;
        let config = kernel::testing::test_config(base.url());
        let etat = api::state::AppState::new(base.db(), config)
            .await
            .expect("état de l'application");
        std::mem::forget(base);
        test::init_service(api::build_app(&etat)).await
    }};
}

fn requete(verbe: &str) -> test::TestRequest {
    match verbe {
        "GET" => test::TestRequest::get(),
        "POST" => test::TestRequest::post(),
        "PUT" => test::TestRequest::put(),
        "DELETE" => test::TestRequest::delete(),
        autre => panic!("verbe inattendu : {autre}"),
    }
}

#[actix_web::test]
async fn les_trente_sept_routes_du_module_sont_montees() {
    let app = application!();

    let publiques = chemins_publics();
    let gardees = chemins_gardes();
    assert_eq!(
        publiques.len() + gardees.len(),
        37,
        "les trente-sept routes du contrat"
    );

    // **Les lectures publiques répondent sans session.** Un 401 ici voudrait
    // dire qu'un intergiciel les a prises pour des routes de back-office.
    for (verbe, chemin) in publiques {
        let reponse = test::call_service(&app, requete(verbe).uri(&chemin).to_request()).await;

        assert_eq!(
            reponse.status(),
            StatusCode::OK,
            "{verbe} {chemin} devrait répondre sans session"
        );
    }

    // **Les routes gardées refusent, mais elles existent.** Ce qu'aucune ne doit
    // répondre, c'est 404 : ce serait le signe qu'elle n'est pas montée.
    for (verbe, chemin) in gardees {
        let reponse = test::call_service(&app, requete(verbe).uri(&chemin).to_request()).await;

        assert_ne!(
            reponse.status(),
            StatusCode::NOT_FOUND,
            "{verbe} {chemin} n'est pas montée — deux scopes du même préfixe ne se \
             complètent pas, et le second est muet"
        );

        assert!(
            matches!(
                reponse.status(),
                StatusCode::UNAUTHORIZED | StatusCode::UNPROCESSABLE_ENTITY | StatusCode::FORBIDDEN
            ),
            "{verbe} {chemin} a répondu {} — on attendait un refus d'authentification, \
             pas autre chose",
            reponse.status()
        );
    }
}

/// **Le préfixe `/admin/planner` est composé une seule fois**, et il porte déjà
/// les deux routes de ce module. B5 y versera les siennes : ce test est là pour
/// que le jour où il le fera, celles-ci ne deviennent pas muettes.
#[actix_web::test]
async fn le_prefixe_du_planificateur_porte_les_deux_routes_du_module() {
    let app = application!();

    for (verbe, chemin) in [
        (
            "GET",
            format!("/api/admin/planner/readiness?event_id={}", id()),
        ),
        ("POST", "/api/admin/planner/publish".to_owned()),
    ] {
        let reponse = test::call_service(&app, requete(verbe).uri(&chemin).to_request()).await;

        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} devrait exiger une session, pas disparaître"
        );
    }
}
