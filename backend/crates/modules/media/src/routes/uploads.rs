//! **Les deux routes du dépôt** — dont la seule de l'API qui ne parle pas JSON.
//!
//! # Pourquoi les métadonnées précèdent le fichier
//!
//! Un corps composite se lit dans l'ordre où il a été écrit. Placer les
//! métadonnées avant le fichier permet de refuser un type, un poids ou un droit
//! **sans avoir lu un octet** — sur un fond vidéo de deux cents mégaoctets, la
//! différence n'est pas théorique. C'est écrit dans le contrat des routes, et
//! c'est ce que l'écran doit respecter.
//!
//! Un fichier reçu **avant** ses métadonnées n'est donc pas une variante
//! tolérée : il est refusé, en le disant.
//!
//! # La limite de corps JSON de l'API ne s'applique pas ici
//!
//! `LIMITE_CORPS` vaut un mégaoctet, et son commentaire dit déjà pourquoi :
//! « un envoi de fichier ne passera jamais par du JSON de toute façon ». Le
//! plafond du dépôt est `MEDIA_MAX_UPLOAD_BYTES`, appliqué **au fil du flux**
//! par la mesure — pas à l'arrivée, quand les octets sont déjà sur le disque.

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use kernel::auth::Actor;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::routes::contexte_de;
use crate::service::stream;
use crate::service::upload::{self, MetadonneesDepot, UploadDeclaration};
use crate::state::MediaState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/assets/precheck", web::post().to(annoncer))
        .route("/assets", web::post().to(deposer));
}

/// L'annonce préalable — **elle n'écrit rien**.
#[utoipa::path(
    post,
    description = "`UploadVerdict` — ce que le dépôt ferait de ce fichier, **sans qu'un octet soit envoyé**. Accepté, refusé pour son type, refusé pour son poids, refusé faute d'espace, ou **l'objet existant** si une empreinte est fournie et déjà connue. \n\n**Tous les refus sortent en 200** : une annonce est une question, pas une tentative, et un refus y est une réponse. Le seul refus qui sorte en erreur est celui du **droit d'écrire sur l'entité visée** — il ne se distingue pas d'une entité inexistante, et n'a donc rien de plus à dire.\n\n**Rien n'est réservé** : ni espace, ni clé, ni identifiant. Sans envoi qui suive, il ne reste aucune trace.",
    path = "/media/assets/precheck",
    tag = "Média — dépôt",
    operation_id = "media_annoncer",
    request_body = UploadDeclaration,
    responses(
        (status = 200, description = "UploadVerdict", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entité porteuse inexistante, hors périmètre, ou dont le rôle n'a pas de garde", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn annoncer(
    state: web::Data<MediaState>,
    Actor(acteur): Actor,
    declaration: web::Json<UploadDeclaration>,
) -> Result<HttpResponse> {
    let verdict = upload::annoncer(&state, acteur, &declaration).await?;
    Ok(HttpResponse::Ok().json(verdict))
}

/// Le dépôt — métadonnées puis fichier, en un seul geste.
#[utoipa::path(
    post,
    description = "`Asset` — le dépôt. Corps **composite** (`multipart/form-data`), seule route de l'API qui ne parle pas JSON.\n\n**Les champs de métadonnées précèdent le fichier**, ce qui permet de refuser un type, un poids ou un droit sans avoir lu un octet. Champs acceptés : `filename`, `mime_type`, `byte_size`, `owner_schema`, `owner_table`, `owner_id`, `role`, `alt_text`, `caption`, `credit`, `license_code`, `visibility` ; puis la partie `file`.\n\n**L'empreinte est calculée pendant la réception**, jamais reçue du client sans être recalculée. Si le contenu est déjà connu du dépôt de stockage, **aucun second objet n'est écrit** et l'objet existant est rendu — c'est le succès de la déduplication, et la réponse porte alors `deduplicated: true`.\n\n**Le texte alternatif est exigé pour une image**, avant lecture : la base interdit à une image d'être servie sans lui, et accepter le dépôt produirait un objet bloqué en traitement pour toujours.",
    path = "/media/assets",
    tag = "Média — dépôt",
    operation_id = "media_deposer",
    responses(
        (status = 201, description = "Asset, avec `deduplicated`", body = Object),
        (status = 400, description = "MEDIA_UPLOAD_INCOMPLETE — flux rompu, ou poids reçu différent du poids annoncé", body = crate::routes::openapi::ApiErrorBody),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Entité porteuse inexistante ou hors périmètre", body = crate::routes::openapi::ApiErrorBody),
        (status = 413, description = "MEDIA_TOO_LARGE — plafond du rôle, ou plafond absolu du dépôt", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "MEDIA_ALT_TEXT_REQUIRED · MEDIA_MIME_NOT_ALLOWED · MEDIA_QUOTA_EXCEEDED · MEDIA_ROLE_NOT_DECLARED", body = crate::routes::openapi::ApiErrorBody),
        (status = 503, description = "MEDIA_STORAGE_UNAVAILABLE — le stockage n'a pas répondu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn deposer(
    state: web::Data<MediaState>,
    requete: HttpRequest,
    Actor(acteur): Actor,
    mut corps: Multipart,
) -> Result<HttpResponse> {
    let mut metadonnees = MetadonneesDepot::default();

    while let Some(partie) = corps.next().await {
        let partie = partie.map_err(refus_de_corps)?;
        let nom = partie
            .content_disposition()
            .and_then(|d| d.get_name())
            .unwrap_or_default()
            .to_owned();

        if nom == "file" {
            // Le nom d'origine et le type viennent du corps composite quand les
            // champs ne les ont pas déclarés : c'est le navigateur qui les
            // connaît le mieux.
            completer_depuis_la_partie(&mut metadonnees, &partie);
            if metadonnees.filename.is_empty() || metadonnees.mime_type.is_empty() {
                return Err(ApiError::validation(
                    "Le fichier doit être précédé de son nom et de son type.",
                    "file",
                ));
            }

            let ctx = contexte_de(&requete, acteur);
            let resultat =
                pomper(state.clone().into_inner(), ctx, acteur, metadonnees, partie).await?;
            let mut corps = serde_json::to_value(&resultat.asset).map_err(ApiError::internal)?;
            if let Some(objet) = corps.as_object_mut() {
                objet.insert(
                    "deduplicated".to_owned(),
                    serde_json::Value::Bool(resultat.deduplique),
                );
            }
            return Ok(HttpResponse::Created().json(corps));
        }

        lire_le_champ(&mut metadonnees, &nom, partie).await?;
    }

    // Aucune partie `file` : le corps s'est arrêté avant le fichier.
    Err(ApiError::new(ErrorCode::MediaUploadIncomplete).field("file"))
}

/// **Le pont entre le corps composite et le service.**
///
/// # Pourquoi il existe
///
/// `actix_multipart::Field` n'est pas `Send` : Actix fait tourner chaque ouvrier
/// sur son propre fil, sans jamais déplacer une requête d'un fil à l'autre, et
/// le corps composite s'en tient à des compteurs non partagés. Le service, lui,
/// doit être `Send` — il traverse le contrat de stockage, qui parle à `reqwest`
/// et à `tokio::fs`, et le worker le tiendra un jour sur un autre fil.
///
/// La lecture du corps reste donc **ici**, sur le fil de la requête, et pousse
/// ses tranches dans un canal ; le dépôt tourne sur une tâche, et lit ce canal.
///
/// # Le canal est BORNÉ, et c'est ce qui tient la promesse
///
/// Quatre tranches d'avance. Si le stockage traîne, la lecture du corps
/// s'arrête : la mémoire reste celle de quelques tranches, quel que soit le
/// poids du fichier. Un canal non borné rendrait le flux inutile — les deux
/// cents mégaoctets s'accumuleraient dans la file au lieu du tampon.
async fn pomper(
    state: std::sync::Arc<MediaState>,
    ctx: kernel::RequestContext,
    acteur: Uuid,
    metadonnees: MetadonneesDepot,
    mut partie: actix_multipart::Field,
) -> Result<upload::ResultatDepot> {
    use crate::storage::StorageError;

    let (envoi, reception) =
        tokio::sync::mpsc::channel::<std::result::Result<actix_web::web::Bytes, StorageError>>(4);

    let flux: crate::storage::FluxOctets = Box::pin(futures_util::stream::unfold(
        reception,
        |mut reception| async move { reception.recv().await.map(|t| (t, reception)) },
    ));

    let tache =
        tokio::spawn(async move { upload::deposer(&state, &ctx, acteur, metadonnees, flux).await });

    while let Some(tranche) = partie.next().await {
        let tranche = tranche.map_err(|e| StorageError::Unavailable(e.to_string()));
        // Le service a rendu la main — refus de droit, plafond dépassé, panne du
        // stockage. Inutile de continuer à lire : c'est son erreur qui sort.
        if envoi.send(tranche).await.is_err() {
            break;
        }
    }
    drop(envoi);

    tache
        .await
        .map_err(|e| ApiError::internal(format!("dépôt interrompu : {e}")))?
}

/// Un champ de métadonnées. **Plafonné**, parce qu'un champ de texte n'a aucune
/// raison de peser : sans cela, un `credit` de deux cents mégaoctets passerait
/// par le même chemin qu'un fichier, sans en porter le plafond.
const CHAMP_MAX: usize = 64 * 1024;

async fn lire_le_champ(
    metadonnees: &mut MetadonneesDepot,
    nom: &str,
    mut partie: actix_multipart::Field,
) -> Result<()> {
    let mut valeur = Vec::new();
    while let Some(tranche) = partie.next().await {
        let tranche = tranche.map_err(refus_de_corps)?;
        valeur.extend_from_slice(&tranche);
        if valeur.len() > CHAMP_MAX {
            return Err(ApiError::validation(
                format!("Le champ « {nom} » dépasse la taille autorisée."),
                nom,
            ));
        }
    }

    let texte = String::from_utf8(valeur).map_err(|_| {
        ApiError::validation(format!("Le champ « {nom} » n'est pas du texte."), nom)
    })?;
    let texte = texte.trim().to_owned();
    if texte.is_empty() {
        return Ok(());
    }

    match nom {
        "filename" => metadonnees.filename = texte,
        "mime_type" => metadonnees.mime_type = texte,
        "byte_size" => metadonnees.byte_size = Some(nombre(&texte, nom)?),
        "owner_schema" => metadonnees.owner_schema = Some(texte),
        "owner_table" => metadonnees.owner_table = Some(texte),
        "owner_id" => metadonnees.owner_id = Some(identifiant(&texte, nom)?),
        "role" => metadonnees.role = Some(texte),
        "alt_text" => metadonnees.alt_text = Some(texte_multilingue(&texte, nom)?),
        "caption" => metadonnees.caption = Some(texte_multilingue(&texte, nom)?),
        "credit" => metadonnees.credit = Some(texte),
        "license_code" => metadonnees.license_code = Some(texte),
        "visibility" => metadonnees.visibility = Some(texte),
        // Un champ inconnu est **ignoré**, jamais refusé : un formulaire ajoute
        // parfois un jeton de protection ou un champ d'interface, et le refuser
        // casserait l'écran pour une raison qui ne le regarde pas.
        _ => {}
    }
    Ok(())
}

fn completer_depuis_la_partie(metadonnees: &mut MetadonneesDepot, partie: &actix_multipart::Field) {
    if metadonnees.filename.is_empty() {
        if let Some(nom) = partie.content_disposition().and_then(|d| d.get_filename()) {
            metadonnees.filename = nom.to_owned();
        }
    }
    if metadonnees.mime_type.is_empty() {
        metadonnees.mime_type = partie
            .content_type()
            .map(|t| t.to_string())
            .unwrap_or_default();
    }
}

/// Le texte alternatif est une **donnée multilingue**, pas une chaîne : un objet
/// servi en anglais doit porter sa description anglaise. La forme attendue est
/// celle de `platform.i18n_text` ; une chaîne nue est acceptée et rangée en
/// français, parce qu'un formulaire simple n'a qu'un champ.
fn texte_multilingue(texte: &str, champ: &str) -> Result<serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(texte) {
        Ok(valeur) if valeur.is_object() => Ok(valeur),
        _ => Ok(serde_json::json!({ "fr": texte })),
    }
    .map_err(|e: ApiError| e.field(champ))
}

fn nombre(texte: &str, champ: &str) -> Result<i64> {
    texte
        .parse()
        .map_err(|_| ApiError::validation(format!("Le champ « {champ} » attend un nombre."), champ))
}

fn identifiant(texte: &str, champ: &str) -> Result<Uuid> {
    Uuid::parse_str(texte).map_err(|_| {
        ApiError::validation(
            format!("Le champ « {champ} » attend un identifiant."),
            champ,
        )
    })
}

/// Un corps composite illisible est une **erreur de requête**, pas une panne :
/// le texte d'`actix-multipart` est anglais et nomme des entêtes, il ne franchit
/// donc pas la réponse.
fn refus_de_corps(erreur: actix_multipart::MultipartError) -> ApiError {
    ApiError::new(ErrorCode::MediaUploadIncomplete)
        .field("file")
        .detail(erreur)
}

/// Réexport, pour que le service et les routes parlent du même flux vide.
pub use stream::flux_vide;
