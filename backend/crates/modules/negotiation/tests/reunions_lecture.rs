//! **Lire les réunions** (US1) — publiques, publiées seulement, sans jamais le
//! lien de visioconférence ; `304` ; l'organisateur.

#[macro_use]
mod reunions;

use actix_web::http::header::IF_NONE_MATCH;
use actix_web::http::{Method, StatusCode};
use reunions::{requete, Decor, Reunion, LIEN};

const LISTE: &str = "/negotiation/meetings?edition=cop31";

#[tokio::test]
async fn un_brouillon_nest_jamais_servi_et_une_publiee_lest_sans_son_lien() {
    let d = Decor::monter().await;
    let app = application!(d);
    let brouillon = d
        .reunion(Reunion {
            statut: "draft",
            ..Default::default()
        })
        .await;
    let publiee = d.reunion(Reunion::default()).await;
    let annulee = d
        .reunion(Reunion {
            statut: "cancelled",
            debut: time::OffsetDateTime::now_utc() + time::Duration::days(8),
            ..Default::default()
        })
        .await;

    let r = frapper!(&app, requete(Method::GET, LISTE, None));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    assert_eq!(r.cache.as_deref(), Some("public, no-cache"));
    assert_eq!(r.corps["edition"]["slug"], "cop31");
    assert_eq!(r.corps["edition"]["timezone"], "Asia/Istanbul");
    let ids: Vec<String> = r.corps["meetings"]
        .as_array()
        .expect("liste")
        .iter()
        .map(|m| m["id"].as_str().expect("id").to_owned())
        .collect();
    assert_eq!(
        ids,
        vec![publiee.to_string(), annulee.to_string()],
        "triées par début"
    );
    assert!(!ids.contains(&brouillon.to_string()), "jamais un brouillon");

    let m = &r.corps["meetings"][0];
    assert_eq!(m["has_video"], true);
    assert_eq!(m["type"]["code"], "negotiators_consultation");
    assert!(
        m["type"]["label"]["fr"].is_string(),
        "la nature vient de la base"
    );
    assert_eq!(m["status"], "scheduled");
    assert_eq!(m["format"], "online");
    assert_eq!(m["requires_registration"], true);
    assert_eq!(m["open_access"], true);
    assert_eq!(r.corps["meetings"][1]["status"], "cancelled");
    assert!(r.corps["meetings"][1]["cancellation_reason"].is_string());
    assert!(
        !r.brut.contains(LIEN),
        "aucun lien de visioconférence dans la réponse publique"
    );
    assert!(!r.brut.contains("external_url"));
}

#[tokio::test]
async fn lempreinte_rend_304_et_change_avec_la_liste() {
    let d = Decor::monter().await;
    let app = application!(d);
    d.reunion(Reunion::default()).await;

    let r = frapper!(&app, requete(Method::GET, LISTE, None));
    let etag = r.etag.expect("ETag");
    let r2 = frapper!(
        &app,
        requete(Method::GET, LISTE, None).insert_header((IF_NONE_MATCH, etag.clone()))
    );
    assert_eq!(
        r2.statut,
        StatusCode::NOT_MODIFIED,
        "l'heure de lecture n'entre pas dans l'empreinte"
    );

    d.reunion(Reunion::default()).await;
    let r3 = frapper!(
        &app,
        requete(Method::GET, LISTE, None).insert_header((IF_NONE_MATCH, etag))
    );
    assert_eq!(r3.statut, StatusCode::OK);
    assert_eq!(r3.corps["meetings"].as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn lorganisateur_est_lifdd_ou_le_nom_de_lorganisation() {
    let d = Decor::monter().await;
    let app = application!(d);
    let org = d.organisation("Réseau ouest-africain pour le climat").await;
    d.reunion(Reunion::default()).await;
    d.reunion(Reunion {
        organisation: Some(org),
        debut: time::OffsetDateTime::now_utc() + time::Duration::days(9),
        ..Default::default()
    })
    .await;

    let r = frapper!(&app, requete(Method::GET, LISTE, None));
    assert_eq!(r.corps["meetings"][0]["organizer"], "IFDD");
    assert_eq!(
        r.corps["meetings"][1]["organizer"],
        "Réseau ouest-africain pour le climat"
    );
}

#[tokio::test]
async fn une_edition_inconnue_rend_404() {
    let d = Decor::monter().await;
    let app = application!(d);
    let r = frapper!(
        &app,
        requete(Method::GET, "/negotiation/meetings?edition=cop99", None)
    );
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
    assert_eq!(r.corps["code"], "NEGOTIATION_EDITION_UNKNOWN");
}
