//! **Valider** (3b, US2) — la garde globale, la file et la source du moment,
//! et la garantie SC-004 : rien n'est public avant la publication, une
//! validation annulée ne publie jamais rien, l'annulation et le travail se
//! départagent par le verrou de ligne, le travail rejoué ne fait rien.

mod importation;
#[macro_use]
mod decision;

use actix_web::http::StatusCode;
use decision::{dans, lire, monter, personne, poster, public};
use kernel::context::RequestContext;
use negotiation::jobs::publish::{publier, Issue};
use serde_json::{json, Value};
use std::cell::Cell;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use uuid::Uuid;

const FILE: &str = "/admin/negotiation/reports?edition=cop31";
const SESSIONS: &str = "/negotiation/sessions?edition=cop31";

fn route(id: Uuid, geste: &str) -> String {
    format!("/admin/negotiation/reports/{id}/{geste}")
}

async fn decide_a(d: &decision::Decor, id: Uuid) -> OffsetDateTime {
    sqlx::query_scalar("SELECT decided_at FROM negotiation.session_reports WHERE id = $1")
        .bind(id)
        .fetch_one(d.bac.pool())
        .await
        .expect("décision")
}

#[tokio::test]
async fn la_file_est_refusee_a_ladministrateur_dune_seule_edition() {
    let d = monter().await;
    let app = application!(d);
    let id = d.signaler("other", d.sessions[0], None, None).await;
    let admin_cop = personne(&d.bac, "admin.cop31@example.org", "Moussa", None).await;
    sqlx::query(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, 'admin', 'event', $2)",
    )
    .bind(admin_cop)
    .bind(d.bac.edition)
    .execute(d.bac.pool())
    .await
    .expect("rôle sur l'édition");

    for acteur in [admin_cop, d.awa] {
        let (statut, _, _) = frapper!(&app, lire(acteur, FILE));
        assert_eq!(statut, StatusCode::FORBIDDEN);
        let (statut, _, _) = frapper!(&app, poster(acteur, &route(id, "validate")));
        assert_eq!(statut, StatusCode::FORBIDDEN);
    }
    let (statut, _, _) = frapper!(&app, public(FILE));
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
    assert!(d.travaux().await.is_empty(), "rien n'est posé");
}

#[tokio::test]
async fn la_file_montre_lautrice_et_la_source_du_moment() {
    let d = monter().await;
    let app = application!(d);
    let premier = d
        .signaler("time", d.sessions[0], Some("2026-11-10T09:30:00Z"), None)
        .await;
    let second = d
        .signaler("venue", d.sessions[1], None, Some("Salle 4"))
        .await;

    let (statut, file, _) = frapper!(&app, lire(d.ifdd, FILE));
    assert_eq!(statut, StatusCode::OK, "{file}");
    let attente = file["pending"].as_array().expect("pending");
    assert_eq!(attente.len(), 2);
    assert_eq!(attente[0]["id"], json!(premier), "les plus anciens d'abord");
    assert_eq!(attente[1]["id"], json!(second));
    assert_eq!(attente[0]["author"]["name"], "Awa Diallo");
    let source = &attente[0]["source_now"];
    assert_eq!(source["status"], "scheduled");
    assert_eq!(source["start_at"], attente[0]["session"]["start_at"]);
    assert!(source["read_at"].is_string(), "heure de lecture : {source}");
    assert_eq!(file["decided_today"], json!([]));

    let (statut, item, _) = frapper!(&app, poster(d.ifdd, &route(premier, "validate")));
    assert_eq!(statut, StatusCode::OK, "{item}");
    assert_eq!(item["status"], "validated");
    assert_eq!(item["published_at"], Value::Null);
    assert_eq!(item["decided_by"], "Ines Diallo");

    let (_, file, _) = frapper!(&app, lire(d.ifdd, FILE));
    assert_eq!(file["pending"].as_array().map(Vec::len), Some(1));
    assert_eq!(file["decided_today"][0]["id"], json!(premier));

    let (statut, r, _) = frapper!(&app, poster(d.ifdd, &route(premier, "validate")));
    assert_eq!(statut, StatusCode::CONFLICT);
    assert_eq!(r["code"], "NEGOTIATION_REPORT_ALREADY_DECIDED");
    let (statut, _, _) = frapper!(&app, poster(d.ifdd, &route(Uuid::now_v7(), "validate")));
    assert_eq!(statut, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn valider_ne_rend_rien_public_avant_la_publication() {
    let d = monter().await;
    let app = application!(d);
    let fatou = personne(&d.bac, "fatou@example.org", "Fatou", Some("negotiator")).await;
    d.garder(fatou, d.sessions[0]).await;
    let id = d
        .signaler("time", d.sessions[0], Some("2026-11-10T09:30:00Z"), None)
        .await;

    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    let (ecart, snapshot): (f64, Value) = sqlx::query_as(
        "SELECT extract(epoch FROM j.run_at - r.decided_at)::float8, r.source_snapshot
           FROM platform.jobs j, negotiation.session_reports r
          WHERE j.task = 'negotiation.report.publish' AND r.id = $1",
    )
    .bind(id)
    .fetch_one(d.bac.pool())
    .await
    .expect("publication posée");
    assert!(
        (ecart - 30.0).abs() < 1.0,
        "trente secondes, horloge de la base : {ecart}"
    );
    assert_eq!(snapshot["status"], "scheduled", "la source du moment");

    let (_, sessions, etag_avant) = frapper!(&app, public(SESSIONS));
    assert_eq!(dans(&sessions, d.sessions[0])["network_reports"], json!([]));
    let (_, mes, _) = frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(mes["reports"][0]["status"], "submitted");
    assert!(
        d.evenements().await.is_empty(),
        "rien n'est émis à la validation"
    );

    d.publier_tout().await;
    let (_, sessions, etag_apres) = frapper!(&app, public(SESSIONS));
    let encart = &dans(&sessions, d.sessions[0])["network_reports"];
    assert_eq!(encart[0]["reason"], "time");
    assert_eq!(encart[0]["proposed_start"], "2026-11-10T09:30:00Z");
    assert!(encart[0]["validated_at"].is_string());
    assert_ne!(etag_avant, etag_apres, "l'empreinte suit l'encart");
    let (_, mes, _) = frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(mes["reports"][0]["status"], "validated");

    let evenements = d.evenements().await;
    let types: Vec<&str> = evenements.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        types,
        ["negotiation.report.published", "negotiation.report.decided"]
    );
    let avis = &evenements[0].1["notification"];
    assert_eq!(avis["recipients"], json!([fatou]));
    assert!(avis["title"]["fr"]
        .as_str()
        .expect("titre")
        .starts_with("Déplacée — "));
    let corps = avis["body"]["fr"].as_str().expect("corps");
    assert!(corps.contains("signalé par le réseau") && corps.ends_with("Sessions de négociation"));
    assert_eq!(avis["replace"], true);
    assert_eq!(
        evenements[1].1["notification"]["recipients"],
        json!([d.awa])
    );
    assert!(
        !evenements
            .iter()
            .any(|(_, p)| p.to_string().contains("Awa")),
        "jamais l'autrice"
    );
}

#[tokio::test]
async fn valider_puis_annuler_ne_publie_jamais_rien() {
    let d = monter().await;
    let app = application!(d);
    d.garder(d.awa, d.sessions[0]).await;
    let id = d.signaler("cancelled", d.sessions[0], None, None).await;

    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    let (statut, item, _) = frapper!(&app, poster(d.ifdd, &route(id, "undo")));
    assert_eq!(statut, StatusCode::OK, "{item}");
    assert_eq!(item["status"], "submitted");
    assert_eq!(item["decided_by"], Value::Null);
    let (statut, _, _) = frapper!(&app, poster(d.ifdd, &route(id, "undo")));
    assert_eq!(statut, StatusCode::OK, "rejouée, l'annulation rend l'état");

    d.publier_tout().await;
    assert!(d.publie_a(id).await.is_none());
    assert!(d.evenements().await.is_empty(), "aucun avis (SC-004)");
    let courriels: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platform.jobs WHERE task = 'negotiation.session_change_email'",
    )
    .fetch_one(d.bac.pool())
    .await
    .expect("courriels");
    assert_eq!(courriels, 0);

    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    frapper!(&app, poster(d.ifdd, &route(id, "undo")));
    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    let travaux = d.travaux().await;
    assert_eq!(travaux.len(), 2, "une publication par validation");
    for t in &travaux {
        d.executer(t).await;
    }
    let publies = d
        .evenements()
        .await
        .iter()
        .filter(|(t, _)| t == "negotiation.report.published")
        .count();
    assert_eq!(publies, 1, "seule la dernière validation publie");
}

#[tokio::test]
async fn le_travail_rejoue_ne_publie_pas_deux_fois_et_lannulation_arrive_trop_tard() {
    let d = monter().await;
    let app = application!(d);
    let id = d.signaler("other", d.sessions[0], None, None).await;
    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    let travaux = d.publier_tout().await;
    let apres_une = d.evenements().await.len();
    d.executer(&travaux[0]).await;
    assert_eq!(
        d.evenements().await.len(),
        apres_une,
        "rejoué, il ne fait rien"
    );

    let (statut, r, _) = frapper!(&app, poster(d.ifdd, &route(id, "undo")));
    assert_eq!(statut, StatusCode::CONFLICT);
    assert_eq!(r["code"], "NEGOTIATION_REPORT_UNDO_EXPIRED");
}

#[tokio::test]
async fn annuler_pendant_que_le_travail_tient_la_ligne_echoue() {
    let d = monter().await;
    let app = application!(d);
    d.garder(d.awa, d.sessions[0]).await;
    let id = d.signaler("cancelled", d.sessions[0], None, None).await;
    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    let decide = decide_a(&d, id).await;

    let ctx = RequestContext::new("essai:travail", "fr");
    let mut travail = d.bac.base.db().write(&ctx).await.expect("transaction");
    assert!(matches!(
        publier(&mut travail, id, decide)
            .await
            .expect("publication"),
        Issue::Publie
    ));

    let valide_a = Cell::new(None::<Instant>);
    let annulation = async {
        let r = frapper!(&app, poster(d.ifdd, &route(id, "undo")));
        (r, Instant::now())
    };
    let validation = async {
        tokio::time::sleep(Duration::from_millis(400)).await;
        valide_a.set(Some(Instant::now()));
        travail.commit().await.expect("commit");
    };
    let (((statut, r, _), fin_annulation), ()) = tokio::join!(annulation, validation);

    assert!(
        fin_annulation >= valide_a.get().expect("validé"),
        "l'annulation a attendu la ligne"
    );
    assert_eq!(statut, StatusCode::CONFLICT, "{r}");
    assert_eq!(r["code"], "NEGOTIATION_REPORT_UNDO_EXPIRED");
    assert!(d.publie_a(id).await.is_some());
}

#[tokio::test]
async fn le_travail_qui_attend_une_annulation_en_cours_ne_publie_rien() {
    let d = monter().await;
    let app = application!(d);
    d.garder(d.awa, d.sessions[0]).await;
    let id = d.signaler("cancelled", d.sessions[0], None, None).await;
    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    let travail = d.travaux().await.remove(0);

    let ctx = RequestContext::new("essai:annulation", "fr");
    let mut annulation = d.bac.base.db().write(&ctx).await.expect("transaction");
    assert!(
        negotiation::repo::admin_reports::annuler(&mut annulation, id)
            .await
            .expect("annulation")
    );

    let annule_a = Cell::new(None::<Instant>);
    let execution = async {
        d.executer(&travail).await;
        Instant::now()
    };
    let validation = async {
        tokio::time::sleep(Duration::from_millis(400)).await;
        annule_a.set(Some(Instant::now()));
        annulation.commit().await.expect("commit");
    };
    let (fin_travail, ()) = tokio::join!(execution, validation);

    assert!(
        fin_travail >= annule_a.get().expect("annulé"),
        "le travail a attendu la ligne"
    );
    assert!(d.publie_a(id).await.is_none());
    assert!(
        d.evenements().await.is_empty(),
        "l'un ou l'autre, jamais les deux"
    );
}

#[tokio::test]
async fn annuler_heurtant_un_nouveau_signalement_en_attente_rend_409() {
    let d = monter().await;
    let app = application!(d);
    let id = d
        .signaler("time", d.sessions[0], Some("2026-11-10T09:30:00Z"), None)
        .await;
    frapper!(&app, poster(d.ifdd, &route(id, "validate")));
    d.signaler("venue", d.sessions[0], None, Some("Salle 4"))
        .await;

    let (statut, r, _) = frapper!(&app, poster(d.ifdd, &route(id, "undo")));
    assert_eq!(statut, StatusCode::CONFLICT);
    assert_eq!(r["code"], "NEGOTIATION_REPORT_UNDO_EXPIRED");
}

#[tokio::test]
async fn le_refus_donne_son_motif_a_lautrice() {
    let d = monter().await;
    let app = application!(d);
    let id = d
        .signaler("venue", d.sessions[0], None, Some("Salle 4"))
        .await;

    let (statut, r, _) = frapper!(
        &app,
        poster(d.ifdd, &route(id, "reject")).set_json(json!({ "reason": "autre" })),
    );
    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert_eq!(r["field"], "reason");

    let (statut, item, _) = frapper!(
        &app,
        poster(d.ifdd, &route(id, "reject"))
            .set_json(json!({ "reason": "source_maintains", "detail": "Vu à l'écran." })),
    );
    assert_eq!(statut, StatusCode::OK, "{item}");
    assert_eq!(item["status"], "rejected");

    let (_, mes, _) = frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(mes["reports"][0]["status"], "rejected");
    assert_eq!(mes["reports"][0]["reject_reason"], "source_maintains");
    assert_eq!(mes["reports"][0]["reject_detail"], "Vu à l'écran.");

    let evenements = d.evenements().await;
    assert_eq!(evenements.len(), 1);
    assert_eq!(evenements[0].0, "negotiation.report.decided");
    let avis = &evenements[0].1["notification"];
    assert_eq!(avis["recipients"], json!([d.awa]));
    assert!(avis["body"]["fr"]
        .as_str()
        .expect("corps")
        .starts_with("Non retenu — "));
    assert_eq!(avis["group_key"], Value::Null);

    let (statut, _, _) = frapper!(
        &app,
        poster(d.ifdd, &route(id, "reject")).set_json(json!({ "reason": "already_known" })),
    );
    assert_eq!(statut, StatusCode::CONFLICT);
    assert!(d.travaux().await.is_empty(), "un refus ne publie rien");
}

#[tokio::test]
async fn retirer_ote_lencart_affiche() {
    let d = monter().await;
    let app = application!(d);
    let id = d.signaler("other", d.sessions[0], None, None).await;
    frapper!(&app, poster(d.ifdd, &route(id, "validate")));

    let (statut, r, _) = frapper!(&app, poster(d.ifdd, &route(id, "withdraw")));
    assert_eq!(statut, StatusCode::CONFLICT, "pas encore affiché : {r}");

    d.publier_tout().await;
    let (_, sessions, _) = frapper!(&app, public(SESSIONS));
    assert_eq!(
        dans(&sessions, d.sessions[0])["network_reports"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let (statut, item, _) = frapper!(&app, poster(d.ifdd, &route(id, "withdraw")));
    assert_eq!(statut, StatusCode::OK);
    assert!(item["withdrawn_at"].is_string());
    let (statut, _, _) = frapper!(&app, poster(d.ifdd, &route(id, "withdraw")));
    assert_eq!(statut, StatusCode::OK, "idempotent");
    let (_, sessions, _) = frapper!(&app, public(SESSIONS));
    assert_eq!(dans(&sessions, d.sessions[0])["network_reports"], json!([]));

    let retrait: Option<String> =
        sqlx::query_scalar("SELECT withdrawal FROM negotiation.session_reports WHERE id = $1")
            .bind(id)
            .fetch_one(d.bac.pool())
            .await
            .expect("retrait");
    assert_eq!(retrait.as_deref(), Some("admin"));
}
