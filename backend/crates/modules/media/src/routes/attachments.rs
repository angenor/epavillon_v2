//! Les quatre routes du rattachement.
//!
//! L'entité porteuse voyage en **paramètres de requête** plutôt que dans le
//! chemin : le rattachement est polymorphe, et un chemin
//! `/media/{schema}/{table}/{id}/attachments` donnerait à croire que chaque
//! entité a son espace. C'est une seule table, gardée par une seule table de
//! gardes.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::attachment::{AttachmentBatch, AttachmentPayload, DetachmentResult};
use crate::routes::contexte_de;
use crate::service::attach;
use crate::service::authz::Porteuse;
use crate::state::MediaState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/attachments", web::get().to(lister))
        .route("/attachments", web::post().to(poser))
        .route("/attachments", web::put().to(remplacer))
        .route("/attachments/{id}", web::delete().to(detacher));
}

/// L'entité visée, telle qu'une requête la nomme.
#[derive(Debug, Deserialize)]
pub struct CibleQuery {
    pub owner_schema: String,
    pub owner_table: String,
    pub owner_id: Uuid,
    /// Facultatif : sans lui, tous les rôles.
    pub role: Option<String>,
}

/// Les médias d'une entité.
#[utoipa::path(
    get,
    description = "`AttachedImage[]` **+** `attachment_id`, `role`, `sort_order` et `status` — trois champs sans lesquels l'écran qui **gère** les médias d'une entité ne peut rien faire : on ne sait pas quoi détacher, ni où ranger la ligne, ni comment réordonner une galerie.\n\n**Cette lecture ne masque pas les objets encore en traitement**, contrairement aux lectures publiques : un fichier déposé il y a trois secondes est parfaitement valide et pas encore servable, et le masquer ferait croire que le téléversement a échoué. `sources` y est alors **vide mais présent**, et `url` porte déjà l'original. Un objet **supprimé**, lui, n'est pas rendu.\n\nOrdonné par rôle puis par ordre de tri déclaré. `role` facultatif : sans lui, tous les rôles.\n\n**La garde est celle de l'écriture** : ce que l'on peut changer, on peut le lire. Les pages publiques, elles, lisent par les routes de leur module.",
    path = "/media/attachments",
    tag = "Média — rattachements",
    operation_id = "media_rattachements",
    params(
        ("owner_schema" = String, Query, description = "Schéma de l'entité porteuse"),
        ("owner_table" = String, Query, description = "Table de l'entité porteuse"),
        ("owner_id" = Uuid, Query, description = "Identifiant de l'entité porteuse"),
        ("role" = Option<String>, Query, description = "Rôle, facultatif"),
    ),
    responses(
        (status = 200, description = "AttachedMedia[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entité inexistante, hors périmètre, ou sans garde déclarée", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
    cible: web::Query<CibleQuery>,
) -> Result<HttpResponse> {
    let medias = attach::lister(
        &state,
        acteur,
        Porteuse {
            owner_schema: &cible.owner_schema,
            owner_table: &cible.owner_table,
            owner_id: cible.owner_id,
        },
        cible.role.as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(medias))
}

/// Ajouter un objet à un rôle.
#[utoipa::path(
    post,
    description = "`AttachedMedia` — ajoute un objet à un rôle **multiple**. Sur un rôle exclusif déjà pourvu, le refus est explicite : c'est un remplacement qu'il faut demander, et `PUT /media/attachments` le fait.\n\n**Les quatre contrôles de forme tombent AVANT l'écriture** — type, poids, cadrage, servabilité — non pour remplacer `tg_validate_attachment`, qui garde le dernier mot, mais pour savoir lequel de ses cinq refus nommer : il les lève sans nom de contrainte, et trois partagent le même état d'erreur.\n\n**Le refus de forme cite ses quatre nombres** : dimensions reçues, rapport obtenu, rapport attendu, tolérance. « Les dimensions ne correspondent pas » n'apprend rien à qui doit recadrer.\n\n`alt_text_override` vit sur le **rattachement** et ne touche jamais l'objet : un même fichier sert plusieurs fiches, et le texte pertinent n'y est pas le même.",
    path = "/media/attachments",
    tag = "Média — rattachements",
    operation_id = "media_rattacher",
    request_body = AttachmentPayload,
    responses(
        (status = 201, description = "AttachedMedia", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entité ou objet inexistant, ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "MEDIA_ROLE_EXCLUSIVE — le rôle n'accepte qu'un seul objet", body = crate::routes::openapi::ApiErrorBody),
        (status = 413, description = "MEDIA_TOO_LARGE — poids dépassé pour ce rôle", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "MEDIA_ROLE_NOT_DECLARED · MEDIA_MIME_NOT_ALLOWED · MEDIA_ASPECT_RATIO · MEDIA_ASSET_NOT_SERVABLE", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn poser(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    payload: web::Json<AttachmentPayload>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let pose = attach::poser(&state, &ctx, acteur, &payload).await?;
    Ok(HttpResponse::Created().json(pose))
}

/// L'écriture de remplacement, en lot.
#[utoipa::path(
    put,
    description = "`AttachedMedia[]` — **une liste d'affectations, appliquées en UNE transaction.** C'est elle que le formulaire d'édition appelle pour ses trois déclinaisons, et c'est ce qui referme l'obligation laissée par B3 : le rattachement s'écrit dans `media.attachments`, sans qu'une ligne du module Événements change.\n\n**Chaque rôle nommé est vidé puis regarni**, dans l'ordre où ses affectations apparaissent. Un rôle **absent de la liste n'est pas touché**, et `asset_id: null` vide le sien **sans toucher aux autres**.\n\nCe même mécanisme réordonne une galerie : renvoyer la même liste dans un autre ordre suffit, et aucune route de réordonnancement n'a besoin d'exister.\n\n**La transaction unique n'est pas un confort** : trois images enregistrées à moitié laisseraient une édition avec un bandeau neuf et une vignette ancienne, sans que rien ne le signale.",
    path = "/media/attachments",
    tag = "Média — rattachements",
    operation_id = "media_remplacer_rattachements",
    request_body = AttachmentBatch,
    responses(
        (status = 200, description = "AttachedMedia[] — tous les médias de l'entité après l'écriture", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entité ou objet inexistant, ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "MEDIA_ROLE_EXCLUSIVE — deux objets demandés pour un rôle qui n'en accepte qu'un", body = crate::routes::openapi::ApiErrorBody),
        (status = 413, description = "MEDIA_TOO_LARGE", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "MEDIA_ROLE_NOT_DECLARED · MEDIA_MIME_NOT_ALLOWED · MEDIA_ASPECT_RATIO · MEDIA_ASSET_NOT_SERVABLE", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn remplacer(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    lot: web::Json<AttachmentBatch>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    let medias = attach::remplacer(&state, &ctx, acteur, &lot).await?;
    Ok(HttpResponse::Ok().json(medias))
}

/// Détacher.
#[utoipa::path(
    delete,
    description = "`{ asset_kept: true }` — retire le **rattachement**. **L'objet stocké demeure**, et le champ le dit parce que c'est la question qu'on se pose en lisant la réponse : un même fichier illustre souvent plusieurs entités, la déduplication le garantissant.\n\nPour supprimer réellement un objet, `DELETE /media/assets/{id}` — qui refuse tant qu'il est encore rattaché.",
    path = "/media/attachments/{id}",
    tag = "Média — rattachements",
    operation_id = "media_detacher",
    params(("id" = Uuid, Path, description = "Identifiant du rattachement, pas de l'objet")),
    responses(
        (status = 200, description = "{ asset_kept }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Rattachement inexistant, ou entité hors périmètre", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn detacher(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
    requete: HttpRequest,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = contexte_de(&requete, acteur);
    attach::detacher(&state, &ctx, acteur, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(DetachmentResult { asset_kept: true }))
}
