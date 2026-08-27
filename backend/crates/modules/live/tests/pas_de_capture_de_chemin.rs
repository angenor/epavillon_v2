//! **`/events/{id}/incidents` ne capture pas `/events/{slug}`.**
//!
//! Le module `event` déclare ses routes `/events/…` à plat, et `/events/{slug}`
//! porte **un** segment là où la lecture publique des messages en porte **deux**.
//! Les motifs ne se recouvrent donc pas — vérifié plutôt que supposé.

mod commun;

use actix_web::{test, web, App};
use commun::*;

#[tokio::test]
async fn la_route_a_deux_segments_ne_prend_pas_la_place_de_celle_a_un() {
    let bac = Bac::monter().await;

    // On monte la route du module **et** une route à un segment jouant le rôle
    // de `/events/{slug}` du module `event` : si l'une capturait l'autre, la
    // seconde ne répondrait jamais.
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(bac.state.clone()))
            .configure(live::event_routes)
            .route(
                "/events/{slug}",
                web::get().to(|| async { actix_web::HttpResponse::Ok().body("edition") }),
            ),
    )
    .await;

    let edition = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/events/cop31-belem")
            .to_request(),
    )
    .await;
    assert_eq!(
        edition.status(),
        actix_web::http::StatusCode::OK,
        "la fiche d'édition continue de répondre"
    );

    let incidents = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/events/01a04136-0000-7000-8000-000000000000/incidents")
            .to_request(),
    )
    .await;
    assert_eq!(
        incidents.status(),
        actix_web::http::StatusCode::OK,
        "et la lecture publique aussi — une liste vide, sans garde"
    );
}
