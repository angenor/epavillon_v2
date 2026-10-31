//! Le quota de Zoom : cent créations ou modifications de réunion par jour et
//! par hôte, trois demandes d'inscription par jour pour une même personne.
//! Un refus qui dit quand revenir est attendu, un déplacement est regroupé
//! cinq minutes, le back-office peut lever l'attente, et un nouvel essai
//! d'inscription cherche l'inscrit avant de le POSTer.

mod commun;

use commun::http::{serveur, Journal};
use commun::zoom::{consommer, etat_avec_zoom, inscrire, reglages};
use commun::{comptes, decor, perimetre_de, personne, Bac};
use kernel::context::RequestContext;
use kernel::events::{EventConsumer, OutboxEvent};
use kernel::jobs::{ClaimedJob, JobHandler};
use live::consumers::sessions::SessionsConsumer;
use live::jobs::sync_meeting::SyncMeeting;
use live::jobs::sync_participant::SyncParticipant;
use live::service::meeting;
use live::zoom::Zoom;
use serde_json::{json, Value};
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

fn travail(tache: &str, charge: Value) -> ClaimedJob {
    ClaimedJob {
        id: Uuid::now_v7(),
        queue: "live".into(),
        task: tache.into(),
        payload: charge,
        attempts: 1,
        max_attempts: 10,
    }
}

fn zoom_sur(adresse: &str) -> Arc<Zoom> {
    let mut config = reglages();
    config.api_base_url = format!("{adresse}/v2");
    config.oauth_url = format!("{adresse}/oauth/token");
    Arc::new(Zoom::depuis(&config).unwrap())
}

fn jeton() -> (u16, String) {
    (
        200,
        json!({ "access_token": "jeton", "expires_in": 3600 }).to_string(),
    )
}

fn appels(journal: &Journal, debut: &str) -> Vec<String> {
    journal
        .lock()
        .unwrap()
        .iter()
        .filter(|l| l.starts_with(debut))
        .cloned()
        .collect()
}

/// Une inscription et sa réunion, créée en base par le consommateur.
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

/// La réunion existe déjà chez Zoom, sous le numéro 42.
async fn deja_chez_zoom(bac: &Bac, meeting_id: Uuid) {
    sqlx::query(
        "UPDATE live.meetings SET external_id = '42', status = 'scheduled', sync_status = 'synced'
          WHERE id = $1",
    )
    .bind(meeting_id)
    .execute(bac.pool())
    .await
    .unwrap();
}

/// Ce que le relais d'outbox ferait d'un déplacement de la séance.
async fn deplacer(bac: &Bac, session_id: Uuid, heures: i32) {
    sqlx::query(
        "UPDATE programme.sessions
            SET starts_at = starts_at + make_interval(hours => $2),
                ends_at = ends_at + make_interval(hours => $2)
          WHERE id = $1",
    )
    .bind(session_id)
    .bind(heures)
    .execute(bac.pool())
    .await
    .unwrap();
    let evenement = OutboxEvent {
        id: Uuid::now_v7(),
        aggregate_schema: "programme".into(),
        aggregate_type: "session".into(),
        aggregate_id: session_id,
        event_type: "programme.session.rescheduled".into(),
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

/// Les travaux de la réunion : (prévus à une heure, immédiats encore en file).
async fn travaux_de(bac: &Bac, meeting_id: Uuid) -> (i64, i64) {
    sqlx::query_as::<_, (i64, i64)>(
        "SELECT count(*) FILTER (WHERE idempotency_key LIKE $1 || ':prevue:%'),
                count(*) FILTER (WHERE idempotency_key NOT LIKE $1 || ':prevue:%'
                                   AND status = 'queued')
           FROM platform.jobs
          WHERE task = 'live.sync_meeting' AND payload->>'meeting_id' = $1",
    )
    .bind(meeting_id.to_string())
    .fetch_one(bac.pool())
    .await
    .unwrap()
}

/// Le temps passe : l'heure prévue est venue.
async fn heure_venue(bac: &Bac, meeting_id: Uuid) {
    sqlx::query("UPDATE live.meetings SET sync_due_at = now() - interval '1 second' WHERE id = $1")
        .bind(meeting_id)
        .execute(bac.pool())
        .await
        .unwrap();
}

/// La réunion 42 chez Zoom, à son horaire d'avant ; la modifier réussit.
async fn zoom_42() -> (Arc<Zoom>, Journal) {
    let (adresse, journal) = serveur(Arc::new(|methode: &str, chemin: &str| {
        match (methode, chemin.split('?').next().unwrap_or_default()) {
            ("POST", "/oauth/token") => jeton(),
            ("GET", "/v2/meetings/42") => (
                200,
                json!({ "id": 42, "password": "p", "join_url": "https://zoom.us/j/42",
                        "start_time": "2020-01-01T00:00:00Z", "duration": 60,
                        "settings": { "approval_type": 0 } })
                .to_string(),
            ),
            ("PATCH", "/v2/meetings/42") => (204, String::new()),
            _ => (404, "{}".to_owned()),
        }
    }))
    .await;
    (zoom_sur(&adresse), journal)
}

#[tokio::test]
async fn un_quota_de_reunion_attend_lheure_de_zoom_sans_abandonner() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let (meeting_id, _) = reunion_et_inscrit(&bac, d.session_id).await;
    let (adresse, journal) = serveur(Arc::new(|methode: &str, chemin: &str| {
        match (methode, chemin.split('?').next().unwrap_or_default()) {
            ("POST", "/oauth/token") => jeton(),
            _ => (429, "{}".to_owned()),
        }
    }))
    .await;
    let reunion = SyncMeeting::new(bac.db(), zoom_sur(&adresse));
    let charge = json!({ "meeting_id": meeting_id });

    // Dernier essai du travail : un quota avec Retry-After n'abandonne pas.
    let mut dernier = travail("live.sync_meeting", charge.clone());
    dernier.attempts = dernier.max_attempts;
    reunion.run(&dernier).await.unwrap();

    let (statut, attend, erreur) = sqlx::query_as::<_, (String, bool, Option<String>)>(
        "SELECT sync_status::text, sync_due_at > now() + interval '119 seconds', last_sync_error
           FROM live.meetings WHERE id = $1",
    )
    .bind(meeting_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(statut, "failed");
    assert!(attend, "Retry-After est respecté");
    assert!(erreur.is_some_and(|e| e.contains("quota")));

    let (essais, a_lheure) = sqlx::query_as::<_, (i16, bool)>(
        "SELECT j.attempts, j.run_at = m.sync_due_at
           FROM platform.jobs j JOIN live.meetings m ON m.id = $1
          WHERE j.task = 'live.sync_meeting' AND j.status = 'queued'
            AND j.idempotency_key LIKE $1::text || ':prevue:%'",
    )
    .bind(meeting_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(essais, 0, "le nouveau travail repart avec tous ses essais");
    assert!(a_lheure);

    // Avant l'heure, un autre travail ne réserve pas la ligne.
    let avant = journal.lock().unwrap().len();
    reunion
        .run(&travail("live.sync_meeting", charge.clone()))
        .await
        .unwrap();
    assert_eq!(journal.lock().unwrap().len(), avant, "aucun appel à Zoom");

    let tableau =
        live::repo::meetings::tableau(&mut bac.pool().acquire().await.unwrap(), d.session_id)
            .await
            .unwrap()
            .unwrap();
    assert!(tableau.sync_due_at.is_some());
}

#[tokio::test]
async fn deux_deplacements_rapproches_ne_font_quune_modification() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let (meeting_id, _) = reunion_et_inscrit(&bac, d.session_id).await;
    deja_chez_zoom(&bac, meeting_id).await;

    deplacer(&bac, d.session_id, 1).await;
    let premiere = sqlx::query_scalar::<_, OffsetDateTime>(
        "SELECT sync_due_at FROM live.meetings WHERE id = $1",
    )
    .bind(meeting_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert!(premiere > OffsetDateTime::now_utc() + time::Duration::minutes(4));
    deplacer(&bac, d.session_id, 1).await;

    let (statut, due) = sqlx::query_as::<_, (String, OffsetDateTime)>(
        "SELECT sync_status::text, sync_due_at FROM live.meetings WHERE id = $1",
    )
    .bind(meeting_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(statut, "pending");
    assert_eq!(
        due, premiere,
        "le second déplacement ne repousse pas l'envoi"
    );
    assert_eq!(
        travaux_de(&bac, meeting_id).await.0,
        1,
        "un seul travail prévu"
    );

    let (zoom, journal) = zoom_42().await;
    let reunion = SyncMeeting::new(bac.db(), zoom);
    let charge = json!({ "meeting_id": meeting_id });

    // Le travail de la création, en file avant les déplacements : il attend.
    reunion
        .run(&travail("live.sync_meeting", charge.clone()))
        .await
        .unwrap();
    assert!(journal.lock().unwrap().is_empty());

    heure_venue(&bac, meeting_id).await;
    reunion
        .run(&travail("live.sync_meeting", charge.clone()))
        .await
        .unwrap();

    let modifications = appels(&journal, "PATCH /v2/meetings/42");
    assert_eq!(modifications.len(), 1);
    let debut = sqlx::query_scalar::<_, OffsetDateTime>(
        "SELECT starts_at FROM programme.sessions WHERE id = $1",
    )
    .bind(d.session_id)
    .fetch_one(bac.pool())
    .await
    .unwrap()
    .to_offset(UtcOffset::UTC)
    .replace_nanosecond(0)
    .unwrap()
    .format(&Rfc3339)
    .unwrap();
    assert!(
        modifications[0].contains(&debut),
        "le dernier horaire part chez Zoom : {}",
        modifications[0]
    );

    let (statut, due) = sqlx::query_as::<_, (String, Option<OffsetDateTime>)>(
        "SELECT sync_status::text, sync_due_at FROM live.meetings WHERE id = $1",
    )
    .bind(meeting_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(statut, "synced");
    assert!(due.is_none());
}

#[tokio::test]
async fn deplacee_pendant_lappel_la_reunion_repart() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let (meeting_id, _) = reunion_et_inscrit(&bac, d.session_id).await;
    deja_chez_zoom(&bac, meeting_id).await;
    deplacer(&bac, d.session_id, 1).await;
    heure_venue(&bac, meeting_id).await;

    // Le serveur déplace la séance au moment où Zoom reçoit la modification.
    let url = bac.base.url().to_owned();
    let session_id = d.session_id;
    let (adresse, _) = serveur(Arc::new(move |methode: &str, chemin: &str| {
        match (methode, chemin.split('?').next().unwrap_or_default()) {
            ("POST", "/oauth/token") => jeton(),
            ("GET", "/v2/meetings/42") => (
                200,
                json!({ "id": 42, "start_time": "2020-01-01T00:00:00Z", "duration": 60,
                        "settings": { "approval_type": 0 } })
                .to_string(),
            ),
            ("PATCH", "/v2/meetings/42") => {
                let url = url.clone();
                std::thread::spawn(move || {
                    tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(async move {
                            let pool = sqlx::PgPool::connect(&url).await.unwrap();
                            let mut conn = pool.acquire().await.unwrap();
                            let debut = sqlx::query_scalar::<_, OffsetDateTime>(
                                "SELECT scheduled_start_at + interval '1 hour'
                               FROM live.meetings WHERE session_id = $1",
                            )
                            .bind(session_id)
                            .fetch_one(&mut *conn)
                            .await
                            .unwrap();
                            live::repo::meetings::recaler_reunion(&mut conn, session_id, debut, 90)
                                .await
                                .unwrap();
                        })
                })
                .join()
                .unwrap();
                (204, String::new())
            }
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

    let (statut, attend) = sqlx::query_as::<_, (String, bool)>(
        "SELECT sync_status::text, sync_due_at > now() FROM live.meetings WHERE id = $1",
    )
    .bind(meeting_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(statut, "pending", "l'horaire envoyé n'est plus le bon");
    assert!(attend);
    assert_eq!(travaux_de(&bac, meeting_id).await.0, 2);
}

#[tokio::test]
async fn envoyer_maintenant_leve_lattente() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let c = comptes(&bac, &d).await;
    let detache = perimetre_de(&bac, c.detache).await;
    let ctx = bac.ctx(c.detache);
    let etat = etat_avec_zoom(&bac);
    let (meeting_id, _) = reunion_et_inscrit(&bac, d.session_id).await;
    deja_chez_zoom(&bac, meeting_id).await;
    deplacer(&bac, d.session_id, 1).await;

    let lu = meeting::lire_admin(&etat, &detache, d.session_id)
        .await
        .unwrap();
    assert!(lu.sync_due_at.is_some(), "le panneau montre l'heure prévue");

    let (_, immediats) = travaux_de(&bac, meeting_id).await;
    let tableau = meeting::envoyer_maintenant(&etat, &ctx, &detache, d.session_id)
        .await
        .unwrap();
    assert_eq!(tableau.sync_status, "pending");
    assert!(tableau.sync_due_at.is_none());
    assert_eq!(
        travaux_de(&bac, meeting_id).await.1,
        immediats + 1,
        "un travail immédiat en file"
    );

    let (zoom, journal) = zoom_42().await;
    SyncMeeting::new(bac.db(), zoom)
        .run(&travail(
            "live.sync_meeting",
            json!({ "meeting_id": meeting_id }),
        ))
        .await
        .unwrap();
    assert_eq!(appels(&journal, "PATCH /v2/meetings/42").len(), 1);

    // Une séance d'une autre édition : hors périmètre.
    let ailleurs = commun::activite(
        &bac,
        d.autre_event_id,
        None,
        None,
        "Ailleurs",
        "ailleurs",
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc() + time::Duration::hours(1),
    )
    .await;
    assert_eq!(
        meeting::envoyer_maintenant(&etat, &ctx, &detache, ailleurs)
            .await
            .unwrap_err()
            .code,
        kernel::error::ErrorCode::NotFound
    );
}

#[tokio::test]
async fn un_nouvel_essai_dinscription_cherche_avant_de_poster() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let (meeting_id, participant_id) = reunion_et_inscrit(&bac, d.session_id).await;
    sqlx::query(
        "UPDATE live.meetings SET external_id = '1', status = 'scheduled', sync_status = 'synced'
          WHERE id = $1",
    )
    .bind(meeting_id)
    .execute(bac.pool())
    .await
    .unwrap();
    // Un premier essai a échoué après avoir peut-être inscrit chez Zoom.
    sqlx::query(
        "UPDATE live.meeting_participants
            SET sync_status = 'failed', sync_attempts = 1, last_error = 'délai dépassé'
          WHERE id = $1",
    )
    .bind(participant_id)
    .execute(bac.pool())
    .await
    .unwrap();

    let (adresse, journal) = serveur(Arc::new(|methode: &str, chemin: &str| {
        let (route, requete) = chemin.split_once('?').unwrap_or((chemin, ""));
        match (methode, route) {
            ("POST", "/oauth/token") => jeton(),
            ("GET", "/v2/meetings/1/registrants") if requete.contains("status=approved") => (
                200,
                json!({ "registrants": [{
                    "id": "reg-awa", "email": "awa.diallo@exemple.org",
                    "join_url": "https://zoom.us/w/1?tk=tk-awa"
                }], "next_page_token": "" })
                .to_string(),
            ),
            ("POST", "/v2/meetings/1/registrants") => (
                201,
                json!({ "registrant_id": "reg-double", "join_url": "https://zoom.us/w/1?tk=x" })
                    .to_string(),
            ),
            _ => (404, "{}".to_owned()),
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

    let (statut, inscrit) = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT sync_status::text, external_registrant_id
           FROM live.meeting_participants WHERE id = $1",
    )
    .bind(participant_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(statut, "synced");
    assert_eq!(inscrit.as_deref(), Some("reg-awa"));
    assert!(
        appels(&journal, "POST /v2/meetings/1/registrants").is_empty(),
        "retrouvé : aucune demande d'inscription de plus"
    );
}
