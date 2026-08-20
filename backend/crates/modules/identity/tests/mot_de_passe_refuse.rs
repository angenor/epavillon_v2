//! Un mot de passe refusé **désigne le champ fautif** et porte un message
//! français.
//!
//! C'est la seule erreur de ces deux écrans qui sorte en 422 : tous les refus de
//! jeton sortent en 200 avec leur discriminant. La distinction n'est pas
//! cosmétique — un formulaire qui se corrige sur place n'a pas la même suite
//! qu'un lien à redemander par courriel.
//!
//! Les deux portes qui acceptent un mot de passe sont éprouvées ensemble :
//! l'inscription et la réinitialisation. Deux écrans qui exigeraient deux choses
//! différentes du même mot de passe seraient impossibles à défendre.

mod commun;

use commun::{semer, Bac, Compte};
use kernel::error::{ApiError, ErrorCode};

const ADRESSE: &str = "awa.diallo@example.org";

/// Trop court, sans majuscule, sans minuscule : les trois règles opposables,
/// une par cas.
const REFUSES: [&str; 3] = ["Cop31", "belem2027", "BELEM2027"];

fn verifier(erreur: &ApiError) {
    assert_eq!(erreur.code, ErrorCode::IdentityPasswordTooWeak);
    assert_eq!(
        erreur.field.as_deref(),
        Some("password"),
        "le champ fautif est nommé : l'écran souligne la bonne case"
    );
    assert_eq!(
        erreur.code.status(),
        actix_web::http::StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(
        erreur.message.contains("majuscule") && erreur.message.contains("caractères"),
        "le message est en français et nomme les trois conditions d'un coup : {}",
        erreur.message
    );
}

#[tokio::test]
async fn linscription_refuse_un_mot_de_passe_non_conforme() {
    let bac = Bac::monter().await;

    for faible in REFUSES {
        let erreur = identity::service::registration::register(
            &bac.state,
            &bac.ctx(),
            identity::service::registration::RegisterRequest {
                first_name: "Awa",
                last_name: "Diallo",
                email: ADRESSE,
                country_id: None,
                password: faible,
                preferred_locale: "fr",
                timezone: "Africa/Dakar",
            },
        )
        .await
        .expect_err(&format!("« {faible} » devait être refusé"));

        verifier(&erreur);
    }

    let creees = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.people
            WHERE primary_email = $1::text::platform.email"#,
        ADRESSE
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(creees, 0, "rien n'est créé quand la saisie est refusée");
}

/// **Le lien n'est pas brûlé** par une saisie refusée : la personne corrige et
/// renvoie, sans repasser par sa boîte aux lettres.
#[tokio::test]
async fn la_reinitialisation_refuse_sans_consommer_le_jeton() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    identity::service::password_reset::request(&bac.state, &bac.ctx(), ADRESSE)
        .await
        .expect("demande");

    let jeton = sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_password_reset_email'
          ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi")
    .expect("le travail porte le jeton");

    for faible in REFUSES {
        let erreur =
            identity::service::password_reset::confirm(&bac.state, &bac.ctx(), &jeton, faible)
                .await
                .expect_err(&format!("« {faible} » devait être refusé"));
        verifier(&erreur);
    }

    let consomme = sqlx::query_scalar!(
        "SELECT consumed_at FROM identity.one_time_tokens WHERE purpose = 'password_reset'"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("relecture du jeton");
    assert!(
        consomme.is_none(),
        "une saisie refusée ne brûle pas le lien"
    );

    assert!(
        matches!(
            identity::service::password_reset::confirm(
                &bac.state,
                &bac.ctx(),
                &jeton,
                "Ouagadougou2027"
            )
            .await
            .expect("enregistrement"),
            identity::domain::token::PasswordResetOutcome::Reset { .. }
        ),
        "le même lien sert encore une fois la saisie corrigée"
    );
}
