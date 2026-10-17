//! L'entrée dans la salle : Zoom configuré, inscription, salle ouverte, place
//! prête, dans cet ordre. Et la lecture publique de la réunion.

mod commun;

use commun::zoom::{consommer, etat_avec_zoom, inscrire, publier};
use commun::{decor, personne, Bac};
use kernel::error::ErrorCode;
use live::service::meeting;
use time::Duration;
use uuid::Uuid;

async fn horaires(bac: &Bac, session_id: Uuid) -> (time::OffsetDateTime, time::OffsetDateTime) {
    let s = sqlx::query!(
        "SELECT starts_at, ends_at FROM programme.sessions WHERE id = $1",
        session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    (s.starts_at, s.ends_at)
}

/// Ce que les deux travaux auraient écrit après le passage chez Zoom.
async fn simuler_zoom(bac: &Bac, session_id: Uuid, code: Vec<u8>) {
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
    sqlx::query!(
        "UPDATE live.meeting_participants p
            SET external_registrant_id = 'reg-' || p.id, sync_status = 'synced',
                personal_join_url = 'https://us06web.zoom.us/w/85746065432?tk=jeton-' || p.id,
                synced_at = now()
          FROM live.meetings m
         WHERE m.id = p.meeting_id AND m.session_id = $1",
        session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn les_regles_dacces_a_la_salle() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    publier(&bac, d.session_id).await;
    let (debut, fin) = horaires(&bac, d.session_id).await;
    let pendant = debut + Duration::minutes(5);

    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let karim = personne(&bac, "karim@exemple.org", "Karim", "Ilboudo").await;

    // Zoom non configuré.
    let erreur = meeting::acces(&bac.state, awa, d.session_id, pendant)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveZoomUnavailable);

    let etat = etat_avec_zoom(&bac);

    // Aucune inscription.
    let erreur = meeting::acces(&etat, karim, d.session_id, pendant)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveMeetingRegistrationRequired);

    let r = inscrire(&bac, d.session_id, awa).await;
    consommer(&bac, r).await;

    // Hors de la fenêtre : plus de trente minutes avant, ou après la fin.
    for instant in [debut - Duration::minutes(31), fin + Duration::minutes(31)] {
        let erreur = meeting::acces(&etat, awa, d.session_id, instant)
            .await
            .unwrap_err();
        assert_eq!(erreur.code, ErrorCode::LiveMeetingNotOpen);
    }

    // Réunion pas encore créée chez Zoom.
    let erreur = meeting::acces(&etat, awa, d.session_id, debut - Duration::minutes(29))
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveMeetingNotReady);
    assert_eq!(
        meeting::publique(&etat, d.session_id)
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound,
        "une réunion encore en brouillon ne se montre pas"
    );

    let zoom = etat.zoom().unwrap();
    simuler_zoom(
        &bac,
        d.session_id,
        zoom.coffre.chiffrer("Pavillon26").unwrap(),
    )
    .await;

    let acces = meeting::acces(&etat, awa, d.session_id, pendant)
        .await
        .unwrap();
    assert_eq!(acces.meeting_number, "85746065432");
    assert_eq!(acces.password, "Pavillon26");
    assert_eq!(acces.user_name, "Awa Diallo");
    assert_eq!(acces.user_email, "awa.diallo@exemple.org");
    assert_eq!(acces.sdk_key, "cle-sdk");
    assert!(acces
        .registrant_token
        .as_deref()
        .unwrap()
        .starts_with("jeton-"));
    assert_eq!(acces.signature.split('.').count(), 3);
    assert!(acces.expires_at > pendant + Duration::minutes(30));

    let publique = meeting::publique(&etat, d.session_id).await.unwrap();
    assert_eq!(publique.status, "scheduled");
    assert_eq!(publique.opens_at, debut - Duration::minutes(30));
    assert_eq!(publique.closes_at, fin + Duration::minutes(30));

    // Une resynchronisation pendant l'activité ne ferme la porte à personne.
    sqlx::query!(
        "UPDATE live.meetings SET sync_status = 'in_progress' WHERE session_id = $1",
        d.session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();
    sqlx::query!("UPDATE live.meeting_participants SET sync_status = 'pending'")
        .execute(bac.pool())
        .await
        .unwrap();
    assert!(meeting::acces(&etat, awa, d.session_id, pendant)
        .await
        .is_ok());

    // Séance annulée : la salle n'ouvre pas.
    sqlx::query!(
        "UPDATE programme.sessions SET status = 'cancelled', cancelled_reason = jsonb_build_object('fr', 'Test') WHERE id = $1",
        d.session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let erreur = meeting::acces(&etat, awa, d.session_id, pendant)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveMeetingCancelled);
}

#[tokio::test]
async fn la_liste_dattente_entre_aussi() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    publier(&bac, d.session_id).await;
    sqlx::query!(
        "UPDATE programme.sessions SET capacity = 1, waitlist_enabled = true WHERE id = $1",
        d.session_id
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let (debut, _) = horaires(&bac, d.session_id).await;

    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let jean = personne(&bac, "jean.kabore@exemple.org", "Jean", "Kaboré").await;
    for p in [awa, jean] {
        let r = inscrire(&bac, d.session_id, p).await;
        consommer(&bac, r).await;
    }
    let etat = etat_avec_zoom(&bac);
    let code = etat.zoom().unwrap().coffre.chiffrer("x").unwrap();
    simuler_zoom(&bac, d.session_id, code).await;

    let acces = meeting::acces(&etat, jean, d.session_id, debut)
        .await
        .unwrap();
    assert_eq!(acces.user_email, "jean.kabore@exemple.org");
}
