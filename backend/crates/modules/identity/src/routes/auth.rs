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
use crate::repo::sessions::ClientKind;
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

/// Ce que le client déclare de lui-même, **facultatif**. L'objet absent vaut
/// `{"kind": "web"}` : aucun appel du site ne change.
///
/// Les trois champs d'appareil sont **déclaratifs** et n'accordent aucun droit,
/// au même titre que `user_agent` — un client peut les forger. Ils servent à
/// nommer une session dans une liste, et à compter les téléphones sans dédoubler
/// personne.
#[derive(Debug, Deserialize)]
pub struct ClientPayload {
    pub kind: String,
    pub device_id: Option<String>,
    pub label: Option<String>,
    pub platform: Option<String>,
}

/// Longueurs tenues **ici** plutôt qu'en base : ce sont des champs d'affichage,
/// et un libellé trop long est une faute de saisie du client, pas une atteinte
/// à un invariant.
const DEVICE_ID_MAX: usize = 128;
const DEVICE_LABEL_MAX: usize = 80;

impl ClientPayload {
    /// Un `kind` inconnu **désigne son champ** et n'est jamais corrigé en
    /// silence : accepter « iOS » en le repliant sur « web » ferait mentir le
    /// décompte des téléphones sans que rien ne le signale.
    fn valider(&self) -> Result<Device<'_>> {
        let kind = ClientKind::parse(self.kind.trim()).ok_or_else(|| {
            ApiError::validation(
                "Ce type de client n'existe pas : « web » ou « app ».",
                "client.kind",
            )
        })?;

        verifier_longueur(self.device_id.as_deref(), DEVICE_ID_MAX, "client.device_id")?;
        verifier_longueur(self.label.as_deref(), DEVICE_LABEL_MAX, "client.label")?;

        Ok(Device {
            user_agent: None,
            ip: None,
            client_kind: kind,
            device_id: nettoyer(self.device_id.as_deref()),
            device_label: nettoyer(self.label.as_deref()),
            device_platform: nettoyer(self.platform.as_deref()),
        })
    }
}

fn nettoyer(valeur: Option<&str>) -> Option<&str> {
    valeur.map(str::trim).filter(|v| !v.is_empty())
}

fn verifier_longueur(valeur: Option<&str>, maximum: usize, champ: &'static str) -> Result<()> {
    match valeur {
        Some(v) if v.chars().count() > maximum => Err(ApiError::validation(
            format!("Cette valeur dépasse {maximum} caractères."),
            champ,
        )),
        _ => Ok(()),
    }
}

/// Le client déclaré, complété par ce que la requête dit d'elle-même. L'objet
/// absent vaut « le site », et le site n'a rien à changer à ses appels.
fn client_de<'a>(
    declare: Option<&'a ClientPayload>,
    agent: Option<&'a str>,
    ip: Option<std::net::IpAddr>,
) -> Result<Device<'a>> {
    let mut device = match declare {
        Some(client) => client.valider()?,
        None => Device::default(),
    };
    device.user_agent = agent;
    device.ip = ip;
    Ok(device)
}

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub remember_me: bool,
    #[serde(default)]
    pub client: Option<ClientPayload>,
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
    #[serde(default)]
    pub client: Option<ClientPayload>,
}

#[derive(Debug, Deserialize)]
pub struct TokenPayload {
    pub token: String,
}

/// Le renvoi d'un lien et la demande de mot de passe portent eux aussi leur
/// client : c'est depuis l'application qu'on redemande, c'est là qu'il faut
/// revenir. Facultatif, comme partout ailleurs.
#[derive(Debug, Deserialize)]
pub struct EmailPayload {
    pub email: String,
    #[serde(default)]
    pub client: Option<ClientPayload>,
}

impl EmailPayload {
    fn client(&self) -> Result<ClientKind> {
        Ok(match self.client.as_ref() {
            Some(declare) => declare.valider()?.client_kind,
            None => ClientKind::Web,
        })
    }
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
        (status = 200, description = "LoginResult — authenticated, mfa_required, invalid_credentials, locked, suspended, email_unverified", body = Object),
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

    let device = client_de(corps.client.as_ref(), agent.as_deref(), ip)?;

    let reponse = auth::login(
        &state,
        &ctx,
        LoginRequest {
            email: corps.email.trim(),
            password: &corps.password,
            remember_me: corps.remember_me,
            device,
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

/// La session courante, jointe à la personne. C'est ce qui permet au profil de
/// dire de quel appareil on est connecté **sans requête de plus**.
#[derive(Serialize)]
struct SessionCourante {
    client_kind: &'static str,
    device_label: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    issued_at: time::OffsetDateTime,
}

/// Le champ s'AJOUTE à la personne, il ne l'enveloppe pas : le site lit déjà
/// cette réponse comme un `Person`, et l'envelopper casserait chacun de ses
/// appels pour un champ dont il n'a pas l'usage.
#[derive(Serialize)]
struct MoiResponse {
    #[serde(flatten)]
    person: crate::domain::person::PersonView,
    session: Option<SessionCourante>,
}

/// **Pas de 401.** Le store du site appelle cette route à chaque navigation, y
/// compris déconnecté ; un statut d'erreur y ferait afficher un écran en panne
/// au lieu d'un état déconnecté. Aucun identifiant n'est accepté du client :
/// c'est la session qui dit qui parle (FR-034).
#[utoipa::path(
    get,
    description = "`Person & { session }`. **Jamais 401** : le site appelle cette route déconnecté.",
    path = "/auth/me",
    tag = "Authentification",
    operation_id = "me",
    responses(
        (status = 200, description = "Person & { session } | null — corps null hors session", body = Object),
    )
)]
pub(crate) async fn me(
    state: web::Data<IdentityState>,
    requete: HttpRequest,
) -> Result<HttpResponse> {
    let Some(acteur) = acteur(&requete) else {
        return Ok(HttpResponse::Ok().json(serde_json::Value::Null));
    };

    let Some(person) = people::view(state.pool(), PersonId(acteur)).await? else {
        return Ok(HttpResponse::Ok().json(serde_json::Value::Null));
    };

    // L'identifiant de session sort du jeton d'accès, déjà vérifié par
    // l'intergiciel. Le contexte de requête ne porte que l'acteur : y ajouter la
    // session toucherait un type que tous les modules partagent, pour un besoin
    // qui n'en concerne qu'une route.
    let session = match session_courante(&requete).and_then(|jeton| state.tokens().verify(&jeton)) {
        Some(charge) => crate::repo::sessions::describe(state.pool(), charge.session_id)
            .await?
            .map(|s| SessionCourante {
                client_kind: s.client_kind.as_db(),
                device_label: s.device_label,
                issued_at: s.issued_at,
            }),
        None => None,
    };

    Ok(HttpResponse::Ok().json(MoiResponse { person, session }))
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

    // **Aucun objet `client` ici, et c'est le point.** La rotation recopie le
    // client de la session remplacée ; l'accepter du corps permettrait de
    // changer d'appareil déclaré en le demandant.
    let demande = session::refresh(
        &state,
        &ctx,
        &jeton,
        Device {
            user_agent: agent.as_deref(),
            ip,
            ..Device::default()
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

    // Le client est retenu AVEC LE JETON, pas avec une session : au moment où
    // le lien du courriel s'ouvre, la personne n'en a pas encore.
    let client = match corps.client.as_ref() {
        Some(declare) => declare.valider()?.client_kind,
        None => ClientKind::Web,
    };

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
            client,
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
    let issue =
        registration::resend_verification(&state, &ctx, corps.email.trim(), corps.client()?)
            .await?;
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
    let issue = password_reset::request(&state, &ctx, corps.email.trim(), corps.client()?).await?;
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

fn session_courante(requete: &HttpRequest) -> Option<String> {
    requete
        .cookie(cookies::COOKIE_ACCES)
        .map(|c| c.value().to_owned())
        .filter(|v| !v.is_empty())
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
) -> Result<Option<ResolvedSession>> {
    let Some(charge) = codec.verify(jeton) else {
        return Ok(None);
    };
    let personne = crate::repo::sessions::resolve_active(pool, charge.session_id).await?;
    Ok(personne.map(|person_id| ResolvedSession {
        person_id: person_id.as_uuid(),
        session_id: charge.session_id.as_uuid(),
    }))
}

/// Qui frappe, **et par quelle session**.
///
/// La session voyage jusqu'au contexte de requête parce qu'un module qui n'a pas
/// le droit de dépendre de celui-ci en a besoin : l'usage d'un code
/// d'invitation la garde, et c'est ce qui dira plus tard d'où une entrée est
/// partie.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedSession {
    pub person_id: uuid::Uuid,
    pub session_id: uuid::Uuid,
}

/// Le nom du cookie d'accès, pour l'intergiciel qui le lit.
pub const COOKIE_ACCES: &str = cookies::COOKIE_ACCES;
