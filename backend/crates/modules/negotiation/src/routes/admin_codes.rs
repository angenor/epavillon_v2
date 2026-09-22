//! Le back-office des codes — **sept routes plates, une seule garde**.
//!
//! # LA GARDE EST `Requires<SpaceManage>`, ET SURTOUT PAS `RequiresAnyScope`
//!
//! Le rôle `admin` porte `negotiation.space.manage` et s'attribue **aussi sur
//! un événement** : « n'importe quelle portée » laisserait donc un
//! administrateur d'une seule édition ouvrir tout le back-office de Guide Négo,
//! alors qu'aucun espace de négociation n'est rattaché à une édition. Le piège
//! est d'autant plus sournois que la route *paraîtrait* gardée (FR-044,
//! SC-008).
//!
//! `Perimeter` ne sert pas non plus : `identity.administered_events()` ne rend
//! que des portées `event`, quand un code porte `global` ou
//! `negotiation_space`.
//!
//! # JAMAIS UN `web::scope("/admin")`
//!
//! Le préfixe est partagé avec cinq autres modules, et deux scopes du même
//! préfixe **ne se complètent pas** : Actix retient le premier dont le préfixe
//! correspond et rend 404 pour ce qui n'y figure pas. Un scope ici rendrait
//! muettes les routes des autres modules.
//!
//! # UN IDENTIFIANT FORGÉ SE REFUSE COMME UN IDENTIFIANT INEXISTANT
//!
//! La garde tombe **avant toute lecture** : un administrateur d'événement qui
//! forge l'adresse d'un code reçoit 403 sans que la route ait touché la base.
//! Et pour qui passe la garde, un code absent rend 404 — la forme des deux
//! réponses ne dit rien de l'existence du code (principe IX).

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::error::{ApiError, Result};
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::admin::{CreateInvitationCodePayload, ReasonPayload};
use crate::domain::permissions::SpaceManage;
use crate::repo::codes::{Filtre, LISTE_LIMITE_DEFAUT, LISTE_LIMITE_MAX};
use crate::service::admin_codes;
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/invitation-codes", web::get().to(lister))
        .route("/admin/negotiation/invitation-codes", web::post().to(creer))
        .route(
            "/admin/negotiation/invitation-codes/{id}",
            web::get().to(fiche),
        )
        .route(
            "/admin/negotiation/invitation-codes/{id}/revoke",
            web::post().to(revoquer),
        )
        .route(
            "/admin/negotiation/invitation-codes/{id}/uses",
            web::get().to(usages),
        )
        .route(
            "/admin/negotiation/invitation-codes/{id}/uses/{person_id}/revoke-access",
            web::post().to(retirer_un_acces),
        )
        .route(
            "/admin/negotiation/invitation-codes/{id}/revoke-all-access",
            web::post().to(retirer_tous_les_acces),
        );
}

/// Les filtres, **nommés en français** : ils apparaissent dans une adresse
/// qu'on partage entre collègues.
#[derive(Debug, Deserialize)]
pub struct ListeQuery {
    /// `active`, `revoked`, `expired`, `not_yet_valid` ou `exhausted`.
    pub etat: Option<String>,
    /// Un identifiant d'espace, ou le mot `global` pour les codes qui ouvrent
    /// Guide Négo en entier.
    pub espace: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[utoipa::path(
    get,
    description = "`InvitationCodeListScreen` — la liste des codes d'invitation, avec les espaces et les réseaux dont le filtre et le formulaire de création ont besoin.\n\nL'état de chaque code — `active`, `revoked`, `expired`, `not_yet_valid`, `exhausted` — est **dérivé par `negotiation.v_invitation_codes`**, la même expression qui refuse un code à l'application : deux calculs séparés divergeraient au premier oubli de `valid_from`.\n\n**Le code est rendu en clair** : c'est un secret partagé, recopié à la main depuis WhatsApp, pas un secret nominatif (FR-037).\n\nFiltres : `etat`, `espace` (un identifiant d'espace ou le mot `global`), `q` (cherche dans le libellé **et** dans le code sous sa forme normalisée).",
    path = "/admin/negotiation/invitation-codes",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_codes_lister",
    params(
        ("etat" = Option<String>, Query, description = "État dérivé : active, revoked, expired, not_yet_valid, exhausted"),
        ("espace" = Option<String>, Query, description = "Identifiant d'espace, ou « global »"),
        ("q" = Option<String>, Query, description = "Cherche dans le libellé et dans le code"),
        ("limit" = Option<i64>, Query, description = "Défaut 25, maximum 100"),
        ("offset" = Option<i64>, Query, description = "Décalage"),
    ),
    responses(
        (status = 200, description = "InvitationCodeListScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans `negotiation.space.manage` **sur la portée globale** — un administrateur d'une seule édition en fait partie", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    _garde: Requires<SpaceManage>,
    query: web::Query<ListeQuery>,
) -> Result<HttpResponse> {
    let (global_seulement, space_id) = portee_filtree(query.espace.as_deref())?;

    let filtre = Filtre {
        etat: query.etat.as_deref().filter(|e| !e.is_empty()),
        global_seulement,
        space_id,
        q: query.q.as_deref().filter(|q| !q.trim().is_empty()),
        limit: query
            .limit
            .unwrap_or(LISTE_LIMITE_DEFAUT)
            .clamp(1, LISTE_LIMITE_MAX),
        offset: query.offset.unwrap_or(0).max(0),
    };

    let ecran = admin_codes::liste(&state, &filtre, &crate::routes::locale_de(&requete)).await?;
    Ok(HttpResponse::Ok().json(ecran))
}

#[utoipa::path(
    post,
    description = "`CreateInvitationCodePayload` → `InvitationCodeRow` — crée un code.\n\n**Le code est engendré par l'API**, jamais choisi : huit caractères tirets compris, sur un alphabet sans `0/O` ni `1/I/L`, parce qu'il se recopie à l'œil. Laisser un administrateur l'écrire produirait des codes devinables sur une porte que rien d'autre ne protège.\n\nLa portée ne peut valoir que `global` — Guide Négo en entier — ou `negotiation_space` avec son identifiant : les deux `allowed_scopes` du rôle `negotiator`. La base le refuse deux fois, par `ck_invitation_codes_scope` puis par `tg_check_role_scope` à l'attribution.\n\nLe code est rendu en clair, pour être diffusé aussitôt.",
    path = "/admin/negotiation/invitation-codes",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_codes_creer",
    request_body = Object,
    responses(
        (status = 201, description = "InvitationCodeRow", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Espace de négociation inexistant", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Libellé vide, quota nul, période inversée, réseau inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    charge: web::Json<CreateInvitationCodePayload>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let code = admin_codes::creer(
        &state,
        &contexte,
        garde.person_id,
        &charge,
        &contexte.locale,
    )
    .await?;

    Ok(HttpResponse::Created().json(code))
}

#[utoipa::path(
    get,
    description = "`InvitationCodeDetail` — la fiche d'un code, code en clair compris, avec le nombre d'accès **encore ouverts** qu'il a accordés.\n\nCe nombre est ce qu'un administrateur doit lire **avant** de révoquer : la révocation n'en retire aucun (ADR-006), et c'est un second geste.",
    path = "/admin/negotiation/invitation-codes/{id}",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_codes_fiche",
    params(("id" = Uuid, Path, description = "Code d'invitation")),
    responses(
        (status = 200, description = "InvitationCodeDetail", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Code inexistant", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn fiche(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    _garde: Requires<SpaceManage>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let fiche = admin_codes::fiche(
        &state,
        chemin.into_inner(),
        &crate::routes::locale_de(&requete),
    )
    .await?
    .ok_or_else(ApiError::not_found)?;

    Ok(HttpResponse::Ok().json(fiche))
}

#[utoipa::path(
    post,
    description = "`ReasonPayload` → `InvitationCodeRow` — révoque un code.\n\n**Ne retire aucun accès déjà accordé** (ADR-006, FR-039) : le code cesse d'ouvrir dès la tentative suivante, et le réseau déjà entré garde le sien. Retirer les accès est un second geste, et une autre route.\n\nLa date et l'auteur sont conservés. Révoquer deux fois ne réécrit ni l'une ni l'autre : la fiche revient telle quelle.\n\nUn code révoqué **reste retrouvable** par sa forme normalisée : c'est ce qui permet de répondre « révoqué le 8 novembre » au lieu de « code inconnu », et de ne pas envoyer la personne chercher une faute de frappe qu'elle n'a pas faite.",
    path = "/admin/negotiation/invitation-codes/{id}/revoke",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_codes_revoquer",
    params(("id" = Uuid, Path, description = "Code d'invitation")),
    request_body = Object,
    responses(
        (status = 200, description = "InvitationCodeRow", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Code inexistant", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn revoquer(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    chemin: web::Path<Uuid>,
    charge: Option<web::Json<ReasonPayload>>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let motif = motif_de(&charge);

    let fiche = admin_codes::revoquer(
        &state,
        &contexte,
        garde.person_id,
        chemin.into_inner(),
        motif.as_deref(),
        &contexte.locale,
    )
    .await?
    .ok_or_else(ApiError::not_found)?;

    Ok(HttpResponse::Ok().json(fiche))
}

#[utoipa::path(
    get,
    description = "`InvitationCodeUsesScreen` — qui est entré avec ce code, quand, et si son accès tient encore.\n\nL'état de l'accès est **lu dans `identity.role_assignments`** par `negotiation.v_invitation_code_uses` : la table des usages n'en porte aucune copie, parce que deux vérités divergent toujours un jour.\n\nLes accès encore ouverts viennent en tête.",
    path = "/admin/negotiation/invitation-codes/{id}/uses",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_codes_usages",
    params(
        ("id" = Uuid, Path, description = "Code d'invitation"),
        ("limit" = Option<i64>, Query, description = "Défaut 25, maximum 100"),
        ("offset" = Option<i64>, Query, description = "Décalage"),
    ),
    responses(
        (status = 200, description = "InvitationCodeUsesScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Code inexistant", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn usages(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    _garde: Requires<SpaceManage>,
    chemin: web::Path<Uuid>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse> {
    let code_id = chemin.into_inner();

    // La fiche d'abord : sans elle, un identifiant inexistant rendrait une liste
    // vide, et l'écran afficherait « personne n'est entré » pour un code qui
    // n'existe pas.
    admin_codes::fiche(&state, code_id, &crate::routes::locale_de(&requete))
        .await?
        .ok_or_else(ApiError::not_found)?;

    let ecran = admin_codes::usages(
        &state,
        code_id,
        query
            .limit
            .unwrap_or(LISTE_LIMITE_DEFAUT)
            .clamp(1, LISTE_LIMITE_MAX),
        query.offset.unwrap_or(0).max(0),
    )
    .await?;

    Ok(HttpResponse::Ok().json(ecran))
}

#[utoipa::path(
    post,
    description = "`ReasonPayload` → `RevokeAllAccessResult` — retire l'accès d'**une** personne entrée par ce code.\n\nL'attribution retirée est celle de la portée du code, et d'elle seule : une personne admise sur la COP31 par ce code et sur Guide Négo entier par un autre ne perd que le premier.\n\nL'accès cesse d'ouvrir **dès la lecture suivante** de `me/access` (FR-041), et « Mon accès » le dit. `revoked` vaut 0 si elle n'avait pas d'accès en cours : deux administrateurs peuvent agir à la seconde près, et ce n'est pas une erreur.",
    path = "/admin/negotiation/invitation-codes/{id}/uses/{person_id}/revoke-access",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_codes_retirer_un_acces",
    params(
        ("id" = Uuid, Path, description = "Code d'invitation"),
        ("person_id" = Uuid, Path, description = "Personne dont l'accès est retiré"),
    ),
    request_body = Object,
    responses(
        (status = 200, description = "RevokeAllAccessResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn retirer_un_acces(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    chemin: web::Path<(Uuid, Uuid)>,
    charge: Option<web::Json<ReasonPayload>>,
) -> Result<HttpResponse> {
    let (code_id, person_id) = chemin.into_inner();
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let motif = motif_de(&charge);

    let retire = admin_codes::retirer_un_acces(
        &state,
        &contexte,
        garde.person_id,
        code_id,
        person_id,
        motif.as_deref(),
    )
    .await?;

    Ok(
        HttpResponse::Ok().json(crate::domain::admin::RevokeAllAccessResult {
            revoked: i64::from(retire),
        }),
    )
}

#[utoipa::path(
    post,
    description = "`ReasonPayload` → `RevokeAllAccessResult` — retire en un geste les accès de **toutes** les personnes entrées par ce code.\n\nC'est le geste d'un code compromis, et il **suit** la révocation sans la remplacer : révoquer ferme la porte, retirer sort ceux qui sont déjà entrés. L'écran demande confirmation.\n\n`revoked` compte les accès qui viennent réellement de tomber : ceux déjà retirés ne sont pas touchés.",
    path = "/admin/negotiation/invitation-codes/{id}/revoke-all-access",
    tag = "Back-office — admission",
    operation_id = "admin_negotiation_codes_retirer_tous_les_acces",
    params(("id" = Uuid, Path, description = "Code d'invitation")),
    request_body = Object,
    responses(
        (status = 200, description = "RevokeAllAccessResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission sur la portée globale", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn retirer_tous_les_acces(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    garde: Requires<SpaceManage>,
    chemin: web::Path<Uuid>,
    charge: Option<web::Json<ReasonPayload>>,
) -> Result<HttpResponse> {
    let contexte = crate::routes::contexte_de(&requete, garde.person_id);
    let motif = motif_de(&charge);

    let resultat = admin_codes::retirer_tous_les_acces(
        &state,
        &contexte,
        garde.person_id,
        chemin.into_inner(),
        motif.as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Le filtre de portée : un identifiant d'espace, ou le mot `global`.
fn portee_filtree(espace: Option<&str>) -> Result<(bool, Option<Uuid>)> {
    match espace.map(str::trim).filter(|e| !e.is_empty()) {
        None => Ok((false, None)),
        Some("global") => Ok((true, None)),
        Some(brut) => {
            let id = Uuid::parse_str(brut).map_err(|_| {
                ApiError::validation(
                    "Le filtre d'espace attend un identifiant d'espace, ou le mot « global ».",
                    "espace",
                )
            })?;
            Ok((false, Some(id)))
        }
    }
}

/// Le motif, quand il y en a un. **Un corps absent est une révocation sans
/// motif**, pas une requête malformée : la confirmation à l'écran n'oblige
/// personne à écrire.
fn motif_de(charge: &Option<web::Json<ReasonPayload>>) -> Option<String> {
    charge
        .as_ref()
        .and_then(|c| c.reason.as_deref())
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned)
}
