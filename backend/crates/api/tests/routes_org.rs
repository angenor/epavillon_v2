//! **Les vingt et une routes du module Organisations sont réellement
//! atteignables.**
//!
//! Ce test existe à cause d'un défaut qui s'est produit : deux
//! `web::scope("/organizations")` étaient enregistrés séparément — l'un pour les
//! lectures ouvertes, l'autre pour les adhésions. **Actix retient le premier
//! dont le préfixe correspond et rend 404 si la route n'y figure pas** : il
//! n'essaie pas le suivant. Trois routes étaient donc muettes, et **rien ne le
//! signalait** — ni la compilation, ni les tests des services, qui les appellent
//! directement, ni la documentation OpenAPI, qui décrit ce qu'on annote et non
//! ce qui est monté.
//!
//! C'est la leçon de B1 répétée : **rien ne traversait HTTP**. Un test qui
//! appelle un service n'éprouve pas le montage.
//!
//! Chaque route est donc frappée sur la **vraie application**, intergiciels
//! compris. On ne vérifie qu'une chose : **le chemin existe**. Le comportement,
//! lui, est éprouvé par les tests du module.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;
use uuid::Uuid;

/// Un identifiant quelconque : la route doit exister, pas répondre juste.
fn id() -> String {
    Uuid::now_v7().to_string()
}

/// Les vingt et une routes, avec leur verbe. Le compte est écrit : une route
/// ajoutée sans être montée fait échouer ce test.
fn chemins() -> Vec<(&'static str, String)> {
    let org = id();
    let membership = id();
    let domain = id();
    let name = id();
    let pair = id();
    let person = id();

    vec![
        ("GET", "/api/organizations/similar?name=ifdd".to_owned()),
        ("GET", "/api/organizations/by-email-domain".to_owned()),
        ("GET", "/api/organizations".to_owned()),
        ("POST", "/api/organizations".to_owned()),
        ("GET", format!("/api/organizations/{org}")),
        ("POST", format!("/api/organizations/{org}/members")),
        ("POST", format!("/api/organizations/{org}/invitations")),
        ("POST", "/api/organizations/invitations/accept".to_owned()),
        ("PUT", format!("/api/memberships/{membership}/decision")),
        ("DELETE", format!("/api/memberships/{membership}")),
        ("GET", format!("/api/people/{person}/memberships")),
        (
            "GET",
            "/api/admin/organizations/similar?name=ifdd".to_owned(),
        ),
        ("GET", "/api/admin/organizations".to_owned()),
        ("GET", format!("/api/admin/organizations/{org}")),
        (
            "PUT",
            format!("/api/admin/organizations/{org}/verification"),
        ),
        (
            "PUT",
            format!("/api/admin/organizations/{org}/domains/{domain}"),
        ),
        (
            "PUT",
            format!("/api/admin/organizations/{org}/names/{name}"),
        ),
        ("GET", "/api/admin/organizations/duplicates".to_owned()),
        ("PUT", format!("/api/admin/organizations/duplicates/{pair}")),
        (
            "GET",
            format!(
                "/api/admin/organizations/{org}/merge-preview?target_id={}",
                id()
            ),
        ),
        ("POST", "/api/admin/organizations/merge".to_owned()),
    ]
}

#[actix_web::test]
async fn les_vingt_et_une_routes_du_module_sont_montees() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let routes = chemins();
    assert_eq!(routes.len(), 21, "les vingt et une routes du contrat");

    for (verbe, chemin) in routes {
        let requete = match verbe {
            "GET" => test::TestRequest::get(),
            "POST" => test::TestRequest::post(),
            "PUT" => test::TestRequest::put(),
            "DELETE" => test::TestRequest::delete(),
            autre => panic!("verbe inattendu : {autre}"),
        };

        let reponse = test::call_service(&app, requete.uri(&chemin).to_request()).await;

        // **Aucune session** : toutes ces routes doivent répondre 401, sauf
        // l'acceptation d'une invitation qui n'en exige pas — elle répond 422,
        // son corps étant absent. Ce qu'aucune ne doit répondre, c'est **404** :
        // ce serait le signe qu'elle n'est pas montée.
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

/// Les routes de `identity` sous `/people` **restent atteignables** après que le
/// module Organisations y a ajouté les siennes. C'est l'autre moitié du même
/// piège : composer le scope aurait pu masquer les premières.
#[actix_web::test]
async fn les_routes_didentite_sous_people_survivent_a_la_composition() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let person = id();
    for chemin in [
        "/api/people".to_owned(),
        format!("/api/people/{person}"),
        format!("/api/people/{person}/roles"),
        format!("/api/people/{person}/permissions"),
        format!("/api/people/{person}/administered-events"),
        format!("/api/people/{person}/memberships"),
    ] {
        let reponse =
            test::call_service(&app, test::TestRequest::get().uri(&chemin).to_request()).await;

        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "GET {chemin} devrait exiger une session, pas disparaître"
        );
    }
}
