//! **`/admin/incidents/overrun-template` n'est pas lu comme un identifiant.**
//!
//! Les deux routes sont en `GET` et partagent le même préfixe : déclarée après
//! `/admin/incidents/{id}`, la littérale serait capturée comme un UUID — et le
//! raccourci « Signaler un débordement » rendrait « message introuvable ».
//!
//! Le module les sépare en deux blocs, `chemins_litteraux` puis
//! `chemins_de_dossier`, pour que la règle soit tenue **par la structure** et
//! non par la vigilance de qui ajoute la prochaine route. Ce test frappe les
//! deux chemins sur un routeur monté avec les routes du module, exactement dans
//! l'ordre où il les déclare.

mod commun;

use actix_web::{test, web, App};
use commun::*;

#[tokio::test]
async fn le_chemin_litteral_nest_pas_capture_par_le_chemin_parametre() {
    let bac = Bac::monter().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(bac.state.clone()))
            .configure(live::routes),
    )
    .await;

    // Sans session, la garde de périmètre refuse — mais **le refus dit quelle
    // route a répondu** : un 404 « message introuvable » signalerait que le
    // chemin littéral a été pris pour un identifiant, alors qu'un 401 dit que
    // la route du gabarit a bien été atteinte et s'est arrêtée sur la session.
    let requete = test::TestRequest::get()
        .uri("/admin/incidents/overrun-template?session_id=00000000-0000-0000-0000-000000000000")
        .to_request();
    let reponse = test::call_service(&app, requete).await;

    assert_ne!(
        reponse.status(),
        actix_web::http::StatusCode::NOT_FOUND,
        "le chemin littéral doit être ROUTÉ, pas lu comme un identifiant"
    );
}

#[tokio::test]
async fn le_chemin_parametre_repond_toujours() {
    let bac = Bac::monter().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(bac.state.clone()))
            .configure(live::routes),
    )
    .await;

    let requete = test::TestRequest::get()
        .uri("/admin/incidents/01a04136-0000-7000-8000-000000000000")
        .to_request();
    let reponse = test::call_service(&app, requete).await;

    // Même raisonnement : la route existe et s'arrête sur la session absente.
    assert_ne!(reponse.status(), actix_web::http::StatusCode::NOT_FOUND);
}
