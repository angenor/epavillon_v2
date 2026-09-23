//! **Chaque garde du back-office des documents laisse passer qui détient son
//! geste, et lui seul** — `contracts/api-admin-documents.md`, « Qui peut quoi ».
//!
//! `perimetre_url_forgee.rs` montre que tout le monde est refusé ; un refus
//! universel y passerait aussi. Ici, chaque garde est éprouvée dans les deux
//! sens, en HTTP : publier, poser une note, la retirer, lire. L'administratrice
//! publie mais ne pose pas de note ; l'experte pose et retire, et ne publie pas.
//!
//! L'experte tient les deux gestes des notes : deux rôles d'essai, d'un seul
//! geste chacun, font voir une interversion des permissions de poser et de
//! retirer, que ni elle ni l'administratrice ne trahiraient.

mod commun;

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::{Method, StatusCode};
// `actix_web::test` s'importe par ses fonctions : le module entier masquerait `#[test]`.
use actix_web::test::{call_service, init_service, read_body, TestRequest};
use actix_web::{web, App, HttpMessage as _};
use commun::documents::{administratrice, expert};
use commun::{attribuer, personne, traces, Bac};
use kernel::auth::Scope;
use kernel::context::RequestContext;
use negotiation::domain::permissions::{CORRECTION_POST, CORRECTION_WITHDRAW, DOCUMENT_PUBLISH};
use serde_json::{json, Value};
use uuid::Uuid;

/// L'en-tête qui tient lieu de session : l'application d'essai n'a pas
/// l'intergiciel de `api`, seulement ce qu'il pose — le contexte et son acteur.
const ACTEUR: &str = "x-essai-acteur";

/// Le back-office du module, monté seul dans une application d'essai.
macro_rules! back_office {
    ($bac:expr) => {
        init_service(
            App::new()
                .app_data(web::Data::new($bac.db()))
                .app_data(web::Data::new($bac.state.clone()))
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
                .configure(negotiation::admin_routes),
        )
        .await
    };
}

/// Une page posée à la main : une note n'a besoin que de son ancre, et PDFium
/// ne se lie qu'une fois par binaire de test.
async fn une_page(bac: &Bac, document: Uuid) {
    sqlx::query(
        "INSERT INTO negotiation.document_pages (document_id, page_index, label)
         VALUES ($1, 1, '1')",
    )
    .bind(document)
    .execute(bac.pool())
    .await
    .expect("page du document");
}

fn appel(verbe: &str, uri: &str, acteur: Uuid, corps: Option<Value>) -> TestRequest {
    let methode = Method::from_bytes(verbe.to_uppercase().as_bytes()).expect("verbe HTTP");
    let mut r = TestRequest::default()
        .method(methode)
        .uri(uri)
        .insert_header((ACTEUR, acteur.to_string()));
    if let Some(c) = corps {
        r = r.set_json(c);
    }
    r
}

async fn frapper<S, R, B>(app: &S, requete: R) -> (StatusCode, Value)
where
    S: Service<R, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let reponse = call_service(app, requete).await;
    let statut = reponse.status();
    let octets = read_body(reponse).await;
    (
        statut,
        serde_json::from_slice(&octets).unwrap_or(Value::Null),
    )
}

/// Ce qu'une écriture laisserait : documents, publiés, notes, travaux, traces.
async fn ecritures(bac: &Bac) -> (i64, i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM negotiation.documents),
                (SELECT count(*) FROM negotiation.documents WHERE published_at IS NOT NULL),
                (SELECT count(*) FROM negotiation.correction_notes),
                (SELECT count(*) FROM platform.jobs),
                (SELECT count(*) FROM platform.audit_log)",
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture des compteurs")
}

async fn peut(bac: &Bac, qui: Uuid, permission: &str) -> bool {
    kernel::auth::has_permission(bac.pool(), qui, permission, Scope::Global)
        .await
        .expect("lecture de la permission")
}

/// Une personne dont le rôle d'essai ne porte qu'un geste, sur la portée globale.
async fn un_seul_geste(bac: &Bac, email: &str, role: &str, permission: &str) -> Uuid {
    sqlx::query(
        "INSERT INTO identity.roles (code, label) VALUES ($1, jsonb_build_object('fr', $1::text))",
    )
    .bind(role)
    .execute(bac.pool())
    .await
    .expect("rôle d'essai");
    sqlx::query(
        "INSERT INTO identity.role_permissions (role_code, permission_code) VALUES ($1, $2)",
    )
    .bind(role)
    .bind(permission)
    .execute(bac.pool())
    .await
    .expect("permission du rôle d'essai");
    let p = personne(bac, email).await;
    attribuer(bac, p, role, "global", None).await;
    p
}

async fn retiree_par(bac: &Bac, note: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("SELECT withdrawn_by FROM negotiation.correction_notes WHERE id = $1")
        .bind(note)
        .fetch_one(bac.pool())
        .await
        .expect("relecture de la note")
}

#[tokio::test]
async fn chaque_garde_decriture_laisse_passer_qui_detient_son_geste_et_lui_seul() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let relectrice = expert(&bac, "experte@example.org").await;
    let pose_seule = un_seul_geste(&bac, "pose@example.org", "essai_pose", CORRECTION_POST).await;
    let retrait_seul = un_seul_geste(
        &bac,
        "retrait@example.org",
        "essai_retrait",
        CORRECTION_WITHDRAW,
    )
    .await;
    for (qui, attendus) in [
        (ifdd, [true, false, false]),
        (relectrice, [false, true, true]),
        (pose_seule, [false, true, false]),
        (retrait_seul, [false, false, true]),
    ] {
        for (permission, attendu) in [DOCUMENT_PUBLISH, CORRECTION_POST, CORRECTION_WITHDRAW]
            .into_iter()
            .zip(attendus)
        {
            assert_eq!(
                peut(&bac, qui, permission).await,
                attendu,
                "{qui} : {permission}"
            );
        }
    }
    let app = back_office!(bac);
    let refus = (StatusCode::FORBIDDEN, Some("FORBIDDEN"));

    // Publier : l'administratrice de la plateforme, elle seule.
    let creation = "/admin/negotiation/documents";
    let titre = json!({ "title": { "fr": "Note technique de Bonn" }, "type": "negotiation_guide" });
    let avant = ecritures(&bac).await;
    for qui in [relectrice, pose_seule, retrait_seul] {
        let (statut, corps) = frapper(
            &app,
            appel("post", creation, qui, Some(titre.clone())).to_request(),
        )
        .await;
        assert_eq!(
            (statut, corps["code"].as_str()),
            refus,
            "{qui} ne publie pas"
        );
    }
    assert_eq!(ecritures(&bac).await, avant, "aucun refus n'a écrit");
    let (statut, cree) = frapper(
        &app,
        appel("post", creation, ifdd, Some(titre)).to_request(),
    )
    .await;
    assert_eq!(statut, StatusCode::CREATED, "{cree}");
    let document: Uuid = serde_json::from_value(cree["id"].clone()).expect("identifiant");
    assert_eq!(
        (
            cree["state"].as_str(),
            cree["title"]["fr"].as_str(),
            cree["can_publish"].as_bool(),
            cree["can_correct"].as_bool()
        ),
        (
            Some("draft"),
            Some("Note technique de Bonn"),
            Some(true),
            Some(false)
        )
    );
    assert_eq!(
        traces(&bac, "documents", document).await,
        [("insert".to_owned(), Some(ifdd))],
        "le brouillon porte son auteur"
    );

    // Poser une note : qui corrige ; ni l'administratrice, ni qui ne sait que
    // retirer. L'administrateur publie, mais ne pose pas de note.
    une_page(&bac, document).await;
    let notes_du_document = format!("/admin/negotiation/documents/{document}/corrections");
    let note =
        json!({ "page_index": 1, "body": { "fr": "Chiffre dépassé.", "en": "Outdated figure." } });
    let avant = ecritures(&bac).await;
    for qui in [ifdd, retrait_seul] {
        let (statut, corps) = frapper(
            &app,
            appel("post", &notes_du_document, qui, Some(note.clone())).to_request(),
        )
        .await;
        assert_eq!(
            (statut, corps["code"].as_str()),
            refus,
            "{qui} ne pose pas de note"
        );
    }
    assert_eq!(ecritures(&bac).await, avant, "aucun refus n'a écrit");
    let mut posees = Vec::new();
    for auteur in [relectrice, pose_seule] {
        let (statut, posee) = frapper(
            &app,
            appel("post", &notes_du_document, auteur, Some(note.clone())).to_request(),
        )
        .await;
        assert_eq!(statut, StatusCode::CREATED, "{posee}");
        assert_eq!(
            (
                &posee["author"]["id"],
                &posee["document_id"],
                posee["page_index"].as_i64(),
                &posee["withdrawn_at"]
            ),
            (&json!(auteur), &json!(document), Some(1), &Value::Null)
        );
        let id: Uuid = serde_json::from_value(posee["id"].clone()).expect("identifiant");
        assert_eq!(
            traces(&bac, "correction_notes", id).await,
            [("insert".to_owned(), Some(auteur))]
        );
        posees.push(id);
    }

    // Retirer : qui retire ; ni l'administratrice, ni qui ne sait que poser.
    let retirer = |note: Uuid| format!("/admin/negotiation/corrections/{note}/withdraw");
    let avant = ecritures(&bac).await;
    for qui in [ifdd, pose_seule] {
        let (statut, corps) = frapper(
            &app,
            appel("post", &retirer(posees[0]), qui, None).to_request(),
        )
        .await;
        assert_eq!(
            (statut, corps["code"].as_str()),
            refus,
            "{qui} ne retire pas"
        );
    }
    assert_eq!(ecritures(&bac).await, avant, "aucun refus n'a écrit");
    assert_eq!(retiree_par(&bac, posees[0]).await, None);
    for (note, qui) in [(posees[0], relectrice), (posees[1], retrait_seul)] {
        let (statut, retiree) =
            frapper(&app, appel("post", &retirer(note), qui, None).to_request()).await;
        assert_eq!(statut, StatusCode::OK, "{retiree}");
        assert_eq!(retiree["withdrawn_by"]["id"], json!(qui));
        assert!(retiree["withdrawn_at"].is_string(), "{retiree}");
        assert_eq!(retiree_par(&bac, note).await, Some(qui));
    }

    // Lire : publier ou poser l'ouvre ; retirer seul, non.
    let (statut, liste) =
        frapper(&app, appel("get", creation, relectrice, None).to_request()).await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(
        (
            liste["can_publish"].as_bool(),
            liste["can_correct"].as_bool()
        ),
        (Some(false), Some(true))
    );
    assert!(liste["documents"]
        .as_array()
        .expect("liste des documents")
        .iter()
        .any(|d| d["id"] == json!(document)));
    let (statut, corps) = frapper(
        &app,
        appel("get", creation, retrait_seul, None).to_request(),
    )
    .await;
    assert_eq!((statut, corps["code"].as_str()), refus);

    // Le droit de retirer, que la liste des notes annonce, se lit sur la
    // portée globale et suit la permission, pas le rôle.
    for (qui, peut_poser, peut_retirer) in [
        (relectrice, true, true),
        (pose_seule, true, false),
        (ifdd, false, false),
    ] {
        let (statut, notes) = frapper(
            &app,
            appel("get", &notes_du_document, qui, None).to_request(),
        )
        .await;
        assert_eq!(statut, StatusCode::OK);
        assert_eq!(
            (
                notes["can_post"].as_bool(),
                notes["can_withdraw"].as_bool(),
                notes["notes"].as_array().map(Vec::len)
            ),
            (Some(peut_poser), Some(peut_retirer), Some(2)),
            "{qui}"
        );
    }
}
