//! **La file des experts, parts « signalements » et « questions »**, en HTTP
//! sur base réelle : groupée par entrée, le plus ancien d'abord ; close avec son
//! issue sans toucher l'entrée (FR-018) ; et aucun auteur dans aucune réponse,
//! fiche FAQ du back-office et brouillon promu compris (SC-009).

mod commun;

use actix_web::http::StatusCode;
use commun::documents::{administratrice, expert, negociatrice};
use commun::http::{appel, frapper};
use commun::savoir::entree_faq;
use commun::{personne, Bac};
use negotiation::domain::savoir_questions::MyQuestionInput;
use negotiation::domain::savoir_retours::{FaqFeedbackInput, FaqReportInput};
use negotiation::service::savoir_questions::poser;
use negotiation::service::savoir_retours::{signaler, voter};
use serde_json::{json, Value};
use uuid::Uuid;

macro_rules! http {
    ($app:expr, $verbe:expr, $uri:expr, $qui:expr) => {
        frapper(&$app, appel($verbe, &$uri, Some($qui), None).to_request()).await
    };
    ($app:expr, $verbe:expr, $uri:expr, $qui:expr, $corps:expr) => {
        frapper(
            &$app,
            appel($verbe, &$uri, Some($qui), Some($corps)).to_request(),
        )
        .await
    };
}

async fn signaleur(bac: &Bac, email: &str, prenom: &str, nom: &str) -> Uuid {
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

async fn signaler_(bac: &Bac, qui: Uuid, entree: Uuid, motifs: &[&str], details: Option<&str>) {
    let s: FaqReportInput = serde_json::from_value(
        json!({ "client_ref": Uuid::now_v7(), "reasons": motifs, "details": details }),
    )
    .unwrap();
    signaler(&bac.state, &bac.ctx(qui), qui, entree, &s)
        .await
        .expect("signalement");
}

async fn ligne(bac: &Bac, id: Uuid) -> String {
    sqlx::query_scalar("SELECT row_to_json(f)::text FROM negotiation.faq_entries f WHERE id = $1")
        .bind(id)
        .fetch_one(bac.pool())
        .await
        .expect("ligne de l'entrée")
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
async fn la_file_groupe_par_entree_et_ne_rend_aucun_auteur() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let moussa = signaleur(&bac, "moussa.signaleur@example.org", "Moussa", "Signaleur").await;
    let awa = signaleur(&bac, "awa.temoin@example.org", "Awatemoin", "Kaboreh").await;
    let premiere = entree_faq(
        &bac,
        experte,
        "Qu'est-ce qu'un groupe de contact ?",
        "published",
    )
    .await;
    let seconde = entree_faq(&bac, experte, "Qu'est-ce qu'un document L ?", "to_review").await;

    signaler_(
        &bac,
        moussa,
        premiere,
        &["rule_changed"],
        Some("Décision 1/CMA.6"),
    )
    .await;
    signaler_(&bac, awa, seconde, &["wrong", "source_mismatch"], None).await;
    let depassee: FaqFeedbackInput =
        serde_json::from_value(json!({ "helpful": false, "missing_reason": "outdated" })).unwrap();
    voter(&bac.state, &bac.ctx(awa), awa, premiere, &depassee)
        .await
        .expect("retour");

    let app = crate::back_office!(bac);
    let (statut, file) = http!(app, "get", "/admin/negotiation/queue?kind=reports", experte);
    assert_eq!(statut, StatusCode::OK, "{file}");
    assert_eq!(file["kind"], "reports");
    assert_eq!(file["counts"]["reports"], 3);
    let groupes = file["reports"].as_array().expect("groupes");
    assert_eq!(groupes.len(), 2);
    assert_eq!(
        groupes[0]["entry"]["id"],
        json!(premiere),
        "le plus ancien d'abord"
    );
    assert_eq!(groupes[0]["reports"].as_array().unwrap().len(), 2);
    assert_eq!(groupes[0]["reports"][1]["from_feedback"], true);
    assert_eq!(groupes[0]["feedback"]["outdated"], 1);
    assert_eq!(groupes[1]["entry"]["status"], "to_review");
    assert_eq!(
        groupes[1]["reports"][0]["reasons"],
        json!(["wrong", "source_mismatch"])
    );

    let traces: Vec<String> = [moussa, awa]
        .iter()
        .map(Uuid::to_string)
        .chain(
            [
                "Moussa",
                "Signaleur",
                "Awatemoin",
                "Kaboreh",
                "moussa.signaleur",
                "awa.temoin",
            ]
            .map(str::to_owned),
        )
        .collect();
    sans_trace(&file, &traces, "la file");
    let (_, sans_kind) = http!(app, "get", "/admin/negotiation/queue", experte);
    sans_trace(&sans_kind, &traces, "la file sans sorte");

    let signalement = groupes[0]["reports"][0]["id"].as_str().unwrap().to_owned();
    let (statut, clos) = http!(
        app,
        "post",
        format!("/admin/negotiation/queue/reports/{signalement}/close"),
        experte,
        json!({ "outcome": "confirmed" })
    );
    assert_eq!(statut, StatusCode::OK, "{clos}");
    assert_eq!(
        (clos["status"].as_str(), clos["outcome"].as_str()),
        (Some("closed"), Some("confirmed"))
    );
    sans_trace(&clos, &traces, "la clôture");
    sans_trace(&clos, &[experte.to_string()], "la clôture (expert)");

    for entree in [premiere, seconde] {
        let (statut, fiche) = http!(
            app,
            "get",
            format!("/admin/negotiation/faq/{entree}"),
            experte
        );
        assert_eq!(statut, StatusCode::OK);
        assert!(!fiche["reports"].as_array().unwrap().is_empty());
        sans_trace(&fiche, &traces, "la fiche FAQ du back-office");
    }
    let (_, liste) = http!(app, "get", "/admin/negotiation/faq", experte);
    sans_trace(&liste, &traces, "la liste de la FAQ");
}

#[tokio::test]
async fn clore_laisse_lentree_identique_quelle_que_soit_lissue() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let faq = entree_faq(&bac, experte, "Qui préside ?", "published").await;
    for motif in ["rule_changed", "wrong", "source_mismatch"] {
        signaler_(&bac, moussa, faq, &[motif], None).await;
    }
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM negotiation.faq_reports WHERE entry_id = $1 ORDER BY created_at, id",
    )
    .bind(faq)
    .fetch_all(bac.pool())
    .await
    .unwrap();
    let avant = ligne(&bac, faq).await;
    let app = crate::back_office!(bac);

    for (id, issue) in ids.iter().zip(["revised", "confirmed", "dismissed"]) {
        let uri = format!("/admin/negotiation/queue/reports/{id}/close");
        let (statut, clos) = http!(app, "post", uri, experte, json!({ "outcome": issue }));
        assert_eq!(statut, StatusCode::OK, "{clos}");
        assert_eq!(clos["outcome"], issue);
        assert_eq!(
            ligne(&bac, faq).await,
            avant,
            "{issue} : l'entrée n'a pas bougé, updated_at compris"
        );

        let (statut, deux) = http!(app, "post", uri, experte, json!({ "outcome": issue }));
        assert_eq!(
            (statut, deux["code"].as_str()),
            (StatusCode::CONFLICT, Some("NEGOTIATION_QUEUE_ITEM_CLOSED"))
        );
    }
    let (_, file) = http!(app, "get", "/admin/negotiation/queue", experte);
    assert_eq!(file["counts"]["reports"], 0);
    assert!(file["reports"].as_array().unwrap().is_empty());

    let traite: (Option<Uuid>, bool) = sqlx::query_as(
        "SELECT handled_by, handled_at IS NOT NULL FROM negotiation.faq_reports WHERE id = $1",
    )
    .bind(ids[0])
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(traite, (Some(experte), true), "gardé en base, jamais rendu");
}

#[tokio::test]
async fn la_file_refuse_ce_quelle_ne_sait_pas_faire_et_qui_ne_verifie_pas() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let faq = entree_faq(&bac, experte, "Qui préside ?", "published").await;
    signaler_(&bac, moussa, faq, &["wrong"], None).await;
    let id: Uuid = sqlx::query_scalar("SELECT id FROM negotiation.faq_reports")
        .fetch_one(bac.pool())
        .await
        .unwrap();
    let app = crate::back_office!(bac);
    let clore = format!("/admin/negotiation/queue/reports/{id}/close");

    let (statut, _) = http!(
        app,
        "get",
        "/admin/negotiation/queue?kind=everything",
        experte
    );
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
    let (statut, _) = http!(app, "post", clore, experte, json!({ "outcome": "deleted" }));
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
    let (statut, _) = http!(
        app,
        "post",
        format!("/admin/negotiation/queue/reports/{}/close", Uuid::now_v7()),
        experte,
        json!({ "outcome": "dismissed" })
    );
    assert_eq!(statut, StatusCode::NOT_FOUND);

    // L'administratrice publie, elle ne vérifie pas : la file lui est fermée.
    let (statut, _) = http!(app, "get", "/admin/negotiation/queue", ifdd);
    assert_eq!(statut, StatusCode::FORBIDDEN);
    let (statut, _) = http!(app, "post", clore, ifdd, json!({ "outcome": "dismissed" }));
    assert_eq!(statut, StatusCode::FORBIDDEN);
    let ouvert: String = sqlx::query_scalar("SELECT status::text FROM negotiation.faq_reports")
        .fetch_one(bac.pool())
        .await
        .unwrap();
    assert_eq!(ouvert, "open");
}

#[tokio::test]
async fn la_file_des_questions_ne_rend_ni_lauteure_ni_lexpert() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    sqlx::query("UPDATE identity.people SET first_name = 'Koffiexpert', last_name = 'Mensahx' WHERE id = $1")
        .bind(experte)
        .execute(bac.pool())
        .await
        .unwrap();
    let aissatou = negociatrice(&bac, "aissatou.anonyme@example.org").await;
    sqlx::query(
        "UPDATE identity.people SET first_name = 'Aissatoux', last_name = 'Diallox' WHERE id = $1",
    )
    .bind(aissatou)
    .execute(bac.pool())
    .await
    .unwrap();
    let mut ids = Vec::new();
    for (body, consentement) in [("Qui coordonne ?", true), ("Où dormir ?", false)] {
        let entree: MyQuestionInput = serde_json::from_value(json!({
            "client_ref": Uuid::now_v7(), "theme_code": "adaptation",
            "body": body, "consent_to_faq": consentement,
        }))
        .unwrap();
        let (q, _) = poser(&bac.state, &bac.ctx(aissatou), aissatou, &entree, "fr")
            .await
            .expect("question");
        ids.push(q.id);
    }
    let traces: Vec<String> = [aissatou, experte]
        .iter()
        .map(Uuid::to_string)
        .chain(
            [
                "Aissatoux",
                "Diallox",
                "aissatou.anonyme",
                "Koffiexpert",
                "Mensahx",
            ]
            .map(str::to_owned),
        )
        .collect();
    let app = crate::back_office!(bac);

    let (statut, file) = http!(
        app,
        "get",
        "/admin/negotiation/queue?kind=questions",
        experte
    );
    assert_eq!(statut, StatusCode::OK, "{file}");
    assert_eq!(file["kind"], "questions");
    assert_eq!(file["counts"]["questions"], 2);
    let questions = file["questions"].as_array().expect("questions");
    assert_eq!(
        questions[0]["id"],
        json!(ids[0]),
        "la plus ancienne d'abord"
    );
    assert_eq!(questions[1]["consent_to_faq"], false);
    sans_trace(&file, &traces, "la file des questions");

    let (statut, repondue) = http!(
        app,
        "post",
        format!("/admin/negotiation/queue/questions/{}/answer", ids[0]),
        experte,
        json!({ "answer": "Le coordinateur du groupe." })
    );
    assert_eq!(statut, StatusCode::OK);
    sans_trace(&repondue, &traces, "la réponse");

    let (_, file) = http!(
        app,
        "get",
        "/admin/negotiation/queue?kind=questions",
        experte
    );
    assert_eq!(file["counts"]["questions"], 1);
    let dernieres = file["questions"].as_array().unwrap();
    assert_eq!(
        (dernieres[0]["id"].clone(), dernieres[1]["status"].clone()),
        (json!(ids[1]), json!("answered")),
        "l'attente d'abord, puis la répondue à promouvoir"
    );
    sans_trace(&file, &traces, "la file après réponse");

    let (statut, brouillon) = http!(
        app,
        "post",
        format!("/admin/negotiation/queue/questions/{}/promote", ids[0]),
        experte,
        json!({ "section_code": "first_cop" })
    );
    assert_eq!(statut, StatusCode::OK, "{brouillon}");
    sans_trace(&brouillon, &traces, "le brouillon promu");
    let (_, fiche) = http!(
        app,
        "get",
        format!(
            "/admin/negotiation/faq/{}",
            brouillon["id"].as_str().unwrap()
        ),
        experte
    );
    sans_trace(&fiche, &traces, "la fiche du brouillon promu");
}
