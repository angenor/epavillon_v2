//! **Afficher par-dessus** (3b, US3) — la session officielle n'est jamais
//! touchée (SC-003), l'import rattrape l'encart (SC-007), la fin de session le
//! cache, la réunion non annoncée reste servie quand l'affichage est coupé, et
//! aucun nom d'autrice ne sort (SC-008). « Mon agenda » garde une réunion non
//! annoncée.

mod importation;
#[macro_use]
mod decision;

use actix_web::http::StatusCode;
use actix_web::test::TestRequest;
use decision::{dans, lire, monter, poster, public, ACTEUR};
use serde_json::{json, Value};
use uuid::Uuid;

const SESSIONS: &str = "/negotiation/sessions?edition=cop31";

fn valider(ifdd: Uuid, id: Uuid) -> TestRequest {
    poster(ifdd, &format!("/admin/negotiation/reports/{id}/validate"))
}

async fn officiel(d: &decision::Decor) -> (Value, Value) {
    sqlx::query_as(
        "SELECT (SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM negotiation.meetings m
                  WHERE m.event_id = $1),
                coalesce((SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id) FROM negotiation.meeting_changes c), '[]')",
    )
    .bind(d.bac.edition)
    .fetch_one(d.bac.pool())
    .await
    .expect("état officiel")
}

async fn retrait(d: &decision::Decor, id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT withdrawal FROM negotiation.session_reports WHERE id = $1")
        .bind(id)
        .fetch_one(d.bac.pool())
        .await
        .expect("signalement")
}

#[tokio::test]
async fn la_session_officielle_ne_bouge_pas_dun_octet() {
    let d = monter().await;
    let app = application!(d);
    d.bac.jeu("cop30/lecture-2").await;
    d.bac.lire().await;
    let avant = officiel(&d).await;
    assert_ne!(avant.1, json!([]), "des changements constatés existent");

    for (motif, i) in [("time", 0), ("venue", 1), ("cancelled", 2), ("other", 3)] {
        let id = d
            .signaler(
                motif,
                d.sessions[i],
                Some("2026-11-10T09:30:00Z"),
                Some("Salle 4"),
            )
            .await;
        frapper!(&app, valider(d.ifdd, id));
    }
    d.publier_tout().await;

    assert_eq!(officiel(&d).await, avant, "SC-003");
}

#[tokio::test]
async fn limport_rattrape_lencart_a_sa_premiere_lecture() {
    let d = monter().await;
    let app = application!(d);
    let heure = d
        .signaler(
            "time",
            d.session("654214").await,
            Some("2026-11-10T11:00:40Z"),
            None,
        )
        .await;
    let salle = d
        .signaler(
            "venue",
            d.session("654010").await,
            None,
            Some("meeting  room 03"),
        )
        .await;
    let annulee = d
        .signaler("cancelled", d.session("654364").await, None, None)
        .await;
    for id in [heure, salle, annulee] {
        frapper!(&app, valider(d.ifdd, id));
    }
    let autre = d
        .signaler("other", d.session("654214").await, None, None)
        .await;
    frapper!(&app, valider(d.ifdd, autre));
    d.publier_tout().await;

    d.bac.jeu("cop30/lecture-2").await;
    d.bac.lire().await;
    assert_eq!(retrait(&d, heure).await.as_deref(), Some("caught_up"));
    assert_eq!(retrait(&d, salle).await.as_deref(), Some("caught_up"));
    assert_eq!(
        retrait(&d, annulee).await,
        None,
        "une absence ne l'annule pas encore"
    );

    d.bac.lire().await;
    assert_eq!(
        retrait(&d, annulee).await.as_deref(),
        Some("caught_up"),
        "annulée par disparition : rattrapée"
    );
    assert_eq!(retrait(&d, autre).await, None, "« Autre chose » jamais");

    let (_, sessions, _) = frapper!(&app, public(SESSIONS));
    let encarts = &dans(&sessions, d.session("654214").await)["network_reports"];
    assert_eq!(encarts.as_array().map(Vec::len), Some(1));
    assert_eq!(encarts[0]["reason"], "other");
}

#[tokio::test]
async fn lencart_dune_session_terminee_nest_plus_servi() {
    let d = monter().await;
    let app = application!(d);
    let id = d.signaler("other", d.sessions[0], None, None).await;
    frapper!(&app, valider(d.ifdd, id));
    d.publier_tout().await;

    sqlx::query(
        "UPDATE negotiation.meetings
            SET start_at = now() - interval '3 hours', end_at = now() - interval '2 hours'
          WHERE id = $1",
    )
    .bind(d.sessions[0])
    .execute(d.bac.pool())
    .await
    .expect("session terminée");
    let (_, sessions, _) = frapper!(&app, public(SESSIONS));
    assert_eq!(dans(&sessions, d.sessions[0])["network_reports"], json!([]));
    assert_eq!(retrait(&d, id).await, None, "la fin ne s'écrit pas");
}

#[tokio::test]
async fn la_reunion_non_annoncee_reste_servie_quand_laffichage_est_coupe() {
    let d = monter().await;
    let app = application!(d);
    let id = d.non_annoncee("2026-11-10").await;
    frapper!(&app, valider(d.ifdd, id));

    let (_, sessions, _) = frapper!(&app, public(SESSIONS));
    assert_eq!(
        sessions["network_meetings"],
        json!([]),
        "rien avant la publication"
    );

    d.publier_tout().await;
    d.bac.regler("is_enabled = false").await;
    let (statut, sessions, _) = frapper!(&app, public(SESSIONS));
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(sessions["state"], "cut");
    assert_eq!(sessions["sessions"], json!([]));
    let reunion = &sessions["network_meetings"][0];
    assert_eq!(reunion["title"], "Groupe Afrique");
    assert_eq!(reunion["day"], "2026-11-10");
    assert_eq!(reunion["venue"], "Couloir B");
    assert_eq!(reunion["theme"], "finance");
    assert!(reunion["validated_at"].is_string());

    let texte = sessions.to_string();
    for nom in ["Awa", "Diallo", "awa@example.org"] {
        assert!(!texte.contains(nom), "SC-008 : {nom}");
    }

    let evenements = d.evenements().await;
    assert_eq!(
        evenements
            .iter()
            .map(|(t, _)| t.as_str())
            .collect::<Vec<_>>(),
        ["negotiation.report.decided"],
        "personne ne suit encore la réunion : aucun avis de publication"
    );

    sqlx::query("UPDATE negotiation.network_meetings SET day = day - 400")
        .execute(d.bac.pool())
        .await
        .expect("jour passé");
    let (_, sessions, _) = frapper!(&app, public(SESSIONS));
    assert_eq!(sessions["network_meetings"], json!([]), "jour passé");
}

#[tokio::test]
async fn aucun_nom_dautrice_dans_la_reponse_publique() {
    let d = monter().await;
    let app = application!(d);
    let id = d
        .signaler("venue", d.sessions[0], None, Some("Salle 4"))
        .await;
    frapper!(&app, valider(d.ifdd, id));
    d.publier_tout().await;

    let (_, sessions, _) = frapper!(&app, public(SESSIONS));
    let encart = &dans(&sessions, d.sessions[0])["network_reports"][0];
    assert_eq!(encart["proposed_venue"], "Salle 4");
    let texte = sessions.to_string();
    for nom in ["Awa", "Diallo", "Ines", "awa@example.org", "author"] {
        assert!(!texte.contains(nom), "SC-008 : {nom}");
    }
}

#[tokio::test]
async fn mon_agenda_garde_une_reunion_non_annoncee() {
    let d = monter().await;
    let app = application!(d);
    let id = d.non_annoncee("2026-11-10").await;
    frapper!(&app, valider(d.ifdd, id));
    let reunion: Option<Uuid> = sqlx::query_scalar(
        "SELECT network_meeting_id FROM negotiation.session_reports WHERE id = $1",
    )
    .bind(id)
    .fetch_one(d.bac.pool())
    .await
    .expect("signalement");
    assert!(reunion.is_none(), "la réunion ne naît qu'à la publication");
    d.publier_tout().await;
    let reunion: Uuid = sqlx::query_scalar(
        "SELECT network_meeting_id FROM negotiation.session_reports WHERE id = $1",
    )
    .bind(id)
    .fetch_one(d.bac.pool())
    .await
    .expect("réunion");

    let garder = |cible: Uuid| {
        TestRequest::put()
            .uri(&format!("/negotiation/me/agenda/network/{cible}"))
            .insert_header((ACTEUR, d.awa.to_string()))
            .set_json(json!({ "remind": true }))
    };
    let (statut, _, _) = frapper!(&app, garder(reunion));
    assert_eq!(statut, StatusCode::NO_CONTENT);
    let (statut, _, _) = frapper!(&app, garder(reunion));
    assert_eq!(statut, StatusCode::NO_CONTENT, "idempotent");
    let (statut, r, _) = frapper!(&app, garder(Uuid::now_v7()));
    assert_eq!(statut, StatusCode::NOT_FOUND);
    assert_eq!(r["code"], "NEGOTIATION_SESSION_UNKNOWN");

    let (_, agenda, etag) = frapper!(&app, lire(d.awa, "/negotiation/me/agenda"));
    assert_eq!(agenda["entries"], json!([]));
    assert_eq!(
        agenda["network_entries"][0]["network_meeting_id"],
        json!(reunion)
    );
    assert_eq!(agenda["network_entries"][0]["remind"], true);

    let destinataires: Vec<Uuid> = sqlx::query_scalar("SELECT negotiation.network_recipients($1)")
        .bind(reunion)
        .fetch_all(d.bac.pool())
        .await
        .expect("destinataires");
    assert_eq!(destinataires, [d.awa]);

    let (statut, _, _) = frapper!(
        &app,
        TestRequest::delete()
            .uri(&format!("/negotiation/me/agenda/network/{reunion}"))
            .insert_header((ACTEUR, d.awa.to_string())),
    );
    assert_eq!(statut, StatusCode::NO_CONTENT);
    let (_, agenda, etag_apres) = frapper!(&app, lire(d.awa, "/negotiation/me/agenda"));
    assert_eq!(agenda["network_entries"], json!([]));
    assert_ne!(etag, etag_apres, "l'empreinte suit les réunions gardées");

    frapper!(
        &app,
        poster(d.ifdd, &format!("/admin/negotiation/reports/{id}/withdraw")),
    );
    let (statut, _, _) = frapper!(&app, garder(reunion));
    assert_eq!(statut, StatusCode::NOT_FOUND, "retirée");
}
