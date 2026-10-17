//! L'entrée libre : une séance diffusée sans inscription accueille toute
//! personne connectée. Sa réunion naît avec la séance programmée, et la place
//! de chacun est créée chez Zoom pendant la requête d'entrée.

mod commun;

use commun::http::serveur;
use commun::zoom::{publier, reglages};
use commun::{decor, personne, Bac};
use kernel::context::RequestContext;
use kernel::error::ErrorCode;
use kernel::events::{EventConsumer, OutboxEvent};
use live::consumers::sessions::SessionsConsumer;
use live::service::meeting;
use live::state::LiveState;
use live::zoom::Zoom;
use serde_json::json;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

async fn etat_sur(bac: &Bac, quota: bool) -> (LiveState, commun::http::Journal) {
    let (adresse, journal) = serveur(Arc::new(move |methode: &str, chemin: &str| {
        match (methode, chemin.split('?').next().unwrap_or_default()) {
            ("POST", "/oauth/token") => (
                200,
                json!({ "access_token": "jeton", "expires_in": 3600 }).to_string(),
            ),
            ("POST", "/v2/meetings/85746065432/registrants") if quota => (429, "{}".to_owned()),
            ("POST", "/v2/meetings/85746065432/registrants") => (
                201,
                json!({ "registrant_id": "reg-libre",
                        "join_url": "https://zoom.us/w/85746065432?tk=tk-libre" })
                .to_string(),
            ),
            _ => (404, "{}".to_owned()),
        }
    }))
    .await;
    let mut config = reglages();
    config.api_base_url = format!("{adresse}/v2");
    config.oauth_url = format!("{adresse}/oauth/token");
    let etat = bac.state.clone().avec_zoom(Zoom::depuis(&config).unwrap());
    (etat, journal)
}

async fn entree_libre(bac: &Bac, session_id: Uuid) {
    sqlx::query!(
        "UPDATE programme.sessions SET registration_required = false WHERE id = $1",
        session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();
    publier(bac, session_id).await;
}

async fn programmer(bac: &Bac, session_id: Uuid) {
    let evenement = OutboxEvent {
        id: Uuid::now_v7(),
        aggregate_schema: "programme".into(),
        aggregate_type: "session".into(),
        aggregate_id: session_id,
        event_type: "programme.session.scheduled".into(),
        event_version: 1,
        payload: json!({}),
        metadata: json!({}),
        correlation_id: None,
        occurred_at: OffsetDateTime::now_utc(),
    };
    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    SessionsConsumer.handle(&mut tx, &evenement).await.unwrap();
    tx.commit().await.unwrap();
}

async fn simuler_reunion(bac: &Bac, etat: &LiveState, session_id: Uuid) {
    let code = etat.zoom().unwrap().coffre.chiffrer("Libre26").unwrap();
    sqlx::query!(
        "UPDATE live.meetings
            SET external_id = '85746065432', status = 'scheduled', sync_status = 'synced',
                passcode_encrypted = $2
          WHERE session_id = $1",
        session_id,
        code
    )
    .execute(bac.pool())
    .await
    .unwrap();
}

async fn debut(bac: &Bac, session_id: Uuid) -> OffsetDateTime {
    sqlx::query_scalar!(
        "SELECT starts_at FROM programme.sessions WHERE id = $1",
        session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn toute_personne_connectee_entre_sans_inscription() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    entree_libre(&bac, d.session_id).await;
    let (etat, journal) = etat_sur(&bac, false).await;
    let pendant = debut(&bac, d.session_id).await + Duration::minutes(5);
    let karim = personne(&bac, "karim@exemple.org", "Karim", "Ilboudo").await;

    // La séance programmée fait naître sa réunion, sans aucune inscription.
    programmer(&bac, d.session_id).await;
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM live.meetings WHERE session_id = $1 AND sync_status = 'pending'"#,
        d.session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);

    // Pas encore créée chez Zoom : pas prête.
    let erreur = meeting::acces(&etat, &bac.ctx(karim), karim, d.session_id, pendant)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveMeetingNotReady);

    simuler_reunion(&bac, &etat, d.session_id).await;
    let acces = meeting::acces(&etat, &bac.ctx(karim), karim, d.session_id, pendant)
        .await
        .unwrap();
    assert_eq!(acces.registrant_token.as_deref(), Some("tk-libre"));
    assert_eq!(acces.password, "Libre26");
    assert_eq!(acces.user_email, "karim@exemple.org");

    let p = sqlx::query!(
        r#"SELECT registration_id, person_id, sync_status::text AS "s!", external_registrant_id
             FROM live.meeting_participants WHERE person_id = $1"#,
        karim
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert!(p.registration_id.is_none());
    assert_eq!(p.s, "synced");
    assert_eq!(p.external_registrant_id.as_deref(), Some("reg-libre"));
    let travaux = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs WHERE task = 'live.sync_meeting_participant'"#
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(
        travaux, 0,
        "l'inscription chez Zoom s'est faite dans la requête"
    );

    // Une seconde entrée ne rappelle pas Zoom.
    meeting::acces(&etat, &bac.ctx(karim), karim, d.session_id, pendant)
        .await
        .unwrap();
    let appels = journal
        .lock()
        .unwrap()
        .iter()
        .filter(|l| l.starts_with("POST /v2/meetings/85746065432/registrants"))
        .count();
    assert_eq!(appels, 1);
}

#[tokio::test]
async fn sans_reunion_lacces_la_fait_naitre() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    entree_libre(&bac, d.session_id).await;
    let (etat, _) = etat_sur(&bac, false).await;
    let pendant = debut(&bac, d.session_id).await;
    let karim = personne(&bac, "karim@exemple.org", "Karim", "Ilboudo").await;

    let erreur = meeting::acces(&etat, &bac.ctx(karim), karim, d.session_id, pendant)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveMeetingNotReady);
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs j
             JOIN live.meetings m ON j.payload->>'meeting_id' = m.id::text
            WHERE j.task = 'live.sync_meeting' AND m.session_id = $1"#,
        d.session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(
        n, 1,
        "la réunion est posée, sa création chez Zoom part en file"
    );
}

#[tokio::test]
async fn un_refus_de_zoom_laisse_la_place_a_reprendre() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    entree_libre(&bac, d.session_id).await;
    let (etat, _) = etat_sur(&bac, true).await;
    programmer(&bac, d.session_id).await;
    simuler_reunion(&bac, &etat, d.session_id).await;
    let pendant = debut(&bac, d.session_id).await;
    let karim = personne(&bac, "karim@exemple.org", "Karim", "Ilboudo").await;

    let erreur = meeting::acces(&etat, &bac.ctx(karim), karim, d.session_id, pendant)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveMeetingNotReady);
    let statut = sqlx::query_scalar!(
        r#"SELECT sync_status::text AS "s!" FROM live.meeting_participants WHERE person_id = $1"#,
        karim
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(statut, "failed");
}

#[tokio::test]
async fn une_seance_sur_inscription_refuse_toujours() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    publier(&bac, d.session_id).await;
    let (etat, _) = etat_sur(&bac, false).await;
    programmer(&bac, d.session_id).await;
    let pendant = debut(&bac, d.session_id).await;
    let karim = personne(&bac, "karim@exemple.org", "Karim", "Ilboudo").await;

    let erreur = meeting::acces(&etat, &bac.ctx(karim), karim, d.session_id, pendant)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveMeetingRegistrationRequired);
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM live.meetings WHERE session_id = $1"#,
        d.session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(
        n, 0,
        "sans inscription, la séance n'a pas encore de réunion"
    );
}
