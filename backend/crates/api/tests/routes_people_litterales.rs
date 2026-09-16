//! **`/people/lookup` ET `/people/suggest` NE SONT PAS LUES COMME DES UUID.**
//!
//! Trois modules déposent des routes sous `/people`, et l'identité y déclare
//! `/{id}`. Actix retient la PREMIÈRE route dont le motif correspond : déclarée
//! avant, `/{id}` capture « lookup » et « suggest », échoue à en faire un UUID,
//! et rend 404.
//!
//! **Le test des routes ne pouvait pas le voir** : sans session, toutes ces
//! adresses répondent 401 — le refus d'authentification précède la lecture du
//! chemin. Le défaut n'apparaît qu'une fois connecté, c'est-à-dire chez
//! l'utilisateur. Mesuré le 16/09 sur la recherche d'un intervenant, qui n'avait
//! donc jamais fonctionné contre l'API réelle.
//!
//! Ce test frappe donc la vraie application **avec une session**.

use actix_web::http::StatusCode;
use actix_web::test;
use kernel::crypto::Passwords;
use kernel::testing::TestDb;
use serde_json::Value;

const ADRESSE: &str = "deposante@example.org";
const MOT_DE_PASSE: &str = "Belem2027!";

macro_rules! avec_session {
    ($app:expr, $requete:expr, $cookies:expr) => {{
        let mut requete = $requete;
        for cookie in $cookies.iter() {
            requete = requete.cookie(cookie.clone());
        }
        test::call_service(&$app, requete.to_request()).await
    }};
}

#[actix_web::test]
async fn les_chemins_litteraux_de_people_repondent_une_fois_connecte() {
    let base = TestDb::new().await;
    let config = kernel::testing::test_config(base.url());
    let etat = api::state::AppState::new(base.db(), config)
        .await
        .expect("état de l'application");

    let empreinte = Passwords::new()
        .expect("Argon2id")
        .hash(MOT_DE_PASSE)
        .expect("empreinte");

    let personne: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Mariam', 'Diallo', now())
         RETURNING id",
    )
    .bind(ADRESSE)
    .fetch_one(base.pool())
    .await
    .expect("insertion de la déposante");

    sqlx::query(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())",
    )
    .bind(personne)
    .bind(&empreinte)
    .execute(base.pool())
    .await
    .expect("insertion du compte");

    let app = test::init_service(api::build_app(&etat)).await;

    let connexion = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(serde_json::json!({ "email": ADRESSE, "password": MOT_DE_PASSE }))
            .to_request(),
    )
    .await;
    assert_eq!(connexion.status(), StatusCode::OK);
    let cookies: Vec<_> = connexion
        .response()
        .cookies()
        .map(|c| c.into_owned())
        .collect();

    // La recherche par adresse exacte : elle rend `null` quand personne ne la
    // porte, jamais 404 — un 404 ici veut dire « lue comme un identifiant ».
    let reponse = avec_session!(
        app,
        test::TestRequest::get().uri("/api/people/lookup?email=inconnue@example.org"),
        cookies
    );
    assert_eq!(
        reponse.status(),
        StatusCode::OK,
        "/people/lookup est capturée par /people/{{id}}"
    );
    let corps: Value = test::read_body_json(reponse).await;
    assert!(corps.is_null(), "une adresse inconnue rend null");

    // La suggestion : une LISTE, vide si rien ne correspond. Ne rien trouver
    // n'empêche jamais de saisir l'intervenant à la main.
    let reponse = avec_session!(
        app,
        test::TestRequest::get().uri("/api/people/suggest?q=inconnue"),
        cookies
    );
    assert_eq!(
        reponse.status(),
        StatusCode::OK,
        "/people/suggest est capturée par /people/{{id}}"
    );
    let corps: Value = test::read_body_json(reponse).await;
    assert_eq!(corps, serde_json::json!([]));

    // Et le chemin paramétré répond toujours : l'ordre ne l'a pas éteint.
    let reponse = avec_session!(
        app,
        test::TestRequest::get().uri(&format!("/api/people/{personne}")),
        cookies
    );
    assert_ne!(
        reponse.status(),
        StatusCode::NOT_FOUND,
        "/people/{{id}} est devenue muette"
    );
}
