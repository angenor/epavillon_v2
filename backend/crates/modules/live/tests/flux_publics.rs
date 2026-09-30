//! **Les flux en cours d'une séance publiée : le principal en tête, puis par
//! langue — et une séance non publiée se tait comme une séance inconnue.**

mod commun;

use actix_web::{test, web, App};
use commun::*;
use kernel::error::ErrorCode;
use uuid::Uuid;

async fn publier(bac: &Bac, session_id: Uuid) {
    sqlx::query!(
        "UPDATE programme.sessions SET published_at = now() WHERE id = $1",
        session_id
    )
    .execute(bac.pool())
    .await
    .expect("publication");
}

async fn flux(bac: &Bac, session_id: Uuid, locale: &str, principal: bool, statut: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO live.streams
               (session_id, provider, kind, status, embed_id, locale, is_primary, started_at)
           VALUES ($1, 'youtube', 'live', $4::text::live.stream_status,
                   'video-' || $2::text || '-' || platform.uuid_v7()::text, $2, $3, now())
        RETURNING id"#,
        session_id,
        locale,
        principal,
        statut
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du flux")
}

#[tokio::test]
async fn le_flux_principal_sort_en_tete_et_seuls_les_directs_en_cours_sortent() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    publier(&bac, decor.session_id).await;

    let anglais = flux(&bac, decor.session_id, "en", false, "live").await;
    let original = flux(&bac, decor.session_id, "fr", true, "live").await;
    flux(&bac, decor.session_id, "en", false, "ended").await;
    flux(&bac, decor.session_id, "fr", true, "scheduled").await;

    let lignes = live::repo::streams::en_cours(bac.pool(), decor.session_id)
        .await
        .expect("lecture publique");

    let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
    assert_eq!(ids, vec![original, anglais]);
    assert!(lignes[0].is_primary);
    assert!(lignes[0]
        .embed_url
        .as_deref()
        .is_some_and(|u| u.starts_with("https://www.youtube.com/embed/video-fr-")));
}

#[tokio::test]
async fn une_seance_publiee_sans_direct_rend_une_liste_vide() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    publier(&bac, decor.session_id).await;

    let lignes = live::repo::streams::en_cours(bac.pool(), decor.session_id)
        .await
        .expect("lecture publique");
    assert!(lignes.is_empty());
}

#[tokio::test]
async fn une_seance_non_publiee_et_une_seance_inconnue_rendent_le_meme_404() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    flux(&bac, decor.session_id, "fr", true, "live").await;

    for session_id in [decor.session_id, Uuid::now_v7()] {
        let erreur = live::repo::streams::en_cours(bac.pool(), session_id)
            .await
            .expect_err("séance non publiée");
        assert_eq!(erreur.code, ErrorCode::NotFound);
    }
}

#[tokio::test]
async fn la_route_repond_sous_le_scope_des_seances() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    publier(&bac, decor.session_id).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(bac.state.clone()))
            .service(web::scope("/sessions").configure(live::session_routes)),
    )
    .await;

    let publiee = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/sessions/{}/streams", decor.session_id))
            .to_request(),
    )
    .await;
    assert_eq!(publiee.status(), actix_web::http::StatusCode::OK);

    let brouillon = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/sessions/{}/streams", decor.autre_session_id))
            .to_request(),
    )
    .await;
    assert_eq!(brouillon.status(), actix_web::http::StatusCode::NOT_FOUND);
}
