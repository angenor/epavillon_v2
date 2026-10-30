//! Le socle du temps réel, sur base réelle : `platform.signal()` livré au
//! commit et jamais au rollback, flux SSE filtré par sujet et par audience,
//! pouls anonyme sur un sujet public, 304.

use actix_web::body::MessageBody;
use actix_web::http::{header, StatusCode};
use actix_web::test;
use api::realtime::hub::HubMessage;
use api::state::AppState;
use async_trait::async_trait;
use kernel::context::RequestContext;
use kernel::crypto::Passwords;
use kernel::realtime::{Audience, Topic, TopicAccess, TopicAuthorizer, Viewer};
use kernel::testing::TestDb;
use serde_json::Value;
use sqlx::PgPool;
use std::pin::Pin;
use std::time::Duration;
use uuid::Uuid;

const MOT_DE_PASSE: &str = "Belem2027!";

async fn monter() -> (TestDb, AppState) {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    (base, etat)
}

async fn personne(base: &TestDb, adresse: &str) -> Uuid {
    let empreinte = Passwords::new()
        .expect("Argon2id")
        .hash(MOT_DE_PASSE)
        .expect("empreinte");
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', now())
         RETURNING id",
    )
    .bind(adresse)
    .fetch_one(base.pool())
    .await
    .expect("personne");
    sqlx::query(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())",
    )
    .bind(id)
    .bind(&empreinte)
    .execute(base.pool())
    .await
    .expect("compte");
    id
}

async fn connexion(etat: &AppState, adresse: &str) -> String {
    let app = test::init_service(api::build_app(etat)).await;
    let reponse = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({ "email": adresse, "password": MOT_DE_PASSE }))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);
    reponse
        .response()
        .cookies()
        .find(|c| c.name() == identity::COOKIE_ACCES)
        .map(|c| format!("{}={}", c.name(), c.value()))
        .expect("cookie d'accès")
}

/// Le prochain morceau du flux, ou `None` passé le délai.
async fn morceau<B: MessageBody>(corps: &mut Pin<Box<B>>, delai: Duration) -> Option<String> {
    let lu = tokio::time::timeout(
        delai,
        std::future::poll_fn(|cx| corps.as_mut().poll_next(cx)),
    )
    .await
    .ok()??;
    let octets = lu.ok()?;
    Some(String::from_utf8_lossy(&octets).into_owned())
}

/// Lit jusqu'à trouver l'événement nommé.
async fn attendre<B: MessageBody>(
    corps: &mut Pin<Box<B>>,
    nom: &str,
    delai: Duration,
) -> Option<String> {
    let fin = tokio::time::Instant::now() + delai;
    while let Some(m) = morceau(corps, fin - tokio::time::Instant::now()).await {
        if m.contains(&format!("event: {nom}\n")) {
            return Some(m);
        }
    }
    None
}

async fn signaler(etat: &AppState, sujet: &str, valider: bool) {
    let mut tx = etat
        .db
        .write(&RequestContext::background("test-temps-reel"))
        .await
        .expect("transaction");
    kernel::realtime::signal(&mut tx, sujet, None)
        .await
        .expect("signal");
    if valider {
        tx.commit().await.expect("commit");
    } else {
        tx.rollback().await.expect("rollback");
    }
}

/// L'écoute s'ouvre en tâche de fond : on attend qu'un signal témoin arrive.
async fn ecoute_prete(etat: &AppState) {
    let temoin = format!("person:{}", Uuid::now_v7());
    let sujet = Topic::parse(&temoin).expect("sujet");
    for _ in 0..50 {
        signaler(etat, &temoin, true).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        if etat.realtime.version(&sujet, &Audience::All) > 0 {
            return;
        }
    }
    panic!("l'écoute de platform_signal ne s'est pas ouverte");
}

#[tokio::test]
async fn le_signal_part_au_commit_et_jamais_au_rollback() {
    let (base, etat) = monter().await;
    etat.realtime.ecouter(base.pool());
    ecoute_prete(&etat).await;

    let mut reception = etat.realtime.abonner();
    let annule = format!("proposal:{}", Uuid::now_v7());
    let valide = format!("proposal:{}", Uuid::now_v7());
    signaler(&etat, &annule, false).await;
    signaler(&etat, &valide, true).await;

    let recu = tokio::time::timeout(Duration::from_secs(3), async {
        let mut vus = Vec::new();
        while let Ok(HubMessage::Signal { topic, .. }) = reception.recv().await {
            vus.push(topic.to_string());
            if topic.as_ref() == valide {
                break;
            }
        }
        vus
    })
    .await
    .expect("signal validé reçu en moins de trois secondes");
    assert_eq!(recu, vec![valide.clone()], "le rollback n'annonce rien");

    let sujet = Topic::parse(&annule).expect("sujet");
    assert_eq!(etat.realtime.version(&sujet, &Audience::All), 0);
}

#[tokio::test]
async fn la_base_refuse_un_sujet_mal_forme() {
    let (base, _etat) = monter().await;
    let refus = sqlx::query("SELECT platform.signal('Proposal:X')")
        .execute(base.pool())
        .await;
    assert!(refus.is_err());
    let trop_gros = sqlx::query(
        "SELECT platform.signal('person:x', jsonb_build_object('k', repeat('a', 8000)))",
    )
    .execute(base.pool())
    .await;
    assert!(trop_gros.is_err());
}

#[tokio::test]
async fn le_flux_livre_ses_sujets_et_refuse_ceux_d_autrui() {
    let (base, etat) = monter().await;
    let awa = personne(&base, "awa.diallo@francophonie.org").await;
    let autre = personne(&base, "jean.kabore@francophonie.org").await;
    let cookie = connexion(&etat, "awa.diallo@francophonie.org").await;
    let app = test::init_service(api::build_app(&etat)).await;

    // Sans session : 401.
    let anonyme = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/realtime/stream?topics=person:{awa}"))
            .to_request(),
    )
    .await;
    assert_eq!(anonyme.status(), StatusCode::UNAUTHORIZED);

    // Le sujet d'une autre personne seul : 403.
    let refuse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/realtime/stream?topics=person:{autre}"))
            .insert_header(("Cookie", cookie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(refuse.status(), StatusCode::FORBIDDEN);

    // Le sien et celui d'autrui : servi, l'autre listé comme refusé.
    let reponse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/realtime/stream?topics=person:{awa},person:{autre}"
            ))
            .insert_header(("Cookie", cookie))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);
    let entetes = reponse.headers().clone();
    assert_eq!(
        entetes.get(header::CONTENT_TYPE).unwrap(),
        "text/event-stream"
    );
    assert_eq!(
        entetes.get(header::CACHE_CONTROL).unwrap(),
        "no-cache, no-transform"
    );
    assert_eq!(entetes.get("X-Accel-Buffering").unwrap(), "no");

    let mut corps = Box::pin(reponse.into_body());
    let pret = attendre(&mut corps, "ready", Duration::from_secs(2))
        .await
        .expect("événement ready");
    assert!(
        pret.contains(&format!("person:{autre}")),
        "refus listé : {pret}"
    );
    assert!(pret.contains(&etat.realtime.boot()));

    // Rafale : un seul événement, au bout du regroupement.
    etat.realtime.recevoir(&format!("person:{autre}"), None);
    for _ in 0..5 {
        etat.realtime.recevoir(&format!("person:{awa}"), None);
    }
    let signal = attendre(&mut corps, "signal", Duration::from_secs(2))
        .await
        .expect("signal livré");
    assert!(signal.contains(&format!("person:{awa}")));
    assert!(!signal.contains(&format!("person:{autre}")));
    assert_eq!(signal.matches("event: signal").count(), 1);

    etat.realtime.annoncer_arret();
    assert!(attendre(&mut corps, "restart", Duration::from_secs(2))
        .await
        .is_some());
}

/// Un sujet de test accordé côté déposant seulement.
struct CoteDeposant;

#[async_trait]
impl TopicAuthorizer for CoteDeposant {
    fn handles(&self, topic: &Topic) -> bool {
        topic.kind() == "essai"
    }
    async fn authorize(
        &self,
        _: &PgPool,
        viewer: &Viewer,
        _: &Topic,
    ) -> kernel::Result<TopicAccess> {
        Ok(if viewer.person_id.is_some() {
            TopicAccess::Granted(Audience::Only(vec!["submitter".into()]))
        } else {
            TopicAccess::Denied
        })
    }
}

#[tokio::test]
async fn un_echange_interne_au_comite_n_atteint_pas_le_deposant() {
    let (base, mut etat) = monter().await;
    etat.topics = etat.topics.clone().register(CoteDeposant);
    personne(&base, "awa.diallo@francophonie.org").await;
    let cookie = connexion(&etat, "awa.diallo@francophonie.org").await;
    let app = test::init_service(api::build_app(&etat)).await;

    let sujet = format!("essai:{}", Uuid::now_v7());
    let reponse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/realtime/stream?topics={sujet}"))
            .insert_header(("Cookie", cookie.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);
    let mut corps = Box::pin(reponse.into_body());
    attendre(&mut corps, "ready", Duration::from_secs(2))
        .await
        .expect("ready");

    etat.realtime.recevoir(&sujet, Some("committee"));
    assert!(attendre(&mut corps, "signal", Duration::from_secs(1))
        .await
        .is_none());

    // Le pouls ne trahit pas davantage l'échange interne.
    let pouls = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/realtime/pulse?topics={sujet}"))
            .insert_header(("Cookie", cookie))
            .to_request(),
    )
    .await;
    let lu: Value = test::read_body_json(pouls).await;
    assert_eq!(lu["versions"][&sujet], 0);

    etat.realtime.recevoir(&sujet, Some("submitter"));
    assert!(attendre(&mut corps, "signal", Duration::from_secs(2))
        .await
        .is_some());
}

#[tokio::test]
async fn le_pouls_sert_le_public_sans_session_et_rend_304() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;
    let edition = Uuid::now_v7();
    let public = format!("edition:{edition}:public");
    let direct = format!("edition:{edition}:public:live");
    let uri = format!("/api/realtime/pulse?topics={public},{direct}");

    let premier = test::call_service(&app, test::TestRequest::get().uri(&uri).to_request()).await;
    assert_eq!(premier.status(), StatusCode::OK);
    let etag = premier
        .headers()
        .get(header::ETAG)
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        premier.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-cache"
    );
    let lu: Value = test::read_body_json(premier).await;
    assert_eq!(lu["boot"], etat.realtime.boot());
    assert_eq!(lu["versions"][&direct], 0);
    assert_eq!(lu["denied"].as_array().map(Vec::len), Some(0));

    let inchange = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&uri)
            .insert_header((header::IF_NONE_MATCH, etag.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);

    etat.realtime.recevoir(&direct, None);
    let change = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&uri)
            .insert_header((header::IF_NONE_MATCH, etag))
            .to_request(),
    )
    .await;
    assert_eq!(change.status(), StatusCode::OK);
    let lu: Value = test::read_body_json(change).await;
    assert_eq!(lu["versions"][&direct], 1);

    // Un sujet privé sans session : 401 ; mal formé : 422.
    let prive = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/realtime/pulse?topics=person:{}",
                Uuid::now_v7()
            ))
            .to_request(),
    )
    .await;
    assert_eq!(prive.status(), StatusCode::UNAUTHORIZED);
    let mal_forme = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/realtime/pulse?topics=Edition")
            .to_request(),
    )
    .await;
    assert_eq!(mal_forme.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn au_dela_de_cinq_flux_le_plus_ancien_est_ferme() {
    let (base, etat) = monter().await;
    let awa = personne(&base, "awa.diallo@francophonie.org").await;
    let cookie = connexion(&etat, "awa.diallo@francophonie.org").await;
    let app = test::init_service(api::build_app(&etat)).await;

    let mut corps = Vec::new();
    for _ in 0..6 {
        let reponse = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/api/realtime/stream?topics=person:{awa}"))
                .insert_header(("Cookie", cookie.clone()))
                .to_request(),
        )
        .await;
        assert_eq!(reponse.status(), StatusCode::OK);
        corps.push(Box::pin(reponse.into_body()));
    }
    assert!(attendre(&mut corps[0], "evicted", Duration::from_secs(2))
        .await
        .is_some());
}
