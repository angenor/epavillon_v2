//! **Les deux modules de B6 montent dans la vraie application, et le préfixe
//! `/sessions` de B5 n'a rien perdu.**
//!
//! Ce fichier est le **squelette** : il éprouve aujourd'hui le montage et la
//! composition de scope, et il se complète d'une histoire à l'autre jusqu'aux
//! trente-trois routes du contrat.
//!
//! # Ce qu'il attrape, et qu'aucun autre test n'attrape
//!
//! `/sessions` appartenait au module Programmation depuis B5 ; le module
//! Engagement y dépose deux routes depuis B6. **Deux `web::scope` du même
//! préfixe ne se complètent pas** : Actix retient le premier dont le préfixe
//! correspond et rend 404 si la route n'y figure pas, sans essayer le suivant.
//! Le défaut a coûté trois routes sur vingt et une en B2, et il se serait
//! reproduit ici — cette fois en rendant **muettes des routes déjà livrées**.
//!
//! Ni la compilation, ni les tests des modules — qui appellent les services
//! directement — ne le verraient. Seul un test qui traverse HTTP le voit.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::testing::TestDb;
use uuid::Uuid;

fn id() -> String {
    Uuid::now_v7().to_string()
}

/// **Les dix-sept routes de B5 sous `/sessions` répondent toujours.**
///
/// C'est l'assertion qui compte le plus de ce fichier : composer un scope livré
/// est le geste le plus risqué du jalon, et il ne se vérifie que comme cela.
#[actix_web::test]
async fn les_routes_de_seances_de_b5_survivent_a_la_composition() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    // **Les routes gardées seulement.** Les deux routes publiques du préfixe —
    // le formulaire d'inscription et l'inscription elle-même — rendent 404 sur
    // une séance inconnue, ce qui est légitime et indiscernable d'une route non
    // montée : `routes_programme_sessions.rs` les éprouve avec une vraie séance,
    // et c'est là qu'elles doivent l'être.
    let session = id();
    for (verbe, chemin) in [
        (
            "GET",
            "/api/sessions/conflicts?event_id=".to_owned() + &id(),
        ),
        ("GET", format!("/api/sessions/{session}/speakers")),
        ("GET", format!("/api/sessions/{session}/organizations")),
        ("GET", format!("/api/sessions/{session}/tracks")),
        ("PUT", format!("/api/sessions/{session}/schedule")),
        ("PUT", format!("/api/sessions/{session}/tracks")),
        ("PUT", format!("/api/sessions/{session}/broadcast")),
    ] {
        let requete = match verbe {
            "GET" => test::TestRequest::get(),
            _ => test::TestRequest::put(),
        };
        let reponse = test::call_service(&app, requete.uri(&chemin).to_request()).await;

        assert_ne!(
            reponse.status(),
            StatusCode::NOT_FOUND,
            "{verbe} {chemin} est devenue muette : deux scopes du même préfixe ne se \
             complètent pas, et le second est ignoré"
        );
    }
}

/// **Les routes du module Média sont montées, back-office compris.**
///
/// Le compte est écrit et s'allonge à chaque histoire : une route ajoutée sans
/// être montée fait échouer ce test. Ce qu'on vérifie ici est **le chemin**, pas
/// le comportement — celui-ci est éprouvé par les tests du module, qui appellent
/// les services.
///
/// **Les trois routes de back-office sont l'assertion neuve.** Elles vivent sous
/// `/admin`, préfixe que B5 et B6 partagent déjà : montées dans un
/// `web::scope("/admin")` propre au module, elles rendraient muettes celles des
/// autres — le défaut qui a coûté trois routes en B2.
#[actix_web::test]
async fn les_routes_du_module_media_sont_montees() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let asset = id();
    let routes = vec![
        ("POST", "/api/media/assets/precheck".to_owned()),
        ("POST", "/api/media/assets".to_owned()),
        ("GET", format!("/api/media/assets/{asset}")),
        ("GET", format!("/api/media/assets/{asset}/status")),
        ("GET", "/api/admin/media/orphans".to_owned()),
        ("GET", "/api/admin/media/quotas".to_owned()),
        ("PUT", format!("/api/admin/media/quotas/{}", id())),
    ];
    assert_eq!(
        routes.len(),
        7,
        "les trois routes d'US1, l'avancement d'US2, et les trois du back-office d'US9"
    );

    for (verbe, chemin) in routes {
        let requete = match verbe {
            "GET" => test::TestRequest::get(),
            "PUT" => test::TestRequest::put(),
            "DELETE" => test::TestRequest::delete(),
            _ => test::TestRequest::post(),
        };
        let reponse = test::call_service(&app, requete.uri(&chemin).to_request()).await;

        assert_ne!(
            reponse.status(),
            StatusCode::NOT_FOUND,
            "{verbe} {chemin} n'est pas montée"
        );
        // **Aucune session** : toutes exigent un compte, et le refus qu'on
        // attend est celui-là — jamais un 404, qui dirait que la route n'existe
        // pas.
        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} devrait exiger une session"
        );
    }
}

/// **La suppression d'un objet répond, alors qu'elle partage son chemin avec la
/// lecture.**
///
/// `GET /media/assets/{id}` et `DELETE /media/assets/{id}` sont enregistrées par
/// deux appels séparés sur le **même chemin**. Si le premier captait la
/// ressource, le second serait ignoré et la suppression rendrait 405 ou 404 sans
/// que rien ne le signale — ni la compilation, ni les tests du module, qui
/// appellent le service directement.
///
/// Elle n'entre pas dans le compte ci-dessus : ce n'est pas une route de
/// back-office, et c'est le **partage de chemin** qu'on éprouve ici.
#[actix_web::test]
async fn la_suppression_partage_son_chemin_avec_la_lecture_sans_la_masquer() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let chemin = format!("/api/media/assets/{}", id());
    let reponse =
        test::call_service(&app, test::TestRequest::delete().uri(&chemin).to_request()).await;

    assert_eq!(
        reponse.status(),
        StatusCode::UNAUTHORIZED,
        "DELETE {chemin} doit exiger une session — ni 404 ni 405, qui diraient \
         que la lecture a capté la ressource"
    );
}

/// Les préfixes qui ne portent encore aucune route rendent bien le **corps
/// d'erreur du catalogue**, et non la réponse vide d'Actix : le principe IX ne
/// souffre pas d'exception parce que la route n'existe pas encore.
#[actix_web::test]
async fn un_chemin_absent_rend_le_corps_derreur_du_catalogue() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    // Les deux préfixes d'US8 ont rejoint les montés : ce qui reste absent est ce
    // qu'aucune histoire ne sert — la messagerie directe et les infolettres, hors
    // périmètre déclaré du jalon.
    for chemin in ["/api/conversations", "/api/newsletters"] {
        let reponse =
            test::call_service(&app, test::TestRequest::get().uri(chemin).to_request()).await;
        assert_eq!(reponse.status(), StatusCode::NOT_FOUND);

        let corps: serde_json::Value = test::read_body_json(reponse).await;
        assert_eq!(
            corps["code"], "NOT_FOUND",
            "{chemin} doit rendre un corps d'erreur du catalogue"
        );
    }
}

/// **Les dix-neuf routes gardées du module Engagement sont montées.**
///
/// Dix-sept sous des préfixes qui n'appartiennent qu'à lui, et **deux sous
/// `/sessions`**, préfixe livré par B5 et composé ici. Ce sont ces deux-là qui
/// rendent le test indispensable : montées dans un second
/// `web::scope("/sessions")`, elles rendraient 404 sans que rien ne le signale.
///
/// La vingtième route du module — la porte d'ingestion — est **hors session** et
/// son montage dépend de la configuration : elle est éprouvée à part.
///
/// Ce qu'on vérifie est **le chemin**, pas le comportement — celui-ci est
/// éprouvé par les tests du module, qui appellent les services.
#[actix_web::test]
async fn les_routes_du_module_engagement_sont_montees() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let regle = id();
    let seance = id();
    let modele = id();
    let routes = vec![
        (
            "GET",
            format!("/api/admin/reminder-rules?event_id={}", id()),
        ),
        ("PUT", "/api/admin/reminder-rules".to_owned()),
        ("DELETE", format!("/api/admin/reminder-rules/{regle}")),
        ("GET", format!("/api/sessions/{seance}/reminders")),
        ("GET", format!("/api/sessions/{seance}/reminder-rule")),
        ("GET", "/api/admin/message-templates".to_owned()),
        ("GET", format!("/api/admin/message-templates/{modele}")),
        (
            "POST",
            format!("/api/admin/message-templates/{modele}/versions"),
        ),
        (
            "POST",
            format!("/api/admin/message-templates/{modele}/versions/1/publish"),
        ),
        (
            "POST",
            format!("/api/admin/message-templates/{modele}/preview"),
        ),
        ("GET", "/api/notifications".to_owned()),
        ("POST", "/api/notifications/read".to_owned()),
        ("POST", "/api/notifications/archive".to_owned()),
        ("GET", "/api/notification-preferences".to_owned()),
        ("PUT", "/api/notification-preferences".to_owned()),
        ("GET", "/api/admin/email-suppressions".to_owned()),
        ("POST", "/api/admin/email-suppressions".to_owned()),
        (
            "DELETE",
            "/api/admin/email-suppressions/qui@example.org".to_owned(),
        ),
        ("POST", "/api/admin/notifications/broadcast".to_owned()),
    ];
    assert_eq!(
        routes.len(),
        19,
        "trois d'US6, deux d'US4, cinq d'US7 et neuf d'US8"
    );

    for (verbe, chemin) in routes {
        let requete = match verbe {
            "GET" => test::TestRequest::get(),
            "PUT" => test::TestRequest::put(),
            "POST" => test::TestRequest::post(),
            _ => test::TestRequest::delete(),
        };
        let reponse = test::call_service(&app, requete.uri(&chemin).to_request()).await;

        assert_ne!(
            reponse.status(),
            StatusCode::NOT_FOUND,
            "{verbe} {chemin} n'est pas montée"
        );
        assert_eq!(
            reponse.status(),
            StatusCode::UNAUTHORIZED,
            "{verbe} {chemin} devrait exiger une session"
        );
    }
}

/// Les deux modules sont **déployés** au semis : sans cela, leurs routes ne
/// seraient jamais montées et tous les tests de ce fichier passeraient au vert
/// pour la pire des raisons.
#[actix_web::test]
async fn les_deux_modules_sont_deployes() {
    let base = TestDb::new().await;

    let codes = sqlx::query_scalar!(
        "SELECT code FROM platform.modules
          WHERE code IN ('media', 'engagement') AND deployment <> 'disabled'
          ORDER BY code"
    )
    .fetch_all(base.pool())
    .await
    .expect("lecture des modules");

    assert_eq!(codes, vec!["engagement".to_owned(), "media".to_owned()]);
}

/// La documentation engendrée porte les étiquettes des deux modules. Un module
/// monté dont la documentation ne se sert pas serait invisible à qui lit
/// `GET /api/docs`.
#[actix_web::test]
async fn la_documentation_porte_les_etiquettes_des_deux_modules() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");

    let document = api::openapi::document(&etat.modules);
    let etiquettes: Vec<String> = document
        .tags
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.name)
        .collect();

    for attendue in [
        "Média — dépôt",
        "Média — rattachements",
        "Back-office — médias",
        "Rappels — calendrier",
        "Back-office — règles de rappel",
        "Notifications",
        "Back-office — modèles de messages",
        "Délivrabilité",
    ] {
        assert!(
            etiquettes.iter().any(|e| e == attendue),
            "étiquette « {attendue} » absente de la documentation : {etiquettes:?}"
        );
    }
}

/// **La porte d'ingestion est montée quand son jeton est configuré, et fermée
/// quand il ne l'est pas.**
///
/// Une route d'ingestion sans secret vaut mieux fermée : sans jeton, elle rend
/// **404**, comme un module éteint — et non 401, qui annoncerait son existence à
/// qui sonde le port. Un défaut de configuration se remarque bien plus vite
/// quand la route disparaît que quand elle accepte tout.
#[actix_web::test]
async fn la_porte_dingestion_suit_son_jeton() {
    let base = TestDb::new().await;

    // Avec jeton : montée, et elle réclame le bon.
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/internal/mail-events")
            .set_json(Vec::<serde_json::Value>::new())
            .to_request(),
    )
    .await;
    assert_eq!(
        reponse.status(),
        StatusCode::UNAUTHORIZED,
        "montée, elle réclame son jeton"
    );

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/internal/mail-events")
            .insert_header(("Authorization", "Bearer jeton-webhook-de-test"))
            .set_json(Vec::<serde_json::Value>::new())
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK, "le bon jeton passe");

    // Sans jeton : **non montée**.
    let mut config = kernel::testing::test_config(base.url());
    config.mail.webhook_token = None;
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    let app = test::init_service(api::build_app(&etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/internal/mail-events")
            .insert_header(("Authorization", "Bearer jeton-webhook-de-test"))
            .set_json(Vec::<serde_json::Value>::new())
            .to_request(),
    )
    .await;
    assert_eq!(
        reponse.status(),
        StatusCode::NOT_FOUND,
        "sans secret, la porte n'existe pas — jamais un 401, qui l'annoncerait"
    );

    let corps: serde_json::Value = test::read_body_json(reponse).await;
    assert_eq!(corps["code"], "NOT_FOUND");
}
