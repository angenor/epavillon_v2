//! Le consommateur des inscriptions : une réunion par séance diffusée, un
//! inscrit Zoom par inscription qui a une place (liste d'attente comprise),
//! l'annulation, la réinscription.

mod commun;

use commun::zoom::{annuler, consommer, inscrire, statut, travaux};
use commun::{decor, personne, Bac};

#[tokio::test]
async fn inscrits_et_liste_dattente_deviennent_des_inscrits_zoom() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    // Jauge d'une place : la seconde inscription part en liste d'attente.
    sqlx::query!(
        "UPDATE programme.sessions SET capacity = 1, waitlist_enabled = true WHERE id = $1",
        d.session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let jean = personne(&bac, "jean.kabore@exemple.org", "Jean", "Kaboré").await;
    let r1 = inscrire(&bac, d.session_id, awa).await;
    let r2 = inscrire(&bac, d.session_id, jean).await;
    assert_eq!(statut(&bac, r2).await, "waitlisted");

    consommer(&bac, r1).await;
    consommer(&bac, r2).await;
    // Rejouer ne double rien.
    consommer(&bac, r1).await;

    let reunion = sqlx::query!(
        r#"SELECT id, topic, provider::text AS "provider!", status::text AS "status!",
                  sync_status::text AS "sync_status!", duration_minutes, timezone::text AS "tz!"
             FROM live.meetings WHERE session_id = $1"#,
        d.session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(reunion.provider, "zoom");
    assert_eq!(reunion.status, "draft");
    assert_eq!(reunion.sync_status, "pending");
    assert_eq!(reunion.topic.as_deref(), Some("Atelier de négociation"));
    assert_eq!(reunion.duration_minutes, Some(90));
    assert_eq!(reunion.tz, commun::FUSEAU);
    assert_eq!(
        travaux(&bac, "live.sync_meeting", &reunion.id.to_string()).await,
        1,
        "le déclencheur dépose la création chez Zoom"
    );

    let inscrits = sqlx::query!(
        r#"SELECT id, email::text AS "email!", sync_status::text AS "s!", request_payload
             FROM live.meeting_participants WHERE meeting_id = $1 ORDER BY email"#,
        reunion.id
    )
    .fetch_all(bac.pool())
    .await
    .unwrap();
    assert_eq!(inscrits.len(), 2);
    assert_eq!(inscrits[0].email, "awa.diallo@exemple.org");
    assert_eq!(inscrits[1].email, "jean.kabore@exemple.org");
    assert!(inscrits.iter().all(|i| i.s == "pending"));
    assert_eq!(inscrits[1].request_payload["first_name"], "Jean");
    for i in &inscrits {
        assert_eq!(
            travaux(&bac, "live.sync_meeting_participant", &i.id.to_string()).await,
            1
        );
    }
}

#[tokio::test]
async fn annuler_puis_se_reinscrire_reprend_le_meme_inscrit() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;

    let r1 = inscrire(&bac, d.session_id, awa).await;
    consommer(&bac, r1).await;
    let id = sqlx::query_scalar!(
        "SELECT id FROM live.meeting_participants WHERE registration_id = $1",
        r1
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();

    annuler(&bac, r1).await;
    consommer(&bac, r1).await;
    let annule = sqlx::query_scalar!(
        "SELECT cancelled_at IS NOT NULL FROM live.meeting_participants WHERE id = $1",
        id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(annule, Some(true));
    assert_eq!(
        travaux(&bac, "live.cancel_meeting_participant", &id.to_string()).await,
        1
    );

    let r2 = inscrire(&bac, d.session_id, awa).await;
    consommer(&bac, r2).await;
    let ligne = sqlx::query!(
        r#"SELECT id, registration_id, cancelled_at, sync_status::text AS "s!"
             FROM live.meeting_participants WHERE person_id = $1"#,
        awa
    )
    .fetch_all(bac.pool())
    .await
    .unwrap();
    assert_eq!(ligne.len(), 1, "l'adresse est unique par réunion");
    assert_eq!(ligne[0].id, id);
    assert_eq!(ligne[0].registration_id, Some(r2));
    assert!(ligne[0].cancelled_at.is_none());
    assert_eq!(ligne[0].s, "pending");
    assert_eq!(
        travaux(&bac, "live.sync_meeting_participant", &id.to_string()).await,
        2,
        "la reprise dépose un nouveau travail, le déclencheur ne réagit qu'à l'insertion"
    );
}

#[tokio::test]
async fn une_seance_non_diffusee_na_pas_de_reunion() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    sqlx::query!(
        "UPDATE programme.sessions SET is_streamed = false WHERE id = $1",
        d.session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let r = inscrire(&bac, d.session_id, awa).await;
    consommer(&bac, r).await;

    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM live.meetings WHERE session_id = $1"#,
        d.session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(n, 0);
}
