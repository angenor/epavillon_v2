//! Les intervenants en ligne (08/11) : la place `panelist` d'un intervenant qui
//! a une adresse, le courriel de son lien posé dès que Zoom l'a donné, une
//! seule fois par demande, et jamais une seconde fois d'office.

mod commun;

use commun::{decor, Bac};
use kernel::context::RequestContext;
use live::service::intervenants;
use uuid::Uuid;

async fn courriels(bac: &Bac) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM platform.jobs WHERE task = 'engagement.send_speaker_zoom_link'",
    )
    .fetch_one(bac.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn un_intervenant_recoit_sa_place_puis_son_lien() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    let personne: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, civility, status)
         VALUES ('fatou.ndiaye@exemple.org', 'Fatou', 'Ndiaye', 'mme', 'active') RETURNING id",
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO programme.session_speakers (session_id, person_id, role, source, is_listed)
         VALUES ($1, $2, 'speaker', 'regie', true)",
    )
    .bind(d.session_id)
    .bind(personne)
    .execute(bac.pool())
    .await
    .unwrap();
    // Un intervenant sans adresse (D16) n'a pas de place.
    sqlx::query(
        "INSERT INTO programme.session_speakers (session_id, role, first_name, last_name)
         VALUES ($1, 'speaker', 'Paul', 'Kasongo')",
    )
    .bind(d.session_id)
    .execute(bac.pool())
    .await
    .unwrap();

    let ctx = RequestContext::background("test");
    let mut tx = bac.db().write(&ctx).await.unwrap();
    let demandes = intervenants::donner_les_places(&mut tx, d.session_id, None, false)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(demandes, 1, "seul l'intervenant qui a une adresse");

    let (role, demande): (String, Option<time::OffsetDateTime>) = sqlx::query_as(
        "SELECT mp.role::text, mp.link_requested_at FROM live.meeting_participants mp
           JOIN live.meetings m ON m.id = mp.meeting_id
          WHERE m.session_id = $1 AND mp.email = 'fatou.ndiaye@exemple.org'",
    )
    .bind(d.session_id)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(role, "panelist");
    assert!(demande.is_some());
    assert_eq!(
        courriels(&bac).await,
        0,
        "pas de lien chez Zoom : rien ne part encore"
    );

    sqlx::query(
        "UPDATE live.meeting_participants
            SET sync_status = 'synced', external_registrant_id = 'r-1',
                personal_join_url = 'https://us06web.zoom.us/w/1?tk=x'
          WHERE email = 'fatou.ndiaye@exemple.org'",
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let mut tx = bac.db().write(&ctx).await.unwrap();
    let doffice = intervenants::donner_les_places(&mut tx, d.session_id, None, true)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(doffice, 0, "lien déjà demandé : la veille n'en renvoie pas");

    let mut tx = bac.db().write(&ctx).await.unwrap();
    intervenants::donner_les_places(&mut tx, d.session_id, None, false)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        courriels(&bac).await,
        1,
        "le renvoi de la régie part tout de suite"
    );

    let charge: serde_json::Value = sqlx::query_scalar(
        "SELECT payload FROM platform.jobs WHERE task = 'engagement.send_speaker_zoom_link'",
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(charge["session_id"], serde_json::json!(d.session_id));
}
