//! **Signaler** (3b, US1) — par la route : chaque motif, la réunion non
//! annoncée, le rejeu d'une même référence (une ligne, `200`), le doublon en
//! attente (`409`), l'accès (`403`, `401`), la précision de 600 caractères, et
//! « Mes signalements » qui ne dit « validé » qu'une fois publié.

mod importation;

use actix_web::dev::Service;
use actix_web::http::header::{ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::test::{call_service, init_service, read_body, TestRequest};
use actix_web::{web, App, HttpMessage as _};
use importation::Bac;
use kernel::context::RequestContext;
use negotiation::state::NegotiationState;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

const ACTEUR: &str = "x-essai-acteur";

macro_rules! application {
    ($etat:expr, $db:expr) => {
        init_service(
            App::new()
                .app_data(web::Data::new($db))
                .app_data(web::Data::new($etat))
                .wrap_fn(|req, srv| {
                    let acteur = req
                        .headers()
                        .get(ACTEUR)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| Uuid::parse_str(v).ok());
                    let ctx = RequestContext::new(RequestContext::generated_request_id(), "fr");
                    req.extensions_mut().insert(match acteur {
                        Some(a) => ctx.with_actor(a),
                        None => ctx,
                    });
                    srv.call(req)
                })
                .configure(negotiation::routes),
        )
        .await
    };
}

/// Une macro plutôt qu'une fonction : le type de requête d'actix n'est pas
/// nommable sans dépendre d'`actix-http`.
macro_rules! frapper {
    ($app:expr, $requete:expr $(,)?) => {{
        let reponse = call_service($app, ($requete).to_request()).await;
        let statut = reponse.status();
        let etag = reponse
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let octets = read_body(reponse).await;
        (
            statut,
            serde_json::from_slice::<Value>(&octets).unwrap_or(Value::Null),
            etag,
        )
    }};
}

fn envoyer(acteur: Option<Uuid>, corps: Value) -> TestRequest {
    let r = TestRequest::post()
        .uri("/negotiation/reports")
        .set_json(corps);
    match acteur {
        Some(a) => r.insert_header((ACTEUR, a.to_string())),
        None => r,
    }
}

fn lire(acteur: Uuid, uri: &str) -> TestRequest {
    TestRequest::get()
        .uri(uri)
        .insert_header((ACTEUR, acteur.to_string()))
}

struct Decor {
    bac: Bac,
    etat: NegotiationState,
    awa: Uuid,
    sessions: Vec<Uuid>,
}

async fn personne(bac: &Bac, email: &str, role: Option<&str>) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', now()) RETURNING id",
    )
    .bind(email)
    .fetch_one(bac.pool())
    .await
    .expect("personne");
    if let Some(role) = role {
        sqlx::query(
            "INSERT INTO identity.role_assignments (person_id, role_code, scope_type)
             VALUES ($1, $2, 'global')",
        )
        .bind(id)
        .bind(role)
        .execute(bac.pool())
        .await
        .expect("rôle");
    }
    id
}

async fn monter() -> Decor {
    let bac = Bac::monter().await;
    bac.lire().await;
    let etat = NegotiationState::new(
        bac.base.db(),
        Arc::new(kernel::testing::test_config(bac.base.url())),
    );
    let awa = personne(&bac, "awa@example.org", Some("negotiator")).await;
    let sessions = sqlx::query_scalar(
        "SELECT id FROM negotiation.meetings WHERE event_id = $1 ORDER BY start_at, id LIMIT 4",
    )
    .bind(bac.edition)
    .fetch_all(bac.pool())
    .await
    .expect("sessions");
    Decor {
        bac,
        etat,
        awa,
        sessions,
    }
}

impl Decor {
    async fn lignes(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM negotiation.session_reports")
            .fetch_one(self.bac.pool())
            .await
            .expect("compte")
    }

    fn changement(&self, reason: &str, session: Uuid) -> Value {
        json!({
            "client_ref": Uuid::now_v7(), "edition": "cop31", "reason": reason,
            "session_id": session, "proposed_start": "2026-11-10T09:30:00Z",
            "proposed_venue": "Salle Pacifique", "detail": "Affiché à l'entrée."
        })
    }
}

#[tokio::test]
async fn chaque_motif_de_changement_secrit_sans_toucher_la_session() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let avant: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, row_to_json(m)::text FROM negotiation.meetings m WHERE event_id = $1 ORDER BY id",
    )
    .bind(d.bac.edition)
    .fetch_all(d.bac.pool())
    .await
    .expect("sessions avant");

    for (motif, session) in ["cancelled", "time", "venue", "other"]
        .into_iter()
        .zip(d.sessions.clone())
    {
        let (statut, r, _) = frapper!(&app, envoyer(Some(d.awa), d.changement(motif, session)));
        assert_eq!(statut, StatusCode::CREATED, "{motif} : {r}");
        assert_eq!(r["reason"], motif);
        assert_eq!(r["status"], "submitted");
        assert_eq!(r["session"]["id"], json!(session));
        assert_eq!(r["detail"], "Affiché à l'entrée.");
        assert_eq!(r["decided_at"], Value::Null);
        let heure = if motif == "time" {
            json!("2026-11-10T09:30:00Z")
        } else {
            Value::Null
        };
        let salle = if motif == "venue" {
            json!("Salle Pacifique")
        } else {
            Value::Null
        };
        assert_eq!(r["proposed_start"], heure, "{motif}");
        assert_eq!(r["proposed_venue"], salle, "{motif}");
    }
    assert_eq!(d.lignes().await, 4);

    let apres: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, row_to_json(m)::text FROM negotiation.meetings m WHERE event_id = $1 ORDER BY id",
    )
    .bind(d.bac.edition)
    .fetch_all(d.bac.pool())
    .await
    .expect("sessions après");
    assert_eq!(avant, apres, "un signalement ne touche jamais la session");

    let evenements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platform.outbox_events WHERE event_type LIKE 'negotiation.report.%'",
    )
    .fetch_one(d.bac.pool())
    .await
    .expect("outbox");
    assert_eq!(evenements, 0, "rien n'est émis à l'envoi");
}

#[tokio::test]
async fn la_reunion_non_annoncee_exige_quoi_et_le_jour() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let corps = |what: Value, day: Value| {
        json!({
            "client_ref": Uuid::now_v7(), "edition": "cop31", "reason": "unannounced",
            "what": what, "day": day, "proposed_venue": "Couloir B",
            "proposed_start": "2026-11-10T14:00:00Z", "theme": "finance",
            "session_id": Uuid::now_v7()
        })
    };

    let (statut, r, _) = frapper!(
        &app,
        envoyer(
            Some(d.awa),
            corps(json!("Groupe Afrique"), json!("2026-11-10")),
        ),
    );
    assert_eq!(statut, StatusCode::CREATED, "{r}");
    assert_eq!(
        r["session"],
        Value::Null,
        "session_id est ignoré pour ce motif"
    );
    assert_eq!(r["what"], "Groupe Afrique");
    assert_eq!(r["day"], "2026-11-10");
    assert_eq!(r["proposed_venue"], "Couloir B");
    let theme: String = sqlx::query_scalar(
        "SELECT t.code FROM negotiation.session_reports r
           JOIN reference.taxonomy_terms t ON t.id = r.theme_term_id",
    )
    .fetch_one(d.bac.pool())
    .await
    .expect("thématique");
    assert_eq!(theme, "finance");

    for (what, day, champ) in [
        (json!("  "), json!("2026-11-10"), "what"),
        (json!("Groupe Afrique"), Value::Null, "day"),
        (json!("Groupe Afrique"), json!("10/11/2026"), "day"),
    ] {
        let (statut, r, _) = frapper!(&app, envoyer(Some(d.awa), corps(what, day)));
        assert_eq!(statut, StatusCode::BAD_REQUEST);
        assert_eq!(r["code"], "NEGOTIATION_REPORT_INVALID");
        assert_eq!(r["field"], champ);
    }
    assert_eq!(d.lignes().await, 1);
}

#[tokio::test]
async fn un_envoi_rejoue_rend_200_et_la_meme_ligne() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let corps = d.changement("time", d.sessions[0]);

    let (premier, r1, _) = frapper!(&app, envoyer(Some(d.awa), corps.clone()));
    let (second, r2, _) = frapper!(&app, envoyer(Some(d.awa), corps));
    assert_eq!(premier, StatusCode::CREATED);
    assert_eq!(second, StatusCode::OK);
    assert_eq!(r1, r2);
    assert_eq!(d.lignes().await, 1);
}

#[tokio::test]
async fn un_second_signalement_en_attente_sur_la_meme_session_est_refuse() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let (statut, _, _) = frapper!(
        &app,
        envoyer(Some(d.awa), d.changement("time", d.sessions[0])),
    );
    assert_eq!(statut, StatusCode::CREATED);

    let (statut, r, _) = frapper!(
        &app,
        envoyer(Some(d.awa), d.changement("venue", d.sessions[0])),
    );
    assert_eq!(statut, StatusCode::CONFLICT);
    assert_eq!(r["code"], "NEGOTIATION_REPORT_DUPLICATE");
    assert_eq!(d.lignes().await, 1);

    let fatou = personne(&d.bac, "fatou@example.org", Some("negotiator")).await;
    let (statut, _, _) = frapper!(
        &app,
        envoyer(Some(fatou), d.changement("venue", d.sessions[0])),
    );
    assert_eq!(
        statut,
        StatusCode::CREATED,
        "une autre personne signale la même session"
    );
}

#[tokio::test]
async fn sans_acces_403_sans_compte_401_et_rien_ne_secrit() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let visiteuse = personne(&d.bac, "visiteuse@example.org", None).await;

    let (statut, r, _) = frapper!(
        &app,
        envoyer(Some(visiteuse), d.changement("time", d.sessions[0])),
    );
    assert_eq!(statut, StatusCode::FORBIDDEN);
    assert_eq!(r["code"], "NEGOTIATION_REPORT_FORBIDDEN");

    let (statut, _, _) = frapper!(&app, envoyer(None, d.changement("time", d.sessions[0])));
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
    let (statut, _, _) = frapper!(
        &app,
        TestRequest::get().uri("/negotiation/me/reports?edition=cop31"),
    );
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
    assert_eq!(d.lignes().await, 0);
}

#[tokio::test]
async fn champs_requis_session_inconnue_et_precision_de_600() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());

    let mut corps = d.changement("other", d.sessions[0]);
    corps["detail"] = json!("é".repeat(601));
    let (statut, r, _) = frapper!(&app, envoyer(Some(d.awa), corps.clone()));
    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert_eq!(
        (r["code"].as_str(), r["field"].as_str()),
        (Some("NEGOTIATION_REPORT_INVALID"), Some("detail"))
    );

    let mut sans_session = d.changement("cancelled", d.sessions[0]);
    sans_session["session_id"] = Value::Null;
    let (statut, r, _) = frapper!(&app, envoyer(Some(d.awa), sans_session));
    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert_eq!(r["field"], "session_id");

    let (statut, r, _) = frapper!(
        &app,
        envoyer(Some(d.awa), d.changement("cancelled", Uuid::now_v7())),
    );
    assert_eq!(statut, StatusCode::NOT_FOUND);
    assert_eq!(r["code"], "NEGOTIATION_SESSION_UNKNOWN");

    let mut autre_edition = d.changement("cancelled", d.sessions[0]);
    autre_edition["edition"] = json!("cop99");
    let (statut, r, _) = frapper!(&app, envoyer(Some(d.awa), autre_edition));
    assert_eq!(statut, StatusCode::NOT_FOUND);
    assert_eq!(r["code"], "NEGOTIATION_EDITION_UNKNOWN");
    assert_eq!(d.lignes().await, 0);

    corps["detail"] = json!("é".repeat(600));
    let (statut, _, _) = frapper!(&app, envoyer(Some(d.awa), corps));
    assert_eq!(statut, StatusCode::CREATED);
}

#[tokio::test]
async fn mes_signalements_ne_disent_valide_quune_fois_publie() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let admin = personne(&d.bac, "ifdd@example.org", Some("admin")).await;
    let (_, premier, _) = frapper!(
        &app,
        envoyer(Some(d.awa), d.changement("time", d.sessions[0])),
    );
    let (_, second, _) = frapper!(
        &app,
        envoyer(Some(d.awa), d.changement("venue", d.sessions[1])),
    );

    let (statut, liste, etag) =
        frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(
        liste["reports"][0]["id"], second["id"],
        "plus récent d'abord"
    );
    assert_eq!(liste["reports"][1]["id"], premier["id"]);
    let etag = etag.expect("ETag");
    let (statut, _, _) = frapper!(
        &app,
        lire(d.awa, "/negotiation/me/reports?edition=cop31")
            .insert_header((IF_NONE_MATCH, etag.clone())),
    );
    assert_eq!(statut, StatusCode::NOT_MODIFIED);

    let id = Uuid::parse_str(premier["id"].as_str().expect("id")).expect("uuid");
    sqlx::query(
        "UPDATE negotiation.session_reports
            SET status = 'validated', decided_by = $2, decided_at = now(), source_snapshot = '{}'
          WHERE id = $1",
    )
    .bind(id)
    .bind(admin)
    .execute(d.bac.pool())
    .await
    .expect("validation");
    let (_, liste, etag_valide) =
        frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(
        liste["reports"][1]["status"], "submitted",
        "validé mais pas publié"
    );
    assert_eq!(liste["reports"][1]["decided_at"], Value::Null);
    assert_eq!(
        etag_valide.as_deref(),
        Some(etag.as_str()),
        "rien de visible n'a changé"
    );

    sqlx::query("UPDATE negotiation.session_reports SET published_at = now() WHERE id = $1")
        .bind(id)
        .execute(d.bac.pool())
        .await
        .expect("publication");
    let (_, liste, _) = frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(liste["reports"][1]["status"], "validated");
    assert_ne!(liste["reports"][1]["decided_at"], Value::Null);
    assert!(
        liste.to_string().find("ifdd@example.org").is_none(),
        "aucun décideur nommé"
    );
}

#[tokio::test]
async fn la_reunion_non_annoncee_dit_sa_thematique_et_sa_fiche_une_fois_publiee() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let admin = personne(&d.bac, "ifdd@example.org", Some("admin")).await;
    let (statut, cree, _) = frapper!(
        &app,
        envoyer(
            Some(d.awa),
            json!({
                "client_ref": Uuid::now_v7(), "edition": "cop31", "reason": "unannounced",
                "what": "Aparté Afrique", "day": "2026-11-10", "theme": "finance"
            }),
        ),
    );
    assert_eq!(statut, StatusCode::CREATED, "{cree}");
    assert_eq!(cree["theme"], "finance");
    assert_eq!(cree["network_meeting_id"], Value::Null);

    let id = Uuid::parse_str(cree["id"].as_str().expect("id")).expect("uuid");
    let reunion: Uuid = sqlx::query_scalar(
        "INSERT INTO negotiation.network_meetings (event_id, report_id, title, day, validated_at)
         VALUES ($1, $2, 'Aparté Afrique', '2026-11-10', now()) RETURNING id",
    )
    .bind(d.bac.edition)
    .bind(id)
    .fetch_one(d.bac.pool())
    .await
    .expect("réunion");
    sqlx::query(
        "UPDATE negotiation.session_reports
            SET status = 'validated', decided_by = $2, decided_at = now(), source_snapshot = '{}',
                network_meeting_id = $3
          WHERE id = $1",
    )
    .bind(id)
    .bind(admin)
    .bind(reunion)
    .execute(d.bac.pool())
    .await
    .expect("validation");
    let (_, liste, _) = frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(
        liste["reports"][0]["network_meeting_id"],
        Value::Null,
        "rien avant la publication"
    );

    sqlx::query("UPDATE negotiation.session_reports SET published_at = now() WHERE id = $1")
        .bind(id)
        .execute(d.bac.pool())
        .await
        .expect("publication");
    let (_, liste, _) = frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(liste["reports"][0]["network_meeting_id"], reunion.to_string());
    assert_eq!(liste["reports"][0]["theme"], "finance");

    sqlx::query("UPDATE negotiation.network_meetings SET withdrawn_at = now() WHERE id = $1")
        .bind(reunion)
        .execute(d.bac.pool())
        .await
        .expect("retrait");
    let (_, liste, _) = frapper!(&app, lire(d.awa, "/negotiation/me/reports?edition=cop31"));
    assert_eq!(
        liste["reports"][0]["network_meeting_id"],
        Value::Null,
        "retirée, plus de fiche"
    );
}

#[tokio::test]
async fn lecriture_porte_son_autrice_et_sa_requete() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let (_, r, _) = frapper!(
        &app,
        envoyer(Some(d.awa), d.changement("other", d.sessions[0])),
    );
    let id = Uuid::parse_str(r["id"].as_str().expect("id")).expect("uuid");

    let traces: Vec<(String, Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT action, actor_id, request_id FROM platform.audit_log
          WHERE entity_table = 'session_reports' AND entity_id = $1",
    )
    .bind(id)
    .fetch_all(d.bac.pool())
    .await
    .expect("audit");
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].0, "insert");
    assert_eq!(traces[0].1, Some(d.awa));
    assert!(traces[0].2.as_deref().is_some_and(|r| !r.is_empty()));
}

#[tokio::test]
async fn mon_acces_dit_qui_peut_valider_et_combien_attendent() {
    let d = monter().await;
    let app = application!(d.etat.clone(), d.bac.base.db());
    let admin = personne(&d.bac, "ifdd@example.org", Some("admin")).await;
    frapper!(
        &app,
        envoyer(Some(d.awa), d.changement("time", d.sessions[0])),
    );
    frapper!(
        &app,
        envoyer(Some(d.awa), d.changement("venue", d.sessions[1])),
    );

    let (_, awa, _) = frapper!(&app, lire(d.awa, "/negotiation/me/access"));
    assert_eq!(awa["can_validate_reports"], false);
    assert_eq!(awa["reports_to_review"], Value::Null);

    let (_, ifdd, _) = frapper!(&app, lire(admin, "/negotiation/me/access"));
    assert_eq!(ifdd["can_validate_reports"], true);
    assert_eq!(ifdd["reports_to_review"], 2);
}
