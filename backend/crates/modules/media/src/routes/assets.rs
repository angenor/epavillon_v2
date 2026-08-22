//! La description d'un objet, et sa suppression.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use uuid::Uuid;

use crate::routes::contexte_de;
use crate::service::{admin, read};
use crate::state::MediaState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    // **Le chemin littéral avant le chemin paramétré** : `/assets/{id}/status`
    // porte un segment de plus que `/assets/{id}`, mais l'ordre est tenu par la
    // structure plutôt que par la vigilance, comme dans les modules livrés.
    cfg.route("/assets/{id}/status", web::get().to(avancement));
    cfg.route("/assets/{id}", web::get().to(objet));
    cfg.route("/assets/{id}", web::delete().to(supprimer));
}

/// Un objet, avec l'adresse composée de son original et ses déclinaisons prêtes.
#[utoipa::path(
    get,
    description = "`Asset` **+** `url` et `sources`. L'adresse est **composée** par la base depuis le point d'accès courant : aucune clé de stockage nue ne sort d'ici, et c'est ce qui rend une migration de fournisseur indolore.\n\n**Un objet non servable n'est pas absent** : en traitement, en échec ou en quarantaine, il est rendu **en 200 avec son état** — « en cours » et « en échec » se lisent tous les deux « pas encore là », et les distinguer demande que l'API le dise. `sources` est alors un objet **vide mais présent**, et `url` porte déjà l'original : l'écran affiche l'image, pas un trou.\n\nSeule la suppression rend 404.",
    path = "/media/assets/{id}",
    tag = "Média — dépôt",
    operation_id = "media_objet",
    params(("id" = Uuid, Path, description = "Identifiant de l'objet")),
    responses(
        (status = 200, description = "Asset", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Objet inexistant ou supprimé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn objet(
    state: web::Data<MediaState>,
    _acteur: Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(read::objet(&state, id.into_inner()).await?))
}

/// L'avancement du traitement d'un objet.
#[utoipa::path(
    get,
    description = "`AssetProgress` : état, verdict d'analyse et **moteur**, dimensions relevées, déclinaisons prêtes sur déclinaisons attendues.\n\n**Sans cette route, un écran ne sait pas distinguer « en cours » de « en échec »** : les deux se lisent « pas encore là ». Un objet en échec ou en quarantaine rend donc son état ici, en **200** — il est simplement absent des lectures publiques.\n\n**Le nombre attendu se compte, il ne s'annonce pas** : une image plus petite que la plus petite taille configurée n'en produit aucune, et annoncer trois attendues laisserait l'avancement bloqué à zéro sur trois pour toujours. Tant que le relevé n'a pas eu lieu, il vaut zéro.\n\n**Le verdict `unsupported` n'est pas une absence de verdict** : c'est « aucun moteur ne sait analyser ceci », et `scan_engine` dit alors qui a répondu — `none` quand aucun moteur n'est branché.",
    path = "/media/assets/{id}/status",
    tag = "Média — dépôt",
    operation_id = "media_avancement",
    params(("id" = Uuid, Path, description = "Identifiant de l'objet")),
    responses(
        (status = 200, description = "AssetProgress", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Objet inexistant ou supprimé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn avancement(
    state: web::Data<MediaState>,
    _acteur: Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(read::avancement(&state, id.into_inner()).await?))
}

/// Supprimer un objet.
#[utoipa::path(
    delete,
    description = "`{ scheduled_purge_at }` — l'objet cesse d'être servi, la consommation baisse **immédiatement**, et il reste récupérable jusqu'à cet instant. La disparition du stockage, elle, appartient au travail récurrent de purge.\n\n**Refusée si l'objet est encore rattaché** (`MEDIA_ASSET_IN_USE`), en disant combien de fiches l'utilisent. La déduplication traverse les propriétaires : le même fichier déposé par deux organisations ne donne **qu'une** ligne, et sans ce refus la première ferait disparaître l'image de la seconde (écart n° 128).",
    path = "/media/assets/{id}",
    tag = "Média — dépôt",
    operation_id = "media_supprimer",
    params(("id" = Uuid, Path, description = "Identifiant de l'objet")),
    responses(
        (status = 200, description = "{ scheduled_purge_at }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "L'objet n'appartient pas à l'acteur", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Objet inexistant ou déjà supprimé", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "MEDIA_ASSET_IN_USE", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn supprimer(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let issue = admin::supprimer(&state, &ctx, acteur, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(issue))
}
