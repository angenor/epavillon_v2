//! Les travaux contre une API Zoom locale : l'inscrit qui arrive avant la
//! réunion attend, la création de la réunion le relance, puis il reçoit son
//! lien personnel. Le code d'accès est chiffré, la réponse conservée sans secret.

mod commun;

use commun::http::serveur;
use commun::zoom::{consommer, inscrire, reglages};
use commun::{decor, personne, Bac};
use kernel::jobs::{ClaimedJob, JobHandler};
use live::jobs::sync_meeting::SyncMeeting;
use live::jobs::sync_participant::{SyncParticipant, PAS_ENCORE_CREEE};
use live::zoom::Zoom;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

fn api(methode: &str, chemin: &str) -> (u16, String) {
    match (methode, chemin.split('?').next().unwrap_or_default()) {
        ("POST", "/oauth/token") => (
            200,
            json!({ "access_token": "jeton", "expires_in": 3600 }).to_string(),
        ),
        ("POST", "/v2/users/me/meetings") => (
            201,
            json!({
                "id": 85746065432u64, "uuid": "occ==", "host_id": "hote", "host_email": "pavillon@ifdd.org",
                "password": "Pav26", "encrypted_password": "chiffre",
                "start_url": "https://zoom.us/s/85746065432?zak=secret",
                "join_url": "https://zoom.us/j/85746065432?pwd=chiffre",
                "registration_url": "https://zoom.us/meeting/register/abc",
                "settings": { "approval_type": 0 }
            })
            .to_string(),
        ),
        ("POST", "/v2/meetings/85746065432/registrants") => (
            201,
            json!({ "registrant_id": "reg-awa", "join_url": "https://zoom.us/w/85746065432?tk=tk-awa" })
                .to_string(),
        ),
        _ => (404, "{}".to_owned()),
    }
}

fn travail(tache: &str, charge: Value) -> ClaimedJob {
    ClaimedJob {
        id: Uuid::now_v7(),
        queue: "live".into(),
        task: tache.into(),
        payload: charge,
        attempts: 1,
        max_attempts: 5,
    }
}

#[tokio::test]
async fn la_reunion_puis_linscrit_sont_crees_chez_zoom() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let (adresse, journal) = serveur(std::sync::Arc::new(api)).await;
    let mut config = reglages();
    config.api_base_url = format!("{adresse}/v2");
    config.oauth_url = format!("{adresse}/oauth/token");
    let zoom = Arc::new(Zoom::depuis(&config).unwrap());

    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let r = inscrire(&bac, d.session_id, awa).await;
    consommer(&bac, r).await;
    let (meeting_id, participant_id) = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT meeting_id, id FROM live.meeting_participants WHERE registration_id = $1",
    )
    .bind(r)
    .fetch_one(bac.pool())
    .await
    .unwrap();

    let inscrit = SyncParticipant::new(bac.db(), zoom.clone());
    let reunion = SyncMeeting::new(bac.db(), zoom.clone());
    let travail_inscrit = travail(
        "live.sync_meeting_participant",
        json!({ "participant_id": participant_id }),
    );

    // L'inscrit passe avant la réunion : il attend, sans échec du travail.
    inscrit.run(&travail_inscrit).await.unwrap();
    let attente = sqlx::query!(
        r#"SELECT sync_status::text AS "s!", last_error FROM live.meeting_participants WHERE id = $1"#,
        participant_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(attente.s, "failed");
    assert_eq!(attente.last_error.as_deref(), Some(PAS_ENCORE_CREEE));

    reunion
        .run(&travail(
            "live.sync_meeting",
            json!({ "meeting_id": meeting_id }),
        ))
        .await
        .unwrap();
    let m = sqlx::query!(
        r#"SELECT external_id, status::text AS "status!", sync_status::text AS "sync!",
                  passcode_encrypted, start_url_encrypted, provider_payload,
                  join_url::text AS join_url, host_email::text AS host_email
             FROM live.meetings WHERE id = $1"#,
        meeting_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(m.external_id.as_deref(), Some("85746065432"));
    assert_eq!(m.status, "scheduled");
    assert_eq!(m.sync, "synced");
    assert_eq!(m.join_url.as_deref(), Some("https://zoom.us/j/85746065432"));
    assert_eq!(m.host_email.as_deref(), Some("pavillon@ifdd.org"));
    assert_eq!(
        zoom.coffre
            .dechiffrer(&m.passcode_encrypted.unwrap())
            .unwrap(),
        "Pav26"
    );
    assert!(zoom
        .coffre
        .dechiffrer(&m.start_url_encrypted.unwrap())
        .unwrap()
        .contains("zak="));
    let gardee = m.provider_payload.to_string();
    assert!(!gardee.contains("pwd="), "le lien générique perd son pwd");
    assert!(!gardee.contains("Pav26") && !gardee.contains("zak=") && !gardee.contains("chiffre"));

    // La création de la réunion a relancé l'inscrit.
    let relances = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = 'live.sync_meeting_participant' AND idempotency_key LIKE $1 || ':%'"#,
        participant_id.to_string()
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(relances, 1);

    inscrit.run(&travail_inscrit).await.unwrap();
    let p = sqlx::query!(
        r#"SELECT sync_status::text AS "s!", external_registrant_id,
                  personal_join_url::text AS lien
             FROM live.meeting_participants WHERE id = $1"#,
        participant_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(p.s, "synced");
    assert_eq!(p.external_registrant_id.as_deref(), Some("reg-awa"));
    assert_eq!(
        p.lien.as_deref(),
        Some("https://zoom.us/w/85746065432?tk=tk-awa")
    );

    // Une seconde fois : rien ne repart chez Zoom.
    reunion
        .run(&travail(
            "live.sync_meeting",
            json!({ "meeting_id": meeting_id }),
        ))
        .await
        .unwrap();
    let creations = journal
        .lock()
        .unwrap()
        .iter()
        .filter(|l| l.starts_with("POST /v2/users/me/meetings"))
        .count();
    assert_eq!(creations, 1);
    let inscription = journal
        .lock()
        .unwrap()
        .iter()
        .find(|l| l.starts_with("POST /v2/meetings/85746065432/registrants"))
        .cloned()
        .unwrap();
    assert!(inscription.contains("awa.diallo@exemple.org") && inscription.contains("Diallo"));
    let creation = journal
        .lock()
        .unwrap()
        .iter()
        .find(|l| l.starts_with("POST /v2/users/me/meetings"))
        .cloned()
        .unwrap();
    assert!(creation.contains(&format!("Réf. ePavillon {meeting_id}")));
}

async fn reunion_et_inscrit(bac: &Bac, session_id: Uuid) -> (Uuid, Uuid) {
    let awa = personne(bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let r = inscrire(bac, session_id, awa).await;
    consommer(bac, r).await;
    sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT meeting_id, id FROM live.meeting_participants WHERE registration_id = $1",
    )
    .bind(r)
    .fetch_one(bac.pool())
    .await
    .unwrap()
}

fn zoom_sur(adresse: &str) -> Arc<Zoom> {
    let mut config = reglages();
    config.api_base_url = format!("{adresse}/v2");
    config.oauth_url = format!("{adresse}/oauth/token");
    Arc::new(Zoom::depuis(&config).unwrap())
}

#[tokio::test]
async fn un_essai_perdu_retrouve_la_reunion_deja_creee() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let (meeting_id, _) = reunion_et_inscrit(&bac, d.session_id).await;
    // Un premier essai a envoyé la création, sa réponse s'est perdue.
    sqlx::query!(
        "UPDATE live.meetings
            SET provider_payload = jsonb_build_object('creation_envoyee', now())
          WHERE id = $1",
        meeting_id
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let marque = live::jobs::sync_meeting::reference(meeting_id);
    let (adresse, journal) = serveur(Arc::new(move |methode: &str, chemin: &str| {
        match (methode, chemin.split('?').next().unwrap_or_default()) {
            ("POST", "/oauth/token") => (
                200,
                json!({ "access_token": "jeton", "expires_in": 3600 }).to_string(),
            ),
            ("GET", "/v2/users/me/meetings") => (
                200,
                json!({ "meetings": [
                    { "id": 111, "agenda": "autre" },
                    { "id": 777, "agenda": marque },
                ], "next_page_token": "" })
                .to_string(),
            ),
            ("PATCH", "/v2/meetings/777") => (204, String::new()),
            ("GET", "/v2/meetings/777") => (
                200,
                json!({ "id": 777, "password": "p", "join_url": "https://zoom.us/j/777",
                        "settings": { "approval_type": 0 } })
                .to_string(),
            ),
            _ => (404, "{}".to_owned()),
        }
    }))
    .await;

    SyncMeeting::new(bac.db(), zoom_sur(&adresse))
        .run(&travail(
            "live.sync_meeting",
            json!({ "meeting_id": meeting_id }),
        ))
        .await
        .unwrap();

    let numero = sqlx::query_scalar!(
        "SELECT external_id FROM live.meetings WHERE id = $1",
        meeting_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(numero.as_deref(), Some("777"));
    assert!(
        !journal
            .lock()
            .unwrap()
            .iter()
            .any(|l| l.starts_with("POST /v2/users/me/meetings")),
        "aucune seconde réunion créée"
    );
}

#[tokio::test]
async fn un_quota_repousse_linscrit_sans_faire_echouer_le_travail() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let (meeting_id, participant_id) = reunion_et_inscrit(&bac, d.session_id).await;
    sqlx::query!(
        "UPDATE live.meetings SET external_id = '1', status = 'scheduled', sync_status = 'synced'
          WHERE id = $1",
        meeting_id
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let (adresse, _) = serveur(Arc::new(|methode: &str, chemin: &str| {
        match (methode, chemin.split('?').next().unwrap_or_default()) {
            ("POST", "/oauth/token") => (
                200,
                json!({ "access_token": "jeton", "expires_in": 3600 }).to_string(),
            ),
            _ => (429, "{}".to_owned()),
        }
    }))
    .await;

    SyncParticipant::new(bac.db(), zoom_sur(&adresse))
        .run(&travail(
            "live.sync_meeting_participant",
            json!({ "participant_id": participant_id }),
        ))
        .await
        .unwrap();

    let p = sqlx::query!(
        r#"SELECT sync_status::text AS "s!", next_retry_at > now() + interval '119 seconds' AS "attend!"
             FROM live.meeting_participants WHERE id = $1"#,
        participant_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(p.s, "failed");
    assert!(p.attend, "Retry-After est respecté");
    let prochains = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = 'live.sync_meeting_participant' AND status = 'queued'
              AND run_at > now() + interval '119 seconds'
              AND idempotency_key LIKE $1 || ':%'"#,
        participant_id.to_string()
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(prochains, 1);
}
