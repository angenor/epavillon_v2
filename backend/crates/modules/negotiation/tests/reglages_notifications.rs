//! **Les réglages de notification** (3b, US5) — l'accord « Notifications »
//! (allumé sans ligne, une preuve par bascule, la version servie) et les
//! thématiques dont on est prévenu (parmi les suivies, éteintes en quittant,
//! comprises dans l'empreinte).

mod importation;
#[macro_use]
mod decision;

use actix_web::http::StatusCode;
use decision::{lire, monter, Decor, ACTEUR};
use serde_json::{json, Value};
use uuid::Uuid;

fn ecrire(acteur: Uuid, uri: &str, corps: Value) -> actix_web::test::TestRequest {
    actix_web::test::TestRequest::put()
        .uri(uri)
        .insert_header((ACTEUR, acteur.to_string()))
        .set_json(corps)
}

async fn preuves(d: &Decor) -> Vec<(bool, String)> {
    sqlx::query_as(
        "SELECT is_granted, policy_version FROM identity.consents
          WHERE person_id = $1 AND purpose = 'guide_nego_notifications' ORDER BY recorded_at",
    )
    .bind(d.awa)
    .fetch_all(d.bac.pool())
    .await
    .expect("preuves")
}

#[tokio::test]
async fn laccord_est_allume_par_defaut_et_chaque_bascule_laisse_sa_preuve() {
    let d = monter().await;
    let app = application!(d);
    let version = kernel::legal::version("privacy");
    const URI: &str = "/negotiation/me/notifications";

    let (statut, r, _) = frapper!(&app, lire(d.awa, URI));
    assert_eq!(statut, StatusCode::OK, "{r}");
    assert_eq!(r, json!({ "email": true, "version": version }));
    let (statut, _, _) = frapper!(&app, actix_web::test::TestRequest::get().uri(URI));
    assert_eq!(statut, StatusCode::UNAUTHORIZED);

    let (_, r, _) = frapper!(&app, ecrire(d.awa, URI, json!({ "email": false })));
    assert_eq!(r["email"], false);
    frapper!(&app, ecrire(d.awa, URI, json!({ "email": false })));
    assert_eq!(
        preuves(&d).await,
        [(false, version.to_owned())],
        "rejouer n'écrit rien"
    );
    let (_, r, _) = frapper!(&app, lire(d.awa, URI));
    assert_eq!(r["email"], false);

    frapper!(&app, ecrire(d.awa, URI, json!({ "email": true })));
    assert_eq!(preuves(&d).await.len(), 2);
    let (_, r, _) = frapper!(&app, lire(d.awa, URI));
    assert_eq!(r["email"], true);
}

#[tokio::test]
async fn les_thematiques_de_notification_parmi_les_suivies() {
    let d = monter().await;
    let app = application!(d);
    const THEMES: &str = "/negotiation/me/themes";
    const NOTIF: &str = "/negotiation/me/themes/notifications";

    let (_, r, avant) = frapper!(
        &app,
        ecrire(d.awa, THEMES, json!({ "codes": ["adaptation", "finance"] }))
    );
    assert_eq!(r["notify"], json!([]), "éteinte par défaut");
    let avant = avant.expect("empreinte");

    let (statut, r, apres) = frapper!(&app, ecrire(d.awa, NOTIF, json!({ "codes": ["finance"] })));
    assert_eq!(statut, StatusCode::OK, "{r}");
    assert_eq!(r["notify"], json!(["finance"]));
    let apres = apres.expect("empreinte");
    assert_ne!(apres, avant, "l'empreinte suit notify");

    let (statut, r, _) = frapper!(
        &app,
        lire(d.awa, THEMES).insert_header(("If-None-Match", avant.clone()))
    );
    assert_eq!(statut, StatusCode::OK, "un autre appareil relit le réglage");
    assert_eq!(r["notify"], json!(["finance"]));
    let (statut, _, _) = frapper!(
        &app,
        ecrire(d.awa, THEMES, json!({ "codes": ["finance"] }))
            .insert_header(("If-Match", avant.clone()))
    );
    assert_eq!(statut, StatusCode::PRECONDITION_FAILED);

    let (statut, r, _) = frapper!(
        &app,
        ecrire(d.awa, NOTIF, json!({ "codes": ["mitigation"] }))
    );
    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert_eq!(r["code"], "NEGOTIATION_THEME_UNKNOWN");

    let (_, r, _) = frapper!(&app, ecrire(d.awa, NOTIF, json!({ "codes": ["finance"] })));
    assert_eq!(r["notify"], json!(["finance"]), "idempotent");

    let (_, r, _) = frapper!(
        &app,
        ecrire(d.awa, THEMES, json!({ "codes": ["adaptation"] }))
    );
    assert_eq!(r["notify"], json!([]), "quitter l'éteint");
    let (_, r, _) = frapper!(
        &app,
        ecrire(d.awa, THEMES, json!({ "codes": ["adaptation", "finance"] }))
    );
    assert_eq!(
        r["notify"],
        json!([]),
        "la suivre à nouveau ne la rallume pas"
    );
}
