//! Le direct s'allume (la base annonce le démarrage d'une réunion, une
//! répétition attend début − 15 min), les présents dans la salle, les échecs
//! d'inscription du poste de direct, les messages d'une activité.

mod commun;

use commun::{comptes, decor, evenements_emis, perimetre_de, poser, Bac};
use kernel::context::RequestContext;
use live::repo::webhooks;
use serde_json::json;
use time::macros::datetime;
use uuid::Uuid;

async fn reunion(bac: &Bac, session_id: Uuid, numero: &str) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO live.meetings (session_id, event_id, external_id, status, sync_status)
         SELECT id, event_id, $2, 'scheduled', 'synced' FROM programme.sessions WHERE id = $1
         RETURNING id",
        session_id,
        numero
    )
    .fetch_one(bac.pool())
    .await
    .expect("réunion")
}

/// Le créneau de la séance, en minutes depuis maintenant.
async fn caler(bac: &Bac, session_id: Uuid, debut_dans: i32) {
    sqlx::query!(
        "UPDATE programme.sessions
            SET starts_at = now() + make_interval(mins => $2),
                ends_at   = now() + make_interval(mins => $2 + 90)
          WHERE id = $1",
        session_id,
        debut_dans
    )
    .execute(bac.pool())
    .await
    .expect("créneau");
}

async fn etat(bac: &Bac, meeting_id: Uuid, statut: &str, occurrence: &str) {
    sqlx::query!(
        "UPDATE live.meetings
            SET status = $2::text::live.meeting_status, external_uuid = $3,
                actual_start_at = COALESCE(actual_start_at, now())
          WHERE id = $1",
        meeting_id,
        statut,
        occurrence
    )
    .execute(bac.pool())
    .await
    .expect("état de la réunion");
}

async fn annonces_prevues(bac: &Bac, meeting_id: Uuid) -> Vec<time::OffsetDateTime> {
    sqlx::query_scalar!(
        "SELECT run_at FROM platform.jobs
          WHERE task = 'live.announce_meeting_start' AND (payload->>'meeting_id')::uuid = $1
          ORDER BY run_at",
        meeting_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("travaux")
}

#[tokio::test]
async fn un_demarrage_a_lheure_annonce_le_direct_et_la_fin_aussi() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    caler(&bac, d.session_id, 10).await;
    let m = reunion(&bac, d.session_id, "111").await;

    etat(&bac, m, "started", "occ-1").await;
    etat(&bac, m, "ended", "occ-1").await;
    assert_eq!(
        evenements_emis(&bac, m).await,
        ["live.meeting.started", "live.meeting.ended"]
    );
    let charge = sqlx::query_scalar!(
        "SELECT payload FROM platform.outbox_events WHERE aggregate_id = $1 ORDER BY occurred_at, id LIMIT 1",
        m
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(charge["session_id"], json!(d.session_id));
    assert_eq!(charge["event_id"], json!(d.event_id));
    assert!(annonces_prevues(&bac, m).await.is_empty());
}

#[tokio::test]
async fn une_repetition_attend_debut_moins_quinze_minutes() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    caler(&bac, d.session_id, 180).await;
    let m = reunion(&bac, d.session_id, "222").await;

    etat(&bac, m, "started", "occ-1").await;
    assert!(
        evenements_emis(&bac, m).await.is_empty(),
        "une répétition n'allume rien"
    );
    let prevues = annonces_prevues(&bac, m).await;
    assert_eq!(prevues.len(), 1);
    let debut = sqlx::query_scalar!(
        "SELECT starts_at FROM programme.sessions WHERE id = $1",
        d.session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(prevues[0], debut - time::Duration::minutes(15));

    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    // Encore trop tôt : l'annonce se reprogramme, sans doublon.
    assert!(!live::jobs::annonce::annoncer(&mut tx, m, Some("occ-1"))
        .await
        .unwrap());
    tx.commit().await.unwrap();
    assert_eq!(annonces_prevues(&bac, m).await.len(), 1);

    // La séance avance : l'heure est venue.
    caler(&bac, d.session_id, 5).await;
    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    assert!(
        !live::jobs::annonce::annoncer(&mut tx, m, Some("occ-0"))
            .await
            .unwrap(),
        "une autre occurrence ne dit rien"
    );
    assert!(live::jobs::annonce::annoncer(&mut tx, m, Some("occ-1"))
        .await
        .unwrap());
    tx.commit().await.unwrap();
    assert_eq!(evenements_emis(&bac, m).await, ["live.meeting.started"]);

    // Arrêtée avant l'heure : rien.
    etat(&bac, m, "ended", "occ-1").await;
    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    assert!(!live::jobs::annonce::annoncer(&mut tx, m, Some("occ-1"))
        .await
        .unwrap());
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn une_relance_annonce_de_nouveau_et_un_demarrage_en_retard_ne_rallume_rien() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    caler(&bac, d.session_id, -30).await;
    let m = reunion(&bac, d.session_id, "333").await;
    let instant = datetime!(2026-11-10 14:00 UTC);

    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    webhooks::demarrer(&mut tx, m, Some("occ-1"), instant)
        .await
        .unwrap();
    webhooks::terminer(&mut tx, m, instant).await.unwrap();
    // `meeting.started` de la même occurrence, livré après sa fin.
    webhooks::demarrer(&mut tx, m, Some("occ-1"), instant)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let statut = sqlx::query_scalar!(
        r#"SELECT status::text AS "s!" FROM live.meetings WHERE id = $1"#,
        m
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(statut, "ended");

    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    webhooks::demarrer(&mut tx, m, Some("occ-2"), instant)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        evenements_emis(&bac, m).await,
        [
            "live.meeting.started",
            "live.meeting.ended",
            "live.meeting.started"
        ]
    );
}

#[test]
fn le_motif_suit_le_texte_des_erreurs_zoom() {
    use live::domain::desk::motif_d_echec;
    use live::zoom::ZoomError;
    let cas = [
        (ZoomError::Passagere("délai".into()), "unreachable"),
        (
            ZoomError::Quota {
                reessayer_dans: Some(30),
            },
            "quota",
        ),
        (
            ZoomError::Quota {
                reessayer_dans: None,
            },
            "quota",
        ),
        (
            ZoomError::Refusee {
                statut: 400,
                code: Some(300),
                message: "Invalid email".into(),
            },
            "refused",
        ),
    ];
    for (erreur, motif) in cas {
        assert_eq!(motif_d_echec(Some(&erreur.to_string())), motif, "{erreur}");
    }
    assert_eq!(
        motif_d_echec(Some(live::jobs::sync_participant::PAS_ENCORE_CREEE)),
        "meeting_not_created"
    );
    assert_eq!(motif_d_echec(None), "other");
}

async fn compte(bac: &Bac, m: Uuid) -> (i64, i64) {
    let c = sqlx::query!(
        r#"SELECT present_count AS "p!", joined_count AS "j!"
             FROM live.meeting_room_counts WHERE meeting_id = $1"#,
        m
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    (c.p, c.j)
}

#[tokio::test]
async fn les_presents_se_comptent_par_occurrence_et_en_desordre() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let m = reunion(&bac, d.session_id, "444").await;
    etat(&bac, m, "started", "occ-1").await;

    assert_eq!(
        live::jobs::webhook::cle_de_connexion(
            &json!({ "user_id": "16778240", "email": "a@b.org" })
        ),
        Some("user_id:16778240".into())
    );
    assert_eq!(
        live::jobs::webhook::cle_de_connexion(&json!({ "email": "" })),
        None
    );

    let t = |h: u8, mi: u8| {
        datetime!(2026-11-10 00:00 UTC)
            + time::Duration::hours(h.into())
            + time::Duration::minutes(mi.into())
    };
    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    webhooks::noter_un_mouvement(&mut tx, m, Some("occ-1"), "hote", true, t(14, 0))
        .await
        .unwrap();
    webhooks::noter_un_mouvement(&mut tx, m, Some("occ-1"), "awa", true, t(14, 2))
        .await
        .unwrap();
    // Sans `uuid` : l'occurrence de la réunion.
    webhooks::noter_un_mouvement(&mut tx, m, None, "karim", true, t(14, 3))
        .await
        .unwrap();
    // Karim sort, revient ; la sortie arrive après le retour.
    webhooks::noter_un_mouvement(&mut tx, m, None, "karim", true, t(14, 20))
        .await
        .unwrap();
    webhooks::noter_un_mouvement(&mut tx, m, None, "karim", false, t(14, 10))
        .await
        .unwrap();
    // Awa sort ; la notification est rejouée.
    webhooks::noter_un_mouvement(&mut tx, m, Some("occ-1"), "awa", false, t(14, 30))
        .await
        .unwrap();
    webhooks::noter_un_mouvement(&mut tx, m, Some("occ-1"), "awa", false, t(14, 30))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(compte(&bac, m).await, (2, 3));

    // Relancée : la salle repart de zéro.
    etat(&bac, m, "started", "occ-2").await;
    assert_eq!(compte(&bac, m).await, (0, 0));
}

#[tokio::test]
async fn le_poste_montre_la_reunion_et_ses_echecs_a_qui_gere_les_reunions() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let c = comptes(&bac, &d).await;
    let m = reunion(&bac, d.session_id, "555").await;
    etat(&bac, m, "started", "occ-1").await;

    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    webhooks::noter_un_mouvement(
        &mut tx,
        m,
        Some("occ-1"),
        "hote",
        true,
        datetime!(2026-11-10 14:00 UTC),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    for (nom, statut, erreur) in [
        (
            "Awa",
            "abandoned",
            "Zoom refuse (400, code 300) : Invalid email",
        ),
        (
            "Karim",
            "failed",
            "quota Zoom atteint, nouvel essai dans 30 s",
        ),
        ("Lina", "failed", "réunion pas encore créée"),
        ("Inès", "synced", ""),
    ] {
        sqlx::query!(
            "INSERT INTO live.meeting_participants
                 (meeting_id, display_name, email, sync_status, last_error, external_registrant_id,
                  last_attempt_at)
             VALUES ($1, $2, lower($2) || '@exemple.org', $3::text::live.sync_status,
                     NULLIF($4, ''), CASE WHEN $3 = 'synced' THEN 'reg-' || $2 END, now())",
            m,
            nom,
            statut,
            erreur
        )
        .execute(bac.pool())
        .await
        .unwrap();
    }

    let detache = perimetre_de(&bac, c.detache).await;
    let ecran = live::service::list::ecran(&bac.state, &detache, d.event_id, "fr")
        .await
        .unwrap();
    assert!(ecran.desk.meetings_visible);
    assert!(
        !ecran.desk.presence_notifications,
        "aucune entrée signée reçue de Zoom"
    );
    let ligne = ecran
        .desk
        .sessions
        .iter()
        .find(|s| s.session_id == d.session_id)
        .unwrap();
    let r = ligne.meeting.as_ref().expect("réunion visible");
    assert_eq!(
        (r.status.as_str(), r.present_count, r.registered_count),
        ("started", 1, 1)
    );
    assert_eq!(r.failure_count, 3);
    let mut motifs: Vec<_> = r.failures.iter().map(|f| f.reason).collect();
    motifs.sort_unstable();
    assert_eq!(motifs, ["meeting_not_created", "quota", "refused"]);
    let autre = ecran
        .desk
        .sessions
        .iter()
        .find(|s| s.session_id == d.autre_session_id)
        .unwrap();
    assert!(autre.meeting.is_none());

    commun::retirer_la_permission(&bac, "programmer", "live.meeting.manage").await;
    let lectrice = perimetre_de(&bac, c.lectrice).await;
    let ecran = live::service::list::ecran(&bac.state, &lectrice, d.event_id, "fr")
        .await
        .unwrap();
    assert!(!ecran.desk.meetings_visible);
    assert!(ecran.desk.sessions.iter().all(|s| s.meeting.is_none()));
}

#[tokio::test]
async fn une_activite_voit_ses_messages_et_ceux_de_ledition_pas_ceux_des_autres() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let c = comptes(&bac, &d).await;
    sqlx::query!(
        "UPDATE programme.sessions SET published_at = now() WHERE event_id = $1",
        d.event_id
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let edition = poser(&bac, c.globale, "event", Some(d.event_id), "active").await;
    let la_mienne = poser(&bac, c.globale, "session", Some(d.session_id), "active").await;
    let lautre = poser(
        &bac,
        c.globale,
        "session",
        Some(d.autre_session_id),
        "active",
    )
    .await;
    // Une autre édition : n'apparaît nulle part ici.
    poser(&bac, c.globale, "event", Some(d.autre_event_id), "active").await;

    let ids = |lignes: Vec<live::repo::active::ActiveIncident>| {
        let mut v: Vec<Uuid> = lignes.into_iter().map(|l| l.incident_id).collect();
        v.sort();
        v
    };
    let mut attendus = vec![edition, la_mienne];
    attendus.sort();
    assert_eq!(
        ids(live::repo::active::pour_la_seance(bac.pool(), d.session_id)
            .await
            .unwrap()),
        attendus
    );
    assert!(
        live::repo::active::pour_la_seance(bac.pool(), Uuid::now_v7())
            .await
            .unwrap()
            .is_empty()
    );

    let fil = sqlx::query_scalar!(
        "INSERT INTO event.programme_tracks (event_id, code, slug, title, kind, published_at)
         VALUES ($1, 'finance', 'journee-finance', '{\"fr\":\"Journée finance\"}', 'special_day', now())
         RETURNING id",
        d.event_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO programme.session_tracks (session_id, track_id) VALUES ($1, $2)",
        d.autre_session_id,
        fil
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let mut attendus = vec![edition, lautre];
    attendus.sort();
    assert_eq!(
        ids(live::repo::active::pour_le_fil(bac.pool(), d.event_id, fil)
            .await
            .unwrap()),
        attendus
    );
}
