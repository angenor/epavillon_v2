//! L'accès, côté application — **deux routes, et un seul écran par route**.
//!
//! # LES NEUF ISSUES D'UN CODE SORTENT EN 200
//!
//! Sept d'entre elles sont des refus, et ce ne sont pas sept pannes : elles font
//! partie du parcours. En 4xx, le transport du client les traiterait comme des
//! erreurs et l'écran perdrait ses sorties, là où la spécification exige
//! qu'aucune issue ne laisse l'écran sans suite (FR-015, recherche R3). C'est
//! exactement ce que la connexion fait déjà pour `mfa_required`.
//!
//! **Ce qui reste en HTTP** : l'absence de session (401), et un corps malformé
//! (422). Rien d'autre.

use actix_web::http::header::{ETAG, IF_NONE_MATCH};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::context::RequestContext;
use kernel::error::Result;
use serde::Deserialize;

use crate::service::{access, redeem};
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/negotiation/me/access", web::get().to(mon_acces))
        .route(
            "/negotiation/invitation-codes/redeem",
            web::post().to(saisir_un_code),
        );
}

/// Ce que le client envoie. `device_id` est **déclaré**, donc forgeable : il est
/// gardé avec l'essai pour qu'un administrateur lise une série d'échecs, et le
/// comptage se fait par personne.
#[derive(Debug, Deserialize)]
pub struct RedeemPayload {
    pub code: String,
    #[serde(default)]
    pub device_id: Option<String>,
}

#[utoipa::path(
    get,
    description = "`AccessStateView` — l'accès de la personne connectée, en une seule lecture : le mode d'admission courant, son état, ce que son accès ouvre, ses appartenances de réseau et sa dernière demande.\n\n**Le parcours d'entrée, le verrou d'un module réservé et « Mon accès » lisent cette route et rien d'autre.** Elle passe par `useGnLecture` : hors connexion, l'application affiche ce qui a été lu avec l'heure de sa lecture (principe XI).\n\n`state` est **dérivé du RBAC** — `identity.has_permission` fait foi —, jamais d'une colonne d'état : deux vérités divergent toujours, et ce jour-là c'est l'écran qui mentirait. Les appartenances de réseau **n'ouvrent aucun droit** à cette étape.\n\nLa réponse porte un `ETag` calculé sur son contenu, et rend **304** sur `If-None-Match`. **Rien ici ne nomme ni ne suppose un genre.**",
    path = "/negotiation/me/access",
    tag = "Guide Négo — accès",
    operation_id = "negotiation_mon_acces",
    responses(
        (status = 200, description = "AccessStateView", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn mon_acces(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
) -> Result<HttpResponse> {
    let etat = access::mon_acces(&state, acteur.0, &crate::routes::locale_de(&requete)).await?;

    // L'empreinte est calculée **sur la réponse rendue**, et non sur une date de
    // modification : l'état vient de quatre tables et de `now()`, et aucune
    // colonne ne porte l'instant où il a changé. Une empreinte de contenu ne
    // peut pas se tromper ; un « modifié depuis » le pourrait, et un 304 fautif
    // laisserait ouverts, sur le téléphone, les modules d'un accès retiré.
    let corps = serde_json::to_string(&etat).map_err(kernel::error::ApiError::internal)?;
    let empreinte = empreinte_de(&corps);

    if crate::routes::entete(&requete, IF_NONE_MATCH.as_str()).as_deref() == Some(&empreinte) {
        return Ok(HttpResponse::NotModified()
            .insert_header((ETAG, empreinte))
            .finish());
    }

    Ok(HttpResponse::Ok()
        .insert_header((ETAG, empreinte))
        .content_type("application/json")
        .body(corps))
}

#[utoipa::path(
    post,
    description = "`RedeemPayload` → `RedeemResult` — saisir le code reçu dans le groupe WhatsApp.\n\n**Les neuf issues sortent en 200**, chacune avec son discriminant `issue` et son `message` français composé par l'API — elle seule connaît la date de révocation et le temps d'attente restant. Le client l'affiche **tel quel** (FR-020) ; les titres, les aides et les boutons autour restent de l'i18n.\n\nLes issues : `accepted`, `pending_approval` (mode « les deux »), `already_granted`, `unknown`, `revoked`, `exhausted`, `expired`, `not_yet_valid`, `throttled`.\n\nLe code est comparé **insensiblement à la casse et aux séparateurs** : `nego-024`, `NEGO 024` et `Nego024` désignent le même code. Il fait huit caractères, tirets compris.\n\n**Le quota est tenu par la base**, pas par une lecture préalable : un code de 120 usages en accorde 120, jamais 121, même si deux personnes entrent à la même seconde. Les essais sont limités **par personne**, tous appareils confondus.",
    path = "/negotiation/invitation-codes/redeem",
    tag = "Guide Négo — accès",
    operation_id = "negotiation_saisir_un_code",
    request_body = Object,
    responses(
        (status = 200, description = "RedeemResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Code vide ou corps malformé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn saisir_un_code(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Actor,
    charge: web::Json<RedeemPayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, acteur.0);
    let session_id = requete
        .extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.session_id);

    let resultat = redeem::redeem(
        &state,
        &contexte,
        redeem::Saisie {
            person_id: acteur.0,
            session_id,
            code: &charge.code,
            device_id: charge.device_id.as_deref(),
            locale: &contexte.locale,
        },
    )
    .await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Empreinte faible et courte, entre guillemets comme l'exige l'en-tête.
fn empreinte_de(corps: &str) -> String {
    let octets = kernel::crypto::token_hash(corps);
    let mut hexa = String::with_capacity(32);
    for octet in &octets[..16] {
        hexa.push_str(&format!("{octet:02x}"));
    }
    format!("\"{hexa}\"")
}
