//! Les notifications de Zoom : validation de l'adresse, journal, mise en file,
//! refus d'une signature fausse, puis application d'une entrée en salle.

mod commun;

use commun::zoom::{consommer, etat_avec_zoom, inscrire};
use commun::{decor, personne, Bac};
use kernel::context::RequestContext;
use kernel::error::ErrorCode;
use live::service::webhook::{recevoir, EnTetes};
use live::zoom::webhook::hmac_hex;
use serde_json::json;
use time::OffsetDateTime;

fn signer(corps: &str, horodatage: &str) -> String {
    format!(
        "v0={}",
        hmac_hex("jeton-zoom", format!("v0:{horodatage}:{corps}").as_bytes())
    )
}

#[tokio::test]
async fn une_entree_en_salle_note_la_presence() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let etat = etat_avec_zoom(&bac);
    let maintenant = OffsetDateTime::now_utc();
    let ts = maintenant.unix_timestamp().to_string();

    // Validation de l'adresse, signée.
    let validation =
        json!({ "event": "endpoint.url_validation", "payload": { "plainToken": "abc" } })
            .to_string();
    let reponse = recevoir(
        &etat,
        &EnTetes {
            signature: Some(&signer(&validation, &ts)),
            horodatage: Some(&ts),
        },
        validation.as_bytes(),
        maintenant,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(reponse["encryptedToken"], hmac_hex("jeton-zoom", b"abc"));

    // Non signée : refusée, sans quoi la route signerait n'importe quoi.
    let erreur = recevoir(
        &etat,
        &EnTetes {
            signature: None,
            horodatage: Some(&ts),
        },
        validation.as_bytes(),
        maintenant,
    )
    .await
    .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::Unauthenticated);

    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let r = inscrire(&bac, d.session_id, awa).await;
    consommer(&bac, r).await;
    sqlx::query!(
        "UPDATE live.meetings SET external_id = '85746065432', status = 'scheduled',
                sync_status = 'synced' WHERE session_id = $1",
        d.session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE live.meeting_participants SET external_registrant_id = 'reg-1',
                sync_status = 'synced' WHERE registration_id = $1",
        r
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let entree = json!({
        "event": "meeting.participant_joined",
        "event_ts": maintenant.unix_timestamp() * 1000,
        "payload": { "object": {
            "id": "85746065432",
            "participant": { "registrant_id": "reg-1", "email": "awa.diallo@exemple.org",
                             "join_time": "2026-11-10T14:02:00Z" }
        } }
    })
    .to_string();
    let en_tetes = EnTetes {
        signature: Some(&signer(&entree, &ts)),
        horodatage: Some(&ts),
    };
    assert!(recevoir(&etat, &en_tetes, entree.as_bytes(), maintenant)
        .await
        .unwrap()
        .is_none());
    // Zoom rejoue : dédoublonnée par empreinte.
    recevoir(&etat, &en_tetes, entree.as_bytes(), maintenant)
        .await
        .unwrap();

    // Signature fausse : journalisée, jamais appliquée.
    let erreur = recevoir(
        &etat,
        &EnTetes {
            signature: Some("v0=faux"),
            horodatage: Some(&ts),
        },
        br#"{"event":"meeting.ended","payload":{"object":{"id":"85746065432"}}}"#,
        maintenant,
    )
    .await
    .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::Unauthenticated);

    let journal = sqlx::query!(
        r#"SELECT id, event_type, signature_verified, status::text AS "s!", payload, payload_hash
             FROM live.provider_webhook_events ORDER BY received_at"#
    )
    .fetch_all(bac.pool())
    .await
    .unwrap();
    assert_eq!(journal.len(), 2);
    assert!(journal[0].signature_verified && journal[0].s == "received");
    assert!(!journal[1].signature_verified && journal[1].s == "ignored");
    assert_eq!(
        journal[1].payload,
        json!({}),
        "rien du corps non signé n'est gardé"
    );
    assert!(journal[1].payload_hash.is_none());
    let travaux = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs WHERE task = 'live.process_webhook_event'"#
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(travaux, 1);

    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .unwrap();
    live::jobs::webhook::traiter(&mut tx, journal[0].id)
        .await
        .unwrap();
    live::jobs::webhook::traiter(&mut tx, journal[1].id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let presence = sqlx::query!(
        r#"SELECT p.joined_at AS participant, r.joined_at AS inscription
             FROM live.meeting_participants p
             JOIN programme.registrations r ON r.id = p.registration_id
            WHERE p.registration_id = $1"#,
        r
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert!(presence.participant.is_some());
    assert!(presence.inscription.is_some());

    let statut = sqlx::query_scalar!(
        r#"SELECT status::text AS "s!" FROM live.meetings WHERE session_id = $1"#,
        d.session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(
        statut, "scheduled",
        "la notification non vérifiée n'a rien terminé"
    );
}

#[tokio::test]
async fn une_reunion_relancee_par_lhote_rouvre_la_salle() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let etat = etat_avec_zoom(&bac);
    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let r = inscrire(&bac, d.session_id, awa).await;
    consommer(&bac, r).await;
    sqlx::query!(
        "UPDATE live.meetings SET external_id = '85746065432', status = 'scheduled',
                sync_status = 'synced' WHERE session_id = $1",
        d.session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let maintenant = OffsetDateTime::now_utc();
    let ts = maintenant.unix_timestamp().to_string();
    for (evenement, instant) in [
        ("meeting.started", "start_time\":\"2026-11-10T14:00:00Z"),
        ("meeting.ended", "end_time\":\"2026-11-10T15:30:00Z"),
        ("meeting.started", "start_time\":\"2026-11-10T15:40:00Z"),
    ] {
        let corps = format!(
            r#"{{"event":"{evenement}","event_ts":{},"payload":{{"object":{{"id":"85746065432","{instant}"}}}}}}"#,
            maintenant.unix_timestamp_nanos() / 1000
        );
        recevoir(
            &etat,
            &EnTetes {
                signature: Some(&signer(&corps, &ts)),
                horodatage: Some(&ts),
            },
            corps.as_bytes(),
            maintenant,
        )
        .await
        .unwrap();
        let id = sqlx::query_scalar!(
            "SELECT id FROM live.provider_webhook_events ORDER BY received_at DESC LIMIT 1"
        )
        .fetch_one(bac.pool())
        .await
        .unwrap();
        let mut tx = bac
            .db()
            .write(&RequestContext::background("test"))
            .await
            .unwrap();
        live::jobs::webhook::traiter(&mut tx, id).await.unwrap();
        tx.commit().await.unwrap();
    }

    let m = sqlx::query!(
        r#"SELECT status::text AS "s!", actual_start_at, actual_end_at
             FROM live.meetings WHERE session_id = $1"#,
        d.session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(m.s, "started", "terminée puis relancée : la salle rouvre");
    assert!(m.actual_end_at.is_none());
    assert_eq!(
        m.actual_start_at,
        Some(time::macros::datetime!(2026-11-10 14:00 UTC)),
        "le premier démarrage reste"
    );
}
