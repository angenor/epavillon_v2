//! Les en-têtes CORS, **à travers HTTP**.
//!
//! Quatre choses se perdraient sans ces tests, et aucune ne se voit avec `curl`
//! — c'est précisément pourquoi le manque a vécu si longtemps :
//!
//! 1. un préalable `OPTIONS` doit être répondu **sans atteindre la route**, qui
//!    n'accepte souvent que `POST` et rendrait 404 ;
//! 2. une réponse d'**erreur** doit porter les en-têtes elle aussi, sinon le
//!    navigateur la masque et l'écran affiche une panne réseau à la place du
//!    message français ;
//! 3. l'identifiant de requête doit être **exposé**, sans quoi le site ne peut
//!    pas le citer dans un signalement ;
//! 4. une origine inconnue ne doit recevoir **aucun** en-tête.

use actix_web::http::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, VARY,
};
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::testing::TestDb;

/// L'origine du site, celle que `APP_PUBLIC_URL` déclare dans la configuration
/// de test. La liste d'origines autorisées en est tirée, et d'elle seule.
const SITE: &str = "http://localhost:3000";
const INCONNUE: &str = "http://ailleurs.example.org";

async fn monter() -> (TestDb, AppState) {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = AppState::new(base.db(), config)
        .await
        .expect("état de l'application");
    (base, etat)
}

fn entete<B>(
    reponse: &actix_web::dev::ServiceResponse<B>,
    cle: actix_web::http::header::HeaderName,
) -> Option<String> {
    reponse
        .headers()
        .get(cle)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Le préalable d'une écriture : `OPTIONS` sur une route qui n'accepte que
/// `POST`. Il doit être répondu par l'intergiciel, pas par le routeur.
#[tokio::test]
async fn un_prealable_est_repondu_sans_atteindre_la_route() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::default()
            .method(actix_web::http::Method::OPTIONS)
            .uri("/api/auth/login")
            .insert_header(("Origin", SITE))
            .insert_header(("Access-Control-Request-Method", "POST"))
            .insert_header((
                "Access-Control-Request-Headers",
                "content-type, accept-language",
            ))
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        entete(&reponse, ACCESS_CONTROL_ALLOW_ORIGIN).as_deref(),
        Some(SITE),
        "jamais `*` : le navigateur le refuse dès que les cookies sont autorisés"
    );
    assert_eq!(
        entete(&reponse, ACCESS_CONTROL_ALLOW_CREDENTIALS).as_deref(),
        Some("true")
    );
    assert!(entete(&reponse, ACCESS_CONTROL_ALLOW_METHODS)
        .is_some_and(|m| m.contains("POST") && m.contains("DELETE")));
    assert_eq!(
        entete(&reponse, ACCESS_CONTROL_ALLOW_HEADERS).as_deref(),
        Some("content-type, accept-language"),
        "les en-têtes demandés sont renvoyés tels quels : une liste fermée écrite ici échouerait \
         en silence le jour où le site en ajoute un"
    );
    assert!(entete(&reponse, VARY).is_some_and(|v| v.contains("Origin")));
}

/// Une lecture ordinaire depuis l'origine du site.
#[tokio::test]
async fn une_reponse_ordinaire_porte_les_entetes_et_expose_lidentifiant() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/ready")
            .insert_header(("Origin", SITE))
            .to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::OK);
    assert_eq!(
        entete(&reponse, ACCESS_CONTROL_ALLOW_ORIGIN).as_deref(),
        Some(SITE)
    );
    assert_eq!(
        entete(&reponse, ACCESS_CONTROL_EXPOSE_HEADERS).as_deref(),
        Some("X-Request-Id"),
        "sans cette ligne le navigateur cache l'identifiant au code du site"
    );
}

/// **Le point qui se perd.** Un refus doit rester lisible par le code du site.
#[tokio::test]
async fn une_reponse_derreur_porte_les_entetes_elle_aussi() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    // 401 : lecture protégée, aucune session.
    let refus = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/health")
            .insert_header(("Origin", SITE))
            .to_request(),
    )
    .await;
    assert_eq!(refus.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        entete(&refus, ACCESS_CONTROL_ALLOW_ORIGIN).as_deref(),
        Some(SITE),
        "sans en-tête sur l'erreur, l'écran affiche une panne réseau au lieu du message"
    );

    // 404 : chemin inconnu, rendu par le service par défaut — donc encore plus
    // à l'intérieur que les intergiciels de refus.
    let inconnu = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/chemin-qui-nexiste-pas")
            .insert_header(("Origin", SITE))
            .to_request(),
    )
    .await;
    assert_eq!(inconnu.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        entete(&inconnu, ACCESS_CONTROL_ALLOW_ORIGIN).as_deref(),
        Some(SITE)
    );
}

/// Une origine inconnue ne reçoit rien. L'écriture reste refusée par
/// `OriginCheck` — les deux gardes disent la même chose.
#[tokio::test]
async fn une_origine_inconnue_ne_recoit_aucun_entete() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    let lecture = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/ready")
            .insert_header(("Origin", INCONNUE))
            .to_request(),
    )
    .await;
    assert!(
        entete(&lecture, ACCESS_CONTROL_ALLOW_ORIGIN).is_none(),
        "aucune permission accordée à une origine inconnue"
    );
    assert!(
        entete(&lecture, VARY).is_some_and(|v| v.contains("Origin")),
        "`Vary` reste posée : la réponse dépend de l'en-tête, un cache partagé doit le savoir"
    );

    let ecriture = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/logout")
            .insert_header(("Origin", INCONNUE))
            .set_json(serde_json::json!({}))
            .to_request(),
    )
    .await;
    assert_eq!(
        ecriture.status(),
        StatusCode::FORBIDDEN,
        "et l'écriture est refusée par OriginCheck : les deux gardes s'accordent"
    );
}

/// `curl` n'envoie pas d'origine, et rien ne change pour lui : c'est ce qui a
/// permis à ce manque de vivre si longtemps sans se voir.
#[tokio::test]
async fn un_appel_sans_origine_est_inchange() {
    let (_base, etat) = monter().await;
    let app = test::init_service(api::build_app(&etat)).await;

    let reponse = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/ready").to_request(),
    )
    .await;

    assert_eq!(reponse.status(), StatusCode::OK);
    assert!(entete(&reponse, ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}
