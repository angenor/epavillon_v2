//! Les lectures ouvertes du référentiel.
//!
//! **Session requise, et rien de plus** (FR-014). La permission de consultation
//! des organisations est détenue par le rôle d'utilisateur ordinaire — que rien
//! n'attribue aujourd'hui (écart n° 74) : l'exiger ici refuserait tout nouvel
//! inscrit, c'est-à-dire exactement les personnes que l'écran de rattachement
//! sert. Le jour où l'inscription attribuera le rôle, l'exigence pourra être
//! resserrée sans changer le contrat.

use actix_web::{web, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::{OrganizationId, PersonId};
use crate::domain::organization::CreateOrganization;
use crate::repo::domains;
use crate::repo::organizations::{self, LIST_DEFAULT_LIMIT, LIST_MAX_LIMIT};
use crate::service::create;
use crate::service::search::{self, SearchQuery};
use crate::state::OrgState;

/// Les routes ouvertes du scope `/organizations`.
///
/// **Elles ne créent pas leur scope**, et c'est délibéré : les routes
/// d'adhésion en partagent le préfixe. Deux `web::scope("/organizations")`
/// enregistrés séparément ne se complètent pas — Actix retient le **premier**
/// qui correspond au préfixe et rend 404 si la route n'y est pas, sans essayer
/// le second. Le montage est donc unique, dans `lib.rs`.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/similar", web::get().to(similaires))
        .route("/by-email-domain", web::get().to(par_domaine_dadresse))
        .route("", web::get().to(lister))
        .route("", web::post().to(creer))
        .route("/{id}", web::get().to(fiche));
}

#[derive(Debug, Deserialize)]
pub struct RechercheQuery {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub country_id: Option<Uuid>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub limit: Option<i32>,
}

impl From<RechercheQuery> for SearchQuery {
    fn from(q: RechercheQuery) -> Self {
        Self {
            name: q.name,
            country_id: q.country_id,
            email: q.email,
            website: q.website,
            limit: q.limit,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// **La première des deux lectures de recherche** (écart n° 23).
///
/// Celle-ci répond à « ce que j'ai tapé, est-ce que ça existe déjà ? ». Elle ne
/// rend que les fiches dont une **dénomination** ressemble au terme : le domaine
/// de l'appelant alimente le score mais ne fait entrer aucune fiche, sans quoi
/// chercher « Agence spatiale du Sahel » ramènerait l'organisation de son propre
/// domaine — qu'un bandeau lui propose déjà nommément.
///
/// L'autre lecture, `GET /admin/organizations/similar`, ne filtre rien : deux
/// fiches qui déclarent le même domaine sont la même maison, quels que soient
/// les libellés saisis.
#[utoipa::path(
    get,
    description = "`SimilarOrganization[]` — **lecture filtrée** : seules les fiches entrées par une ressemblance de dénomination. Le domaine de l'appelant alimente le score sans faire entrer une fiche sans rapport. L'autre lecture, `/admin/organizations/similar`, ne filtre rien. Un terme sous deux caractères rend une liste vide, pas une erreur.",
    path = "/organizations/similar",
    tag = "Organisations",
    operation_id = "organisations_similaires",
    params(
        ("name" = String, Query, description = "Ce qui a été tapé : nom complet, sigle, fragment"),
        ("country_id" = Option<Uuid>, Query, description = "Pays du profil — bonus de 10"),
        ("email" = Option<String>, Query, description = "Adresse : son domaine vaut 40, sauf messagerie grand public"),
        ("website" = Option<String>, Query, description = "Site saisi au formulaire — même usage"),
        ("limit" = Option<i32>, Query, description = "Défaut 10, maximum 50"),
    ),
    responses(
        (status = 200, description = "SimilarOrganization[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn similaires(
    state: web::Data<OrgState>,
    _acteur: Actor,
    query: web::Query<RechercheQuery>,
) -> Result<HttpResponse> {
    let resultats = search::similar_for_person(state.pool(), query.into_inner().into()).await?;
    Ok(HttpResponse::Ok().json(resultats))
}

/// Ce que révèle le domaine de **son** adresse.
///
/// **Le paramètre `email` du client est ignoré** (FR-017, écart n° 75) : le
/// domaine vient de la session. Le front le transmet encore parce que les
/// données simulées n'en ont pas ; il disparaîtra au raccordement. Le lire
/// permettrait à n'importe qui de demander ce que révèle l'adresse de n'importe
/// qui — c'est le motif « les droits déclarés par le client sont ignorés »,
/// éprouvé en B1.
#[utoipa::path(
    get,
    description = "`EmailDomainMatch | null`. Le paramètre `email` est **ignoré** : le domaine vient de la session. `null` sur messagerie grand public ou domaine inconnu — l'écran ne propose rien, il ne devine pas.",
    path = "/organizations/by-email-domain",
    tag = "Organisations",
    operation_id = "organisation_par_domaine",
    params(("email" = Option<String>, Query, description = "**Ignoré.** Le domaine vient de la session.")),
    responses(
        (status = 200, description = "EmailDomainMatch | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn par_domaine_dadresse(
    state: web::Data<OrgState>,
    acteur: Actor,
    _ignore: web::Query<RechercheQuery>,
) -> Result<HttpResponse> {
    let adresse = adresse_de_la_session(&state, acteur).await?;
    let revele = domains::what_email_reveals(state.pool(), &adresse).await?;
    Ok(HttpResponse::Ok().json(revele))
}

/// L'adresse de la personne connectée. C'est la seule que la route puisse
/// interroger.
async fn adresse_de_la_session(state: &OrgState, acteur: Actor) -> Result<String> {
    let adresse = sqlx::query_scalar!(
        r#"SELECT primary_email::text AS "email!" FROM identity.people WHERE id = $1"#,
        acteur.0
    )
    .fetch_optional(state.pool())
    .await?;

    adresse.ok_or_else(kernel::error::ApiError::unauthenticated)
}

/// La liste ouverte, **bornée**. Seule la page de guide de style l'appelle ;
/// elle est livrée pour ne pas la casser.
#[utoipa::path(
    get,
    description = "`Organization[]` — bornée : défaut 50, maximum 200, fiches vivantes, triées par nom légal.",
    path = "/organizations",
    tag = "Organisations",
    operation_id = "organisations_liste",
    params(
        ("limit" = Option<i64>, Query, description = "Défaut 50, maximum 200"),
        ("offset" = Option<i64>, Query, description = "Décalage"),
    ),
    responses(
        (status = 200, description = "Organization[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn lister(
    state: web::Data<OrgState>,
    _acteur: Actor,
    query: web::Query<PaginationQuery>,
) -> Result<HttpResponse> {
    let q = query.into_inner();
    let fiches = organizations::list(
        state.pool(),
        q.limit
            .unwrap_or(LIST_DEFAULT_LIMIT)
            .clamp(1, LIST_MAX_LIMIT),
        q.offset.unwrap_or(0),
    )
    .await?;

    Ok(HttpResponse::Ok().json(fiches))
}

/// Créer une organisation. **Rend 200 pour les deux issues** : une fiche créée,
/// ou le nom déjà pris — qui n'est pas une erreur mais un refus prévu par le
/// contrat, portant la fiche en cause.
#[utoipa::path(
    post,
    description = "`CreateOrganizationResult` — deux issues, toutes deux en 200. La fiche naît `candidate`, jamais `active`, et son créateur en devient référent actif. `name_taken` porte la fiche en conflit sous forme de `SimilarOrganization` : de quoi la rejoindre. **Une simple ressemblance ne bloque jamais.**",
    path = "/organizations",
    tag = "Organisations",
    operation_id = "organisation_creer",
    request_body = Object,
    responses(
        (status = 200, description = "CreateOrganizationResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Valeur refusée par le modèle — nom trop court, sigle hors bornes, pays inconnu", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer(
    state: web::Data<OrgState>,
    acteur: Actor,
    corps: web::Json<CreateOrganization>,
    requete: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let issue = create::create(
        &state,
        &crate::routes::contexte(&requete, acteur),
        PersonId(acteur.0),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// Une fiche, **telle quelle**, absorbée comprise : elle porte alors
/// `merged_into_id`, et l'appelant sait quoi en faire. Les anciennes adresses
/// continuent de mener quelque part, c'est la promesse de la fusion.
#[utoipa::path(
    get,
    description = "`Organization | null` — rendue telle quelle, absorbée comprise.",
    path = "/organizations/{id}",
    tag = "Organisations",
    operation_id = "organisation_fiche",
    params(("id" = Uuid, Path, description = "Identifiant de l'organisation")),
    responses(
        (status = 200, description = "Organization | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn fiche(
    state: web::Data<OrgState>,
    _acteur: Actor,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let fiche = organizations::by_id(state.pool(), OrganizationId(chemin.into_inner())).await?;
    Ok(HttpResponse::Ok().json(fiche))
}
