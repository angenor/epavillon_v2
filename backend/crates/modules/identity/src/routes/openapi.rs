//! Documentation OpenAPI du module — **générée**, jamais écrite à la main
//! (FR-063).
//!
//! Les routes s'y déclarent par annotation, auprès du gestionnaire qu'elles
//! décrivent : une route ajoutée sans son annotation ne se documente pas, mais
//! une route documentée qui disparaîtrait ne compilerait plus. C'est le seul
//! couplage qui empêche une documentation de mentir.
//!
//! **Deux arbitrages, et ils sont assumés.**
//!
//! 1. Les **formes de réponse** sont désignées par leur nom TypeScript et
//!    déclarées `object`. Leur source unique est `frontend/app/types/` — le
//!    contrat de routes le dit explicitement —, et en dériver un second jeu de
//!    schémas en Rust produirait deux vérités dont la seconde se périmerait à la
//!    première évolution du site. Le nom, lui, mène au fichier qui fait foi.
//! 2. Le **catalogue d'erreurs**, à l'inverse, vit entièrement en Rust : il est
//!    donc engendré depuis `ErrorCode::ALL`, code par code, avec son statut et
//!    son message. Un code ajouté au noyau apparaît ici au prochain démarrage.

use kernel::error::ErrorCode;
use utoipa::openapi::schema::{ObjectBuilder, Schema, SchemaType};
use utoipa::openapi::{Components, OpenApi as Document, RefOr, Type};
use utoipa::{Modify, OpenApi, ToSchema};

/// Forme du corps d'erreur, référencée par chaque route.
///
/// Les valeurs de `code` ne peuvent pas être énumérées par la dérivation — elles
/// vivent dans une macro du noyau. `CatalogueDErreurs` remplace donc ce schéma
/// au montage par celui qu'il engendre depuis `ErrorCode::ALL`, catalogue
/// complet et à jour. Cette déclaration ne sert qu'à donner un nom auquel les
/// routes se réfèrent.
#[derive(ToSchema)]
#[schema(as = ApiError)]
#[allow(dead_code)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub request_id: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::auth::login,
        crate::routes::auth::logout,
        crate::routes::auth::me,
        crate::routes::auth::refresh,
        crate::routes::auth::register,
        crate::routes::auth::verify_email,
        crate::routes::auth::resend_verification,
        crate::routes::auth::request_password_reset,
        crate::routes::auth::check_password_reset_token,
        crate::routes::auth::reset_password,
        crate::routes::people::lister,
        crate::routes::people::fiche,
        crate::routes::people::roles,
        crate::routes::people::permissions,
        crate::routes::people::perimetre,
        crate::routes::admin_users::liste,
        crate::routes::admin_users::options_dattribution,
        crate::routes::admin_users::fiche,
        crate::routes::admin_users::permissions_effectives,
        crate::routes::admin_users::attribuer_role,
        crate::routes::admin_users::retirer_role,
        crate::routes::admin_users::changer_le_statut,
        crate::routes::admin_privacy::file,
        crate::routes::admin_privacy::traiter,
    ),
    components(schemas(ApiErrorBody)),
    modifiers(&CatalogueDErreurs),
    tags(
        (name = "Authentification", description = "Connexion, session, inscription, réinitialisation. Les refus prévus par le contrat du site sortent en 200 avec leur discriminant."),
        (name = "Identité", description = "Lectures de personnes, de rôles et de périmètres. « Soi-même » est décidé par la session, jamais par un paramètre."),
        (name = "Back-office — utilisateurs", description = "Toute liste est bornée par le périmètre d'administration, y compris quand l'URL est forgée."),
        (name = "Back-office — RGPD", description = "Portée globale exigée : la file ne se borne pas par édition."),
    )
)]
pub struct IdentityApi;

/// Injecte le schéma d'erreur **depuis le catalogue du noyau**.
struct CatalogueDErreurs;

impl Modify for CatalogueDErreurs {
    fn modify(&self, document: &mut Document) {
        let composants = document.components.get_or_insert_with(Components::new);
        composants
            .schemas
            .insert("ApiError".to_owned(), schema_du_catalogue_derreurs());
    }
}

/// Le schéma d'erreur engendré depuis `ErrorCode::ALL`.
///
/// Public : l'API le repose **après** avoir fusionné les documents des modules,
/// sinon la déclaration plate qu'elle porte pour ses propres routes resterait en
/// place et la documentation n'énumérerait aucun code.
pub fn schema_du_catalogue_derreurs() -> RefOr<Schema> {
    let codes: Vec<serde_json::Value> = ErrorCode::ALL
        .iter()
        .map(|c| serde_json::Value::String(c.as_str().to_owned()))
        .collect();

    // Le tableau des codes est repris dans la description, statut par statut :
    // le front branche sur le CODE, jamais sur le texte, et c'est cette liste
    // qu'il lui faut sous les yeux.
    let catalogue = ErrorCode::ALL
        .iter()
        .map(|c| {
            format!(
                "- `{}` ({}) — {}",
                c.as_str(),
                c.status().as_u16(),
                c.message()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    ObjectBuilder::new()
        .schema_type(SchemaType::Type(Type::Object))
        .title(Some("ApiError"))
        .description(Some(format!(
            "Corps d'erreur unique de l'API. Le front branche sur `code`, jamais sur `message`.\n\n\
             Catalogue des codes stables :\n\n{catalogue}"
        )))
        .property(
            "code",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .enum_values(Some(codes))
                .description(Some("Code stable. Le renommer est un changement majeur.")),
        )
        .required("code")
        .property(
            "message",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .description(Some("Message français, affichable tel quel.")),
        )
        .required("message")
        .property(
            "field",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .description(Some("Champ fautif, quand le refus en désigne un.")),
        )
        .property(
            "request_id",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .description(Some(
                    "Identifiant de requête, à citer dans un signalement d'incident.",
                )),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La documentation porte **chaque** code stable du catalogue. Une
    /// documentation qui en oublierait un laisserait le site brancher sur un
    /// code qu'elle ne mentionne pas.
    #[test]
    fn le_catalogue_derreurs_est_complet() {
        let document = IdentityApi::openapi();
        let rendu = document.to_json().expect("sérialisation du document");

        for code in ErrorCode::ALL {
            assert!(
                rendu.contains(code.as_str()),
                "code absent de la documentation : {}",
                code.as_str()
            );
        }
    }

    /// Les vingt-quatre routes du module sont décrites — une opération par
    /// chemin, aucun chemin partagé. Le compte est écrit pour qu'une route
    /// ajoutée sans son annotation fasse échouer ce test plutôt que de passer
    /// inaperçue.
    #[test]
    fn les_routes_du_module_sont_toutes_decrites() {
        let document = IdentityApi::openapi();
        assert_eq!(document.paths.paths.len(), 24);

        let rendu = document.to_json().expect("sérialisation du document");
        let chemins: serde_json::Value = serde_json::from_str(&rendu).expect("document JSON");
        let operations: usize = chemins["paths"]
            .as_object()
            .expect("les chemins")
            .values()
            .map(|c| c.as_object().expect("un chemin").len())
            .sum();
        assert_eq!(operations, 24);
    }
}
