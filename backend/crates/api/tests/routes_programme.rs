//! **Les routes du module Propositions sont réellement atteignables.**
//!
//! Même raison qu'en B2 et B3 : un test qui appelle un service n'éprouve pas le
//! montage. Ici, deux préfixes sont **partagés** — `/people` avec l'identité et
//! les organisations, `/organizations` avec les organisations —, et deux
//! `web::scope` du même préfixe **ne se complètent pas** : Actix retient le
//! premier et rend 404 sur les routes du second, sans essayer.
//!
//! Chaque route est donc frappée sur la **vraie application**, intergiciels
//! compris. On ne vérifie qu'une chose : **le chemin existe**. Le comportement,
//! lui, est éprouvé par les tests du module.
//!
//! Le compte est écrit : une route ajoutée sans être montée fait échouer ce
//! test.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;
use uuid::Uuid;

fn id() -> String {
    Uuid::now_v7().to_string()
}

/// **Les trente-sept routes du contrat, toutes montées.**
///
/// Le compte est écrit : une route ajoutée sans être montée fait échouer ce
/// test, et une route retirée du contrat aussi.
fn chemins() -> Vec<(&'static str, String)> {
    let dossier = id();
    let edition = id();
    let organisation = id();
    let message = id();
    let piece = id();

    vec![
        // US1 — le dépôt
        ("GET", "/api/proposals/form-context".to_owned()),
        ("GET", "/api/proposals/draft".to_owned()),
        ("POST", "/api/proposals".to_owned()),
        ("PUT", format!("/api/proposals/{dossier}")),
        ("POST", format!("/api/proposals/{dossier}/submit")),
        // US6 — corriger et renvoyer
        ("GET", format!("/api/proposals/{dossier}/draft")),
        ("POST", format!("/api/proposals/{dossier}/resubmit")),
        ("GET", "/api/people/lookup?email=a@example.org".to_owned()),
        // US2 — la machine à états
        ("GET", "/api/proposals/transitions".to_owned()),
        ("POST", "/api/proposals/status".to_owned()),
        ("GET", format!("/api/proposals/{dossier}/transitions")),
        (
            "GET",
            format!("/api/proposals/{dossier}/available-transitions"),
        ),
        // US3 — la liste du comité
        ("GET", format!("/api/proposals/list?event_id={edition}")),
        (
            "GET",
            format!("/api/proposals/dashboard?event_id={edition}"),
        ),
        (
            "GET",
            format!("/api/proposals/committee?event_id={edition}"),
        ),
        ("POST", "/api/proposals/assignments".to_owned()),
        (
            "GET",
            format!("/api/proposals?organization_id={organisation}"),
        ),
        ("GET", format!("/api/proposals/{dossier}")),
        // US4 — la fiche d'évaluation
        ("GET", format!("/api/proposals/{dossier}/review-desk")),
        ("PUT", format!("/api/proposals/{dossier}/reviews")),
        ("POST", format!("/api/proposals/{dossier}/recusal")),
        ("POST", format!("/api/proposals/{dossier}/decision")),
        // US5 — l'espace organisation. Les deux premières vivent sous un
        // préfixe PARTAGÉ avec le module Organisations : si elles répondaient
        // 404, c'est que les deux scopes ne se sont pas complétés.
        (
            "GET",
            format!("/api/organizations/{organisation}/workspace"),
        ),
        ("GET", format!("/api/organizations/{organisation}/editions")),
        ("GET", format!("/api/proposals/{dossier}/file")),
        ("POST", format!("/api/proposals/{dossier}/comments")),
        (
            "POST",
            format!("/api/proposal-comments/{message}/resolution"),
        ),
        (
            "DELETE",
            format!("/api/proposal-comments/{message}/resolution"),
        ),
        // US7 — les pièces du dossier
        ("GET", format!("/api/proposals/{dossier}/documents")),
        ("POST", format!("/api/proposals/{dossier}/documents")),
        (
            "DELETE",
            format!("/api/proposals/{dossier}/documents/{piece}"),
        ),
        // US8 — le détail, l'historique et la reprise v1
        ("GET", format!("/api/proposals/{dossier}/organizations")),
        ("GET", format!("/api/proposals/{dossier}/speakers")),
        ("GET", format!("/api/proposals/{dossier}/themes")),
        ("GET", format!("/api/proposals/{dossier}/history")),
        ("GET", format!("/api/proposals/{dossier}/comments")),
        (
            "POST",
            "/api/admin/proposals/transitions-backfill".to_owned(),
        ),
    ]
}

#[actix_web::test]
async fn les_routes_du_depot_sont_montees() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let routes = chemins();
    assert_eq!(routes.len(), 37, "les trente-sept routes du contrat");

    for (verbe, chemin) in routes {
        let requete = match verbe {
            "GET" => test::TestRequest::get(),
            "POST" => test::TestRequest::post(),
            "PUT" => test::TestRequest::put(),
            "DELETE" => test::TestRequest::delete(),
            autre => panic!("verbe inattendu : {autre}"),
        };

        let reponse = test::call_service(&app, requete.uri(&chemin).to_request()).await;

        // **Aucune session** : toutes répondent 401. Ce qu'aucune ne doit
        // répondre, c'est **404** — ce serait le signe qu'elle n'est pas montée.
        assert_ne!(
            reponse.status(),
            StatusCode::NOT_FOUND,
            "{verbe} {chemin} n'est pas montée — deux scopes du même préfixe ne se \
             complètent pas, et le second est muet"
        );
        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} a répondu {} — on attendait un refus d'authentification",
            reponse.status()
        );
    }
}

/// **Les chemins littéraux ne sont pas capturés par le chemin paramétré.**
///
/// Actix retient la première ressource dont le motif correspond. **Mesuré
/// plutôt que supposé** : quand la méthode ne correspond à aucune route de
/// cette ressource, il ne rend pas 405 — il poursuit, et un chemin non servi
/// finit sur la route par défaut de l'API, donc en **404**.
///
/// C'est donc 404 que ce test guette, et c'est ce qui le rend discriminant dès
/// aujourd'hui : `/proposals/{id}` n'est servi qu'en `PUT`, si bien qu'un `GET
/// /proposals/transitions` capturé par lui rendrait 404 et non 401.
///
/// **Le risque n'existe pour l'instant que là**, les méthodes ne se recouvrant
/// pas. Il deviendra réel à US4, quand `GET /proposals/{id}` arrivera : les six
/// chemins littéraux entreront alors en concurrence avec lui sur la même
/// méthode, et seul l'ordre d'enregistrement les sauvera.
#[actix_web::test]
async fn les_chemins_litteraux_precedent_le_chemin_parametre() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    for (verbe, chemin) in [
        ("GET", "/api/proposals/form-context"),
        ("GET", "/api/proposals/draft"),
        ("GET", "/api/proposals/transitions"),
        ("POST", "/api/proposals/status"),
        // Depuis US3, ces quatre-là sont en concurrence RÉELLE avec
        // `/proposals/{id}` : même méthode, un seul segment. C'est le cas que
        // `lib.rs` annonçait, et il est désormais couvert.
        (
            "GET",
            "/api/proposals/list?event_id=00000000-0000-0000-0000-000000000000",
        ),
        (
            "GET",
            "/api/proposals/dashboard?event_id=00000000-0000-0000-0000-000000000000",
        ),
        (
            "GET",
            "/api/proposals/committee?event_id=00000000-0000-0000-0000-000000000000",
        ),
        ("POST", "/api/proposals/assignments"),
    ] {
        let requete = match verbe {
            "GET" => test::TestRequest::get(),
            _ => test::TestRequest::post(),
        };
        let reponse = test::call_service(&app, requete.uri(chemin).to_request()).await;

        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} a répondu {} — un 404 signifierait qu'il est capturé par \
             /proposals/{{id}} et n'atteint jamais son gestionnaire",
            reponse.status()
        );
    }
}

/// **Les routes des deux autres modules sous `/people` survivent.** C'est
/// l'autre moitié du piège : composer un scope aurait pu masquer les
/// premières.
#[actix_web::test]
async fn les_routes_des_autres_modules_sous_people_survivent() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let personne = id();
    for chemin in [
        format!("/api/people/{personne}"),
        format!("/api/people/{personne}/memberships"),
    ] {
        let reponse =
            test::call_service(&app, test::TestRequest::get().uri(&chemin).to_request()).await;
        assert_ne!(
            reponse.status(),
            StatusCode::NOT_FOUND,
            "{chemin} est devenue muette après l'ajout de /people/lookup"
        );
    }
}

/// **Ce module n'enregistre AUCUN travail différé, et c'est une décision.**
///
/// C'est le premier module du jalon dans ce cas (R20), et un fait à vérifier
/// plutôt qu'un oubli à constater : rien ici n'a d'effet à échéance. Les
/// rappels de revue et les avis de dépôt appartiennent à B6 et se
/// déclencheront sur les événements du service ; la clôture d'un appel échu
/// appartient à B3 et y est livrée. La déduction des transitions v1, elle, est
/// **synchrone** — son résultat doit être lu par celui qui la lance, pas
/// remplacé par un identifiant de tâche.
///
/// Le contrôle porte sur le crate, pas sur une intention : si `programme`
/// exposait un jour `job_handlers()`, ce test ne compilerait plus, et c'est ce
/// qu'on veut — la décision se rediscuterait au lieu de se contourner.
// `use actix_web::test` masque l'attribut `#[test]` de la bibliothèque
// standard dans ce fichier : on emploie donc celui d'Actix, comme les autres.
#[actix_web::test]
async fn le_module_propositions_nenregistre_aucun_travail_differe() {
    // `identity`, `org` et `event` exposent tous `job_handlers()` ; celui-ci
    // n'en a pas, et le worker n'est donc pas modifié.
    let symboles = include_str!("../../modules/programme/src/lib.rs");
    assert!(
        !symboles.contains("pub fn job_handlers"),
        "ce module ne déclare aucun travail différé — voir research.md § R20"
    );
}
