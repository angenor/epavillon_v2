//! Routes d'authentification.
//!
//! **Les six issues de connexion sortent en 200.** Un refus prévu par le contrat
//! du site n'est pas une erreur HTTP : son client lève une exception sur tout
//! statut d'erreur, et rendre 401 sur un mot de passe faux ferait afficher un
//! écran en panne au lieu du message attendu. Ce n'est pas un adoucissement —
//! un `invalid_credentials` en 200 divulgue moins qu'un 401 qui se
//! distinguerait d'un 404.

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, ResponseError};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::{Deserialize, Serialize};

use crate::domain::ids::PersonId;
use crate::repo::people;
use crate::routes::cookies;
use crate::service::auth::{self, LoginRequest};
use crate::service::password_reset;
use crate::service::registration;
use crate::service::session::{self, Device, RefreshOutcome};
use crate::state::IdentityState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/me", web::get().to(me))
            .route("/refresh", web::post().to(refresh))
            .route("/register", web::post().to(register))
            .route("/verify-email", web::post().to(verify_email))
            .route("/verify-email/resend", web::post().to(resend_verification))
            .route("/password-reset", web::post().to(request_password_reset))
            .route(
                "/password-reset/check",
                web::get().to(check_password_reset_token),
            )
            .route("/password-reset/confirm", web::post().to(reset_password)),
    );
}

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub remember_me: bool,
}

#[derive(Debug, Deserialize)]
pub struct RegisterPayload {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub country_id: Option<uuid::Uuid>,
    pub password: String,
    pub preferred_locale: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenPayload {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct EmailPayload {
    pub email: String,
}

/// Le jeton vient de la **chaîne de requête** : le contrôle est un `GET`, et un
/// `GET` avec un corps ne se met pas en cache, ne se rejoue pas, et ne se
/// documente pas.
#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordPayload {
    pub token: String,
    pub password: String,
}

#[derive(Serialize)]
struct Statut {
    status: &'static str,
}

#[utoipa::path(
    post,
    description = "`LoginPayload` → `LoginResult`. **Les six issues sortent en 200.**",
    path = "/auth/login",
    tag = "Authentification",
    operation_id = "login",
    request_body = Object,
    responses(
        (status = 200, description = "LoginResult — authenticated, mfa_required, invalid_credentials, locked, suspended, unverified_email", body = Object),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn login(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
    corps: web::Json<LoginPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let agent = entete(&requete, actix_web::http::header::USER_AGENT.as_str());
    let ip = kernel::net::client_ip(&requete, &state.config().trusted_proxies);

    let reponse = auth::login(
        &state,
        &ctx,
        LoginRequest {
            email: corps.email.trim(),
            password: &corps.password,
            remember_me: corps.remember_me,
            device: Device {
                user_agent: agent.as_deref(),
                ip,
            },
        },
    )
    .await?;

    let mut sortie = HttpResponse::Ok();
    if let Some(ouverte) = reponse.session {
        sortie.cookie(cookies::acces(
            state.config(),
            ouverte.access_token,
            state.tokens().duree(),
        ));
        sortie.cookie(cookies::rafraichissement(
            state.config(),
            ouverte.refresh_token,
            ouverte.expires_at,
        ));
    }

    Ok(sortie.json(reponse.outcome))
}

/// **Pas de 401.** Le store du site appelle cette route à chaque navigation, y
/// compris déconnecté ; un statut d'erreur y ferait afficher un écran en panne
/// au lieu d'un état déconnecté. Aucun identifiant n'est accepté du client :
/// c'est la session qui dit qui parle (FR-034).
#[utoipa::path(
    get,
    description = "`Person | null`. **Jamais 401** : le site appelle cette route déconnecté.",
    path = "/auth/me",
    tag = "Authentification",
    operation_id = "me",
    responses(
        (status = 200, description = "Person | null — corps null hors session", body = Object),
    )
)]
pub(crate) async fn me(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let Some(acteur) = acteur(&requete) else {
        return Ok(HttpResponse::Ok().json(serde_json::Value::Null));
    };

    let personne = people::view(state.pool(), PersonId(acteur)).await?;
    Ok(HttpResponse::Ok().json(personne))
}

#[utoipa::path(
    post,
    description = "Rotation du jeton de session.",
    path = "/auth/refresh",
    tag = "Authentification",
    operation_id = "refresh",
    request_body = Object,
    responses(
        (status = 200, description = "{ status: \"renewed\" | \"expired\" }", body = Object),
    )
)]
pub(crate) async fn refresh(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let agent = entete(&requete, actix_web::http::header::USER_AGENT.as_str());
    let ip = kernel::net::client_ip(&requete, &state.config().trusted_proxies);

    let Some(jeton) = jeton_de_rafraichissement(&requete) else {
        return Ok(HttpResponse::Ok().json(Statut { status: "expired" }));
    };

    let demande = session::refresh(
        &state,
        &ctx,
        &jeton,
        Device {
            user_agent: agent.as_deref(),
            ip,
        },
    )
    .await;

    let issue = match demande {
        Ok(issue) => issue,
        // Le rejeu vient de faire tomber toutes les sessions : laisser les deux
        // cookies dans le navigateur ferait rejouer la même détection à chaque
        // appel, et remplirait le journal d'alertes sans objet.
        Err(erreur) if erreur.code == ErrorCode::IdentityRefreshReused => {
            let mut reponse = erreur.error_response();
            for cookie in cookies::effacer(state.config()) {
                reponse.add_cookie(&cookie).map_err(ApiError::internal)?;
            }
            return Ok(reponse);
        }
        Err(erreur) => return Err(erreur),
    };

    match issue {
        RefreshOutcome::Renewed(ouverte) => {
            let mut sortie = HttpResponse::Ok();
            sortie.cookie(cookies::acces(
                state.config(),
                ouverte.access_token,
                state.tokens().duree(),
            ));
            sortie.cookie(cookies::rafraichissement(
                state.config(),
                ouverte.refresh_token,
                ouverte.expires_at,
            ));
            Ok(sortie.json(Statut { status: "renewed" }))
        }
        RefreshOutcome::Expired => {
            let mut sortie = HttpResponse::Ok();
            for cookie in cookies::effacer(state.config()) {
                sortie.cookie(cookie);
            }
            Ok(sortie.json(Statut { status: "expired" }))
        }
    }
}

/// **Aucune session n'est ouverte par l'inscription** : l'adresse n'est pas
/// encore vérifiée, et une adresse non vérifiée ne se connecte pas (FR-024).
///
/// La langue et le fuseau viennent de l'interface et du navigateur ; à défaut,
/// la langue négociée par l'intergiciel et l'UTC. Deux colonnes `NOT NULL`
/// remplies sans deux champs de formulaire de plus.
#[utoipa::path(
    post,
    description = "`RegisterPayload` → `RegisterResult`. **Réponse invariable**, adresse libre ou prise.",
    path = "/auth/register",
    tag = "Authentification",
    operation_id = "register",
    request_body = Object,
    responses(
        (status = 200, description = "RegisterResult", body = Object),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn register(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
    corps: web::Json<RegisterPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);

    let reponse = registration::register(
        &state,
        &ctx,
        registration::RegisterRequest {
            first_name: corps.first_name.trim(),
            last_name: corps.last_name.trim(),
            email: corps.email.trim(),
            country_id: corps.country_id,
            password: &corps.password,
            preferred_locale: corps.preferred_locale.as_deref().unwrap_or(&ctx.locale),
            timezone: corps.timezone.as_deref().unwrap_or("UTC"),
        },
    )
    .await?;

    Ok(HttpResponse::Ok().json(reponse))
}

/// Les trois refus sortent en **200** avec leur discriminant : le site les
/// distingue pour ne pas envoyer redemander un courriel à qui a déjà cliqué.
#[utoipa::path(
    post,
    description = "`VerifyEmailResult` — « déjà utilisé » avant « périmé ».",
    path = "/auth/verify-email",
    tag = "Authentification",
    operation_id = "verify_email",
    request_body = Object,
    responses(
        (status = 200, description = "VerifyEmailResult", body = Object),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn verify_email(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
    corps: web::Json<TokenPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let issue = registration::verify_email(&state, &ctx, corps.token.trim()).await?;
    Ok(HttpResponse::Ok().json(issue))
}

/// **Réponse invariable** (FR-036) : adresse inconnue, déjà vérifiée ou en
/// attente, la réponse est la même.
#[utoipa::path(
    post,
    description = "`ResendVerificationResult`. **Réponse invariable.**",
    path = "/auth/verify-email/resend",
    tag = "Authentification",
    operation_id = "resend_verification",
    request_body = Object,
    responses(
        (status = 200, description = "ResendVerificationResult", body = Object),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn resend_verification(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
    corps: web::Json<EmailPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let issue = registration::resend_verification(&state, &ctx, corps.email.trim()).await?;
    Ok(HttpResponse::Ok().json(issue))
}

/// **Réponse invariable** (FR-036) : adresse connue ou non, la réponse est la
/// même. Seul le courriel diffère, et il n'arrive que si le compte existe.
#[utoipa::path(
    post,
    description = "`PasswordResetRequestResult`. **Réponse invariable.**",
    path = "/auth/password-reset",
    tag = "Authentification",
    operation_id = "request_password_reset",
    request_body = Object,
    responses(
        (status = 200, description = "PasswordResetRequestResult", body = Object),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn request_password_reset(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
    corps: web::Json<EmailPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let issue = password_reset::request(&state, &ctx, corps.email.trim()).await?;
    Ok(HttpResponse::Ok().json(issue))
}

/// Contrôle **avant** d'afficher le formulaire : il ne consomme rien, et ne vaut
/// aucune garantie — le jeton est revérifié à l'envoi (FR-042).
#[utoipa::path(
    get,
    description = "Contrôle du lien **sans le consommer**, avant d'afficher le formulaire.",
    path = "/auth/password-reset/check",
    tag = "Authentification",
    operation_id = "check_password_reset_token",
    params(("token" = String, Query, description = "Le jeton reçu par courriel")),
    responses(
        (status = 200, description = "TokenCheckResult", body = Object),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn check_password_reset_token(
    state: web::Data<IdentityState>,
    requete: web::Query<TokenQuery>,
) -> Result<HttpResponse> {
    let issue = password_reset::check(&state, requete.token.trim()).await?;
    Ok(HttpResponse::Ok().json(issue))
}

/// **Deux statuts, et ils ne disent pas la même chose.** Un jeton refusé sort en
/// 200 avec son discriminant : l'écran propose de redemander un lien. Un mot de
/// passe refusé sort en 422 sur le champ `password` : le formulaire se corrige
/// sur place, sans repasser par la boîte aux lettres.
#[utoipa::path(
    post,
    description = "Le jeton est **revérifié ici**, pas seulement au contrôle. Révoque toutes les sessions.",
    path = "/auth/password-reset/confirm",
    tag = "Authentification",
    operation_id = "reset_password",
    request_body = Object,
    responses(
        (status = 200, description = "PasswordResetResult", body = Object),
        (status = 422, description = "Requête invalide", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn reset_password(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
    corps: web::Json<ResetPasswordPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let issue = password_reset::confirm(&state, &ctx, corps.token.trim(), &corps.password).await?;
    Ok(HttpResponse::Ok().json(issue))
}

/// Réussit même sans session : se déconnecter deux fois n'est pas une erreur.
#[utoipa::path(
    post,
    description = "Ferme la session portée par le cookie. **Réussit même sans session.**",
    path = "/auth/logout",
    tag = "Authentification",
    operation_id = "logout",
    request_body = Object,
    responses(
        (status = 200, description = "{ status: \"signed_out\" }", body = Object),
    )
)]
pub(crate) async fn logout(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let ctx = contexte(&requete);
    let jeton = jeton_de_rafraichissement(&requete);
    session::logout(&state, &ctx, jeton.as_deref()).await?;

    let mut sortie = HttpResponse::Ok();
    for cookie in cookies::effacer(state.config()) {
        sortie.cookie(cookie);
    }
    Ok(sortie.json(Statut {
        status: "signed_out",
    }))
}

// -----------------------------------------------------------------------------

fn contexte(requete: &HttpRequest) -> RequestContext {
    requete
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::new(RequestContext::generated_request_id(), "fr"))
}

fn acteur(requete: &HttpRequest) -> Option<uuid::Uuid> {
    requete
        .extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.actor_id)
}

fn jeton_de_rafraichissement(requete: &HttpRequest) -> Option<String> {
    requete
        .cookie(cookies::COOKIE_RAFRAICHISSEMENT)
        .map(|c| c.value().to_owned())
        .filter(|v| !v.is_empty())
}

fn entete(requete: &HttpRequest, nom: &str) -> Option<String> {
    requete
        .headers()
        .get(nom)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Sert l'intergiciel de session de l'API : une signature vérifiée, puis la
/// session relue en base. Les permissions, elles, ne voyagent jamais dans le
/// jeton — elles se relisent à chaque requête, et c'est ce qui rend une
/// révocation immédiate (research.md § R1).
///
/// `Ok(None)` dit « aucune session valide » — jeton absent, mal signé, périmé,
/// session révoquée, personne suspendue. Une base injoignable, elle, sort en
/// **erreur** : la confondre avec une absence de session ferait annoncer
/// « déconnecté » à quelqu'un qui ne l'est pas.
pub async fn resolve_actor(
    pool: &sqlx::PgPool,
    codec: &crate::domain::access_token::AccessTokenCodec,
    jeton: &str,
) -> Result<Option<uuid::Uuid>> {
    let Some(charge) = codec.verify(jeton) else {
        return Ok(None);
    };
    let personne = crate::repo::sessions::resolve_active(pool, charge.session_id).await?;
    Ok(personne.map(PersonId::as_uuid))
}

/// Le nom du cookie d'accès, pour l'intergiciel qui le lit.
pub const COOKIE_ACCES: &str = cookies::COOKIE_ACCES;
