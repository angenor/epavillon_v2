//! **Les termes proposés**, en HTTP sur base réelle : regroupés par forme
//! normalisée, rejoués par `client_ref`, refusés s'ils sont déjà au lexique
//! (avec le `slug`), plafonnés ; la file les rend sans auteur ; accepter donne
//! un brouillon, refuser demande un motif ; publier l'entrée née met un
//! courriel en file par auteur, une fois (FR-025 à FR-027, R8, R9, R11).

mod commun;

use actix_web::http::StatusCode;
use async_trait::async_trait;
use commun::documents::expert;
use commun::http::{appel, frapper, ACTEUR};
use commun::savoir::terme;
use commun::{personne, Bac};
use kernel::mail::{MailError, Mailer, OutgoingMail};
use negotiation::jobs::emails::SEND_PUBLISHED_EMAIL;
use negotiation::service::savoir_admin::{changer_terme, Transition};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const PROPOSER: &str = "/negotiation/glossary/proposals";

/// Les routes du téléphone et celles du back-office, dans une même application.
macro_rules! application {
    ($bac:expr) => {
        actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new($bac.db()))
                .app_data(actix_web::web::Data::new($bac.state.clone()))
                .wrap_fn(|req, srv| {
                    let acteur = req
                        .headers()
                        .get(ACTEUR)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| Uuid::parse_str(v).ok());
                    let ctx = kernel::context::RequestContext::new(
                        kernel::context::RequestContext::generated_request_id(),
                        "fr",
                    );
                    actix_web::HttpMessage::extensions_mut(&req).insert(match acteur {
                        Some(a) => ctx.with_actor(a),
                        None => ctx,
                    });
                    actix_web::dev::Service::call(srv, req)
                })
                .configure(negotiation::routes)
                .configure(negotiation::admin_routes),
        )
        .await
    };
}

macro_rules! http {
    ($app:expr, $verbe:expr, $uri:expr, $qui:expr) => {
        frapper(&$app, appel($verbe, &$uri, $qui, None).to_request()).await
    };
    ($app:expr, $verbe:expr, $uri:expr, $qui:expr, $corps:expr) => {
        frapper(&$app, appel($verbe, &$uri, $qui, Some($corps)).to_request()).await
    };
}

#[derive(Default)]
struct Boite(Mutex<Vec<OutgoingMail>>);

#[async_trait]
impl Mailer for Boite {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        self.0.lock().expect("boîte").push(mail.clone());
        Ok(())
    }
}

async fn auteur(bac: &Bac, email: &str, prenom: &str, nom: &str) -> Uuid {
    let p = personne(bac, email).await;
    sqlx::query("UPDATE identity.people SET first_name = $2, last_name = $3 WHERE id = $1")
        .bind(p)
        .bind(prenom)
        .bind(nom)
        .execute(bac.pool())
        .await
        .expect("nom");
    p
}

fn proposition(term: &str, context: Option<&str>) -> Value {
    json!({ "client_ref": Uuid::now_v7(), "term": term, "context": context })
}

async fn compter(bac: &Bac, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(bac.pool())
        .await
        .expect("compte")
}

async fn auteurs(bac: &Bac) -> i64 {
    compter(
        bac,
        "SELECT count(*) FROM negotiation.glossary_proposal_authors",
    )
    .await
}

async fn courriels(bac: &Bac) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM platform.jobs WHERE task = $1")
        .bind(SEND_PUBLISHED_EMAIL)
        .fetch_one(bac.pool())
        .await
        .expect("compte des travaux")
}

fn sans_trace(corps: &Value, traces: &[String], ou: &str) {
    let texte = corps.to_string();
    for t in traces {
        assert!(
            !texte.contains(t.as_str()),
            "{ou} laisse voir « {t} » : {texte}"
        );
    }
}

#[tokio::test]
async fn le_meme_terme_se_regroupe_se_rejoue_et_la_file_ne_rend_aucun_auteur() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let awa = auteur(&bac, "awa.proposante@example.org", "Awaprop", "Kaboreh").await;
    let moussa = auteur(
        &bac,
        "moussa.proposant@example.org",
        "Moussaprop",
        "Traorem",
    )
    .await;
    let app = application!(bac);

    let premiere = proposition("Bracketed text", Some("  Entendu en plénière.  "));
    let (statut, recu) = http!(app, "post", PROPOSER, Some(awa), premiere.clone());
    assert_eq!(statut, StatusCode::CREATED, "{recu}");
    assert_eq!(recu["term"], "Bracketed text");
    assert_eq!(recu["client_ref"], premiere["client_ref"]);
    let id = recu["id"].clone();

    let (statut, rejeu) = http!(app, "post", PROPOSER, Some(awa), premiere);
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(rejeu, recu, "le rejeu rend le même reçu");
    let (statut, redit) = http!(
        app,
        "post",
        PROPOSER,
        Some(awa),
        proposition("bracketed TEXT", None)
    );
    assert_eq!(
        statut,
        StatusCode::OK,
        "le même terme, redit : rien ne naît"
    );
    assert_eq!(redit["id"], id);
    assert_eq!(auteurs(&bac).await, 1);

    let (statut, second) = http!(
        app,
        "post",
        PROPOSER,
        Some(moussa),
        proposition("  BRÂCKETED-text ! ", Some("Dans un projet de décision."))
    );
    assert_eq!(statut, StatusCode::CREATED, "{second}");
    assert_eq!(second["id"], id, "regroupé à la casse et aux accents près");
    assert_eq!(second["term"], "Bracketed text");
    assert_eq!(
        compter(&bac, "SELECT count(*) FROM negotiation.glossary_proposals").await,
        1
    );

    let traces: Vec<String> = [awa.to_string(), moussa.to_string()]
        .into_iter()
        .chain(
            [
                "awa.proposante",
                "moussa.proposant",
                "Awaprop",
                "Moussaprop",
                "Kaboreh",
                "Traorem",
            ]
            .map(str::to_owned),
        )
        .collect();
    let (statut, file) = http!(
        app,
        "get",
        "/admin/negotiation/queue?kind=proposals",
        Some(experte)
    );
    assert_eq!(statut, StatusCode::OK, "{file}");
    assert_eq!(file["kind"], "proposals");
    assert_eq!(file["counts"]["proposals"], 1);
    assert_eq!(file["questions"], json!([]));
    assert_eq!(file["reports"], json!([]));
    let ligne = &file["proposals"][0];
    assert_eq!(ligne["id"], id);
    assert_eq!(ligne["status"], "pending");
    assert_eq!(ligne["authors_count"], 2);
    sans_trace(&file, &traces, "la file");

    let (statut, fiche) = http!(
        app,
        "get",
        format!(
            "/admin/negotiation/queue/proposals/{}",
            id.as_str().unwrap()
        ),
        Some(experte)
    );
    assert_eq!(statut, StatusCode::OK);
    let contextes: Vec<_> = fiche["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["context"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        contextes,
        vec!["Entendu en plénière.", "Dans un projet de décision."]
    );
    sans_trace(&fiche, &traces, "la fiche");

    let (statut, _) = http!(
        app,
        "get",
        format!("/admin/negotiation/queue/proposals/{}", Uuid::now_v7()),
        Some(experte)
    );
    assert_eq!(statut, StatusCode::NOT_FOUND);
    let (statut, _) = http!(app, "post", PROPOSER, None, proposition("Chair", None));
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn deja_au_lexique_trop_long_ou_trop_nombreux_est_refuse() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa@example.org").await;
    terme(&bac, "Contact group", "published", None, &[]).await;
    terme(
        &bac,
        "Informal consultations",
        "to_review",
        Some("infs"),
        &[],
    )
    .await;
    terme(&bac, "Draft decision", "draft", None, &[]).await;
    let proche = terme(&bac, "Contact groups", "draft", None, &[]).await;
    let app = application!(bac);

    for (texte, slug) in [
        ("contact  GROUP", "contact-group"),
        ("INFS", "informal-consultations"),
    ] {
        let (statut, r) = http!(app, "post", PROPOSER, Some(awa), proposition(texte, None));
        assert_eq!(
            (statut, r["code"].as_str(), r["slug"].as_str()),
            (
                StatusCode::CONFLICT,
                Some("NEGOTIATION_GLOSSARY_TERM_EXISTS"),
                Some(slug)
            ),
            "{texte} : {r}"
        );
        assert_eq!(r["message"], "Ce terme est déjà dans le lexique.");
    }

    let (statut, r) = http!(
        app,
        "post",
        PROPOSER,
        Some(awa),
        proposition("Contact grp", Some(&"é".repeat(601)))
    );
    assert_eq!(
        (statut, r["code"].as_str(), r["field"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("NEGOTIATION_TEXT_TOO_LONG"),
            Some("context")
        )
    );
    for vide in ["   ", "?!", &"a".repeat(201)] {
        let (statut, r) = http!(app, "post", PROPOSER, Some(awa), proposition(vide, None));
        assert_eq!(
            (statut, r["field"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some("term"))
        );
    }
    assert_eq!(auteurs(&bac).await, 0, "aucun refus n'a écrit");

    // Un brouillon n'est pas au lexique : on le propose, et la file le montre proche.
    let (statut, _) = http!(
        app,
        "post",
        PROPOSER,
        Some(awa),
        proposition("Draft decision", None)
    );
    assert_eq!(statut, StatusCode::CREATED);
    for i in 1..10 {
        let (statut, r) = http!(
            app,
            "post",
            PROPOSER,
            Some(awa),
            proposition(&format!("Contact group {i}"), None)
        );
        assert_eq!(statut, StatusCode::CREATED, "{i} : {r}");
    }
    let (statut, r) = http!(
        app,
        "post",
        PROPOSER,
        Some(awa),
        proposition("Contact group 10", None)
    );
    assert_eq!(
        (statut, r["code"].as_str()),
        (
            StatusCode::TOO_MANY_REQUESTS,
            Some("NEGOTIATION_PROPOSAL_LIMIT")
        )
    );

    let experte = expert(&bac, "experte@example.org").await;
    let (_, file) = http!(
        app,
        "get",
        "/admin/negotiation/queue?kind=proposals",
        Some(experte)
    );
    let un = file["proposals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["term"] == "Contact group 1")
        .expect("proposée");
    let proches = un["nearby"].as_array().unwrap();
    assert!(proches.len() <= 5);
    assert!(proches.iter().any(|p| p["id"] == json!(proche)));
    assert!(proches
        .iter()
        .all(|p| p["similarity"].as_f64().unwrap() >= 0.4));
    assert!(proches
        .windows(2)
        .all(|w| w[0]["similarity"].as_f64() >= w[1]["similarity"].as_f64()));
}

#[tokio::test]
async fn accepter_donne_un_brouillon_et_refuser_demande_un_motif() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let awa = personne(&bac, "awa@example.org").await;
    let app = application!(bac);
    let (_, a) = http!(
        app,
        "post",
        PROPOSER,
        Some(awa),
        proposition("Bracketed text", None)
    );
    let (_, b) = http!(
        app,
        "post",
        PROPOSER,
        Some(awa),
        proposition("Friends of the chair", None)
    );
    let accepter = format!(
        "/admin/negotiation/queue/proposals/{}/accept",
        a["id"].as_str().unwrap()
    );
    let rejeter = |r: &Value| {
        format!(
            "/admin/negotiation/queue/proposals/{}/reject",
            r["id"].as_str().unwrap()
        )
    };
    let fiche = json!({
        "family_code": "meetings",
        "translation": { "fr": "Texte entre crochets" },
        "definition": { "fr": "Un passage sur lequel aucun accord n'est encore fait." },
    });

    let (statut, entree) = http!(app, "post", accepter, Some(experte), fiche.clone());
    assert_eq!(statut, StatusCode::OK, "{entree}");
    assert_eq!(entree["status"], "draft");
    assert_eq!(entree["term"], "Bracketed text");
    assert_eq!(entree["slug"], "bracketed-text");
    let (statut, r) = http!(app, "post", accepter, Some(experte), fiche);
    assert_eq!(
        (statut, r["code"].as_str()),
        (StatusCode::CONFLICT, Some("NEGOTIATION_QUEUE_ITEM_CLOSED"))
    );
    let (_, acceptee) = http!(
        app,
        "get",
        format!(
            "/admin/negotiation/queue/proposals/{}",
            a["id"].as_str().unwrap()
        ),
        Some(experte)
    );
    assert_eq!(acceptee["status"], "accepted");
    assert_eq!(acceptee["glossary_entry_id"], entree["id"]);
    assert_eq!(acceptee["glossary_entry_slug"], "bracketed-text");
    assert!(acceptee["handled_at"].is_string());

    let (statut, r) = http!(
        app,
        "post",
        rejeter(&b),
        Some(experte),
        json!({ "reason": " " })
    );
    assert_eq!(
        (statut, r["field"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("reason"))
    );
    let (statut, refusee) = http!(
        app,
        "post",
        rejeter(&b),
        Some(experte),
        json!({ "reason": "  Hors du champ des négociations.  " })
    );
    assert_eq!(statut, StatusCode::OK, "{refusee}");
    assert_eq!(refusee["status"], "rejected");
    assert_eq!(
        refusee["rejection_reason"],
        "Hors du champ des négociations."
    );
    let (statut, r) = http!(
        app,
        "post",
        rejeter(&b),
        Some(experte),
        json!({ "reason": "Encore." })
    );
    assert_eq!(
        (statut, r["code"].as_str()),
        (StatusCode::CONFLICT, Some("NEGOTIATION_QUEUE_ITEM_CLOSED"))
    );
    let (_, file) = http!(
        app,
        "get",
        "/admin/negotiation/queue?kind=proposals",
        Some(experte)
    );
    assert_eq!(file["proposals"], json!([]), "plus rien n'attend");
    assert_eq!(courriels(&bac).await, 0, "aucun courriel à l'acceptation");
}

#[tokio::test]
async fn publier_lentree_nee_previent_chaque_auteur_une_fois() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let awa = personne(&bac, "awa@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let fatou = personne(&bac, "fatou@example.org").await;
    let app = application!(bac);
    let (_, a) = http!(
        app,
        "post",
        PROPOSER,
        Some(awa),
        proposition("Bracketed text", None)
    );
    http!(
        app,
        "post",
        PROPOSER,
        Some(moussa),
        proposition("bracketed text", None)
    );
    let (statut, entree) = http!(
        app,
        "post",
        format!(
            "/admin/negotiation/queue/proposals/{}/accept",
            a["id"].as_str().unwrap()
        ),
        Some(experte),
        json!({
            "family_code": "meetings",
            "translation": { "fr": "Texte entre crochets" },
            "definition": { "fr": "Un passage encore discuté." },
        })
    );
    assert_eq!(statut, StatusCode::OK, "{entree}");
    let entree_id: Uuid = serde_json::from_value(entree["id"].clone()).unwrap();

    // Le brouillon n'est pas au lexique : un troisième auteur rejoint la proposition acceptée.
    let (statut, r) = http!(
        app,
        "post",
        PROPOSER,
        Some(fatou),
        proposition("Bracketed Text", Some("En groupe de contact."))
    );
    assert_eq!(statut, StatusCode::CREATED, "{r}");
    assert_eq!(r["id"], a["id"]);

    let ctx = bac.ctx(experte);
    changer_terme(&bac.state, &ctx, entree_id, Transition::Publier)
        .await
        .expect("publication");
    assert_eq!(courriels(&bac).await, 3, "un courriel par auteur");
    changer_terme(&bac.state, &ctx, entree_id, Transition::Depublier)
        .await
        .expect("dépublication");
    changer_terme(&bac.state, &ctx, entree_id, Transition::Publier)
        .await
        .expect("republication");
    assert_eq!(
        courriels(&bac).await,
        3,
        "une republication ne renvoie rien"
    );

    let (statut, r) = http!(
        app,
        "post",
        PROPOSER,
        Some(fatou),
        proposition("bracketed text", None)
    );
    assert_eq!(
        (statut, r["slug"].as_str()),
        (StatusCode::CONFLICT, Some("bracketed-text"))
    );

    let boite = Arc::new(Boite::default());
    let gestionnaires = negotiation::job_handlers(bac.db(), &bac.config, boite.clone());
    let gestionnaire = gestionnaires
        .iter()
        .find(|g| g.task() == SEND_PUBLISHED_EMAIL)
        .expect("gestionnaire enregistré");
    let mut tx = bac.db().write(&bac.ctx_anonyme()).await.unwrap();
    let travaux = kernel::jobs::claim(&mut tx, gestionnaire.queue(), "test-worker", 10)
        .await
        .expect("réservation dans la file écoutée");
    tx.commit().await.unwrap();
    let travail = travaux
        .iter()
        .find(|t| t.task == SEND_PUBLISHED_EMAIL)
        .expect("posé dans la file que le gestionnaire écoute");
    gestionnaire.run(travail).await.expect("envoi");
    let remis = boite.0.lock().unwrap().clone();
    assert_eq!(remis.len(), 1);
    assert_eq!(
        remis[0].subject,
        "Le terme que vous avez proposé est au lexique"
    );
    assert!(remis[0].text.contains("« Bracketed text »"));
    assert!(remis[0].text.contains("/guide-nego/lexique/bracketed-text"));
    assert!(!remis[0].text.contains("/fr/guide-nego"));
}
