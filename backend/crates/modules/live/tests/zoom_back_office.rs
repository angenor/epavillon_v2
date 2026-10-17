//! Le back-office d'une réunion : périmètre, permission, création rejouable,
//! rattachement d'une réunion Zoom existante.

mod commun;

use commun::zoom::{etat_avec_zoom, inscrire};
use commun::{comptes, decor, perimetre_de, personne, Bac};
use kernel::error::ErrorCode;
use live::domain::meeting::MeetingSyncInput;
use live::service::meeting;

#[tokio::test]
async fn creer_puis_rattacher_la_reunion_dune_seance() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let c = comptes(&bac, &d).await;
    let detache = perimetre_de(&bac, c.detache).await;
    let ctx = bac.ctx(c.detache);

    // Les inscriptions existaient avant la réunion : la création les reprend.
    for (email, prenom) in [("a@exemple.org", "Awa"), ("b@exemple.org", "Binta")] {
        let p = personne(&bac, email, prenom, "Test").await;
        inscrire(&bac, d.session_id, p).await;
    }

    let vide = MeetingSyncInput::default();
    let erreur = meeting::reconstruire(&bac.state, &ctx, &detache, d.session_id, &vide)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveZoomUnavailable);

    let etat = etat_avec_zoom(&bac);
    // Séance d'une autre édition : hors périmètre, donc introuvable.
    let ailleurs = commun::activite(
        &bac,
        d.autre_event_id,
        None,
        None,
        "Ailleurs",
        "ailleurs",
        time::OffsetDateTime::now_utc(),
        time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    )
    .await;
    let erreur = meeting::reconstruire(&etat, &ctx, &detache, ailleurs, &vide)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::NotFound);

    let tableau = meeting::reconstruire(&etat, &ctx, &detache, d.session_id, &vide)
        .await
        .unwrap();
    assert_eq!(tableau.provider, "zoom");
    assert_eq!(tableau.sync_status, "pending");
    assert_eq!(tableau.participants_total, 2);
    assert_eq!(tableau.participants_synced, 0);

    // Rejouer ne double rien.
    let encore = meeting::reconstruire(&etat, &ctx, &detache, d.session_id, &vide)
        .await
        .unwrap();
    assert_eq!(encore.id, tableau.id);
    assert_eq!(encore.participants_total, 2);

    let mal_forme = MeetingSyncInput {
        external_id: Some("857-abc".into()),
    };
    let erreur = meeting::reconstruire(&etat, &ctx, &detache, d.session_id, &mal_forme)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::ValidationFailed);

    let rattache = MeetingSyncInput {
        external_id: Some("857 4606 5432".into()),
    };
    // Rattacher une réunion existante exige la permission sur toute la plateforme.
    let erreur = meeting::reconstruire(&etat, &ctx, &detache, d.session_id, &rattache)
        .await
        .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::Forbidden);

    let globale = perimetre_de(&bac, c.globale).await;
    let ctx_globale = bac.ctx(c.globale);
    let tableau = meeting::reconstruire(&etat, &ctx_globale, &globale, d.session_id, &rattache)
        .await
        .unwrap();
    assert_eq!(tableau.external_id.as_deref(), Some("85746065432"));
    assert_eq!(tableau.sync_status, "pending");

    // Relancer le même rattachement reste ouvert au périmètre de l'édition.
    meeting::reconstruire(&etat, &ctx, &detache, d.session_id, &rattache)
        .await
        .unwrap();

    // Pendant qu'un travail tient la réunion, une relance ne la remet pas en file.
    sqlx::query!(
        "UPDATE live.meetings SET sync_status = 'in_progress' WHERE id = $1",
        tableau.id
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let pendant = meeting::reconstruire(&etat, &ctx, &detache, d.session_id, &vide)
        .await
        .unwrap();
    assert_eq!(pendant.sync_status, "in_progress");

    // La même réunion Zoom pour une autre séance : refusée.
    let erreur =
        meeting::reconstruire(&etat, &ctx_globale, &globale, d.autre_session_id, &rattache)
            .await
            .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::LiveMeetingExternalIdTaken);

    let lu = meeting::lire_admin(&etat, &detache, d.session_id)
        .await
        .unwrap();
    assert_eq!(lu.id, tableau.id);
    assert_eq!(
        meeting::lire_admin(&etat, &detache, d.autre_session_id)
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
}

#[tokio::test]
async fn sans_la_permission_rien_ne_se_cree() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let c = comptes(&bac, &d).await;
    commun::retirer_la_permission(&bac, "admin", "live.meeting.manage").await;
    let detache = perimetre_de(&bac, c.detache).await;
    let etat = etat_avec_zoom(&bac);

    let erreur = meeting::reconstruire(
        &etat,
        &bac.ctx(c.detache),
        &detache,
        d.session_id,
        &MeetingSyncInput::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(erreur.code, ErrorCode::Forbidden);
}
