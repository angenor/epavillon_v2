//! Le back-office des documents et des notes de correction. **Portée globale
//! partout** : pour Guide Négo, « son périmètre » veut dire la plateforme
//! entière. Les écritures du document demandent `Requires<DocumentPublish>` ;
//! les lectures, `LectureDocuments` — publier **ou** corriger ; poser et
//! retirer une note, la permission du geste.

use std::future::Future;
use std::pin::Pin;

use actix_web::dev::Payload;
use actix_web::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE};
use actix_web::{web, FromRequest, HttpMessage, HttpRequest, HttpResponse};
use kernel::auth::Requires;
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::admin_documents::{
    AdminDocumentInput, AttachFileInput, CorrectionNoteInput, ServeAsIsInput,
};
use crate::domain::permissions::{
    CorrectionPost, CorrectionWithdraw, DocumentPublish, CORRECTION_WITHDRAW,
};
use crate::service::admin_documents::{self as service, Droits};
use crate::service::{corrections, documents};
use crate::state::NegotiationState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/negotiation/documents", web::get().to(lister))
        .route("/admin/negotiation/documents", web::post().to(creer))
        .route("/admin/negotiation/documents/{id}", web::get().to(fiche))
        .route(
            "/admin/negotiation/documents/{id}",
            web::patch().to(modifier),
        )
        .route(
            "/admin/negotiation/documents/{id}",
            web::delete().to(supprimer),
        )
        .route(
            "/admin/negotiation/documents/{id}/file",
            web::put().to(attacher_le_fichier),
        )
        .route("/admin/negotiation/documents/{id}/file", web::get().to(pdf))
        .route(
            "/admin/negotiation/documents/{id}/extraction",
            web::post().to(relancer),
        )
        .route(
            "/admin/negotiation/documents/{id}/as-is",
            web::put().to(tel_quel),
        )
        .route(
            "/admin/negotiation/documents/{id}/publish",
            web::post().to(publier),
        )
        .route(
            "/admin/negotiation/documents/{id}/unpublish",
            web::post().to(depublier),
        )
        .route(
            "/admin/negotiation/documents/{id}/new-version",
            web::post().to(nouvelle_version),
        )
        .route(
            "/admin/negotiation/documents/{id}/preview",
            web::get().to(apercu),
        )
        .route(
            "/admin/negotiation/documents/{id}/pages/{index}/image",
            web::get().to(image),
        )
        .route(
            "/admin/negotiation/documents/{id}/corrections",
            web::get().to(notes),
        )
        .route(
            "/admin/negotiation/documents/{id}/corrections",
            web::post().to(poser_une_note),
        )
        .route(
            "/admin/negotiation/corrections/{note_id}/withdraw",
            web::post().to(retirer_une_note),
        );
}

/// Lire la liste, la fiche, l'aperçu, le PDF et les images : `publish` **ou**
/// `correction.post`, sur la portée globale. L'expert doit voir ce qu'il corrige.
pub struct LectureDocuments {
    pub person_id: Uuid,
    pub droits: Droits,
}

impl FromRequest for LectureDocuments {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let acteur = req
            .extensions()
            .get::<RequestContext>()
            .and_then(|c| c.actor_id);
        let state = req.app_data::<web::Data<NegotiationState>>().cloned();
        Box::pin(async move {
            let person_id = acteur.ok_or_else(ApiError::unauthenticated)?;
            let state = state.ok_or_else(|| ApiError::internal("état du module absent"))?;
            let droits = service::exiger_la_lecture(&state, person_id).await?;
            Ok(LectureDocuments { person_id, droits })
        })
    }
}

async fn rendre_la_fiche(
    state: &NegotiationState,
    requete: &HttpRequest,
    personne: Uuid,
    id: Uuid,
) -> Result<HttpResponse> {
    let droits = service::droits(state, personne).await?;
    let locale = crate::routes::locale_de(requete);
    let fiche = service::fiche(state, &droits, id, &locale).await?;
    Ok(HttpResponse::Ok().json(fiche))
}

#[utoipa::path(
    get,
    description = "`AdminDocumentList` — tous les documents : état (brouillon, publié, dépublié), type, version, remplacement, extraction. Ouvert à qui publie ou corrige, sur la portée globale.",
    path = "/admin/negotiation/documents",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_documents",
    responses(
        (status = 200, description = "AdminDocumentList", body = Object),
        (status = 401, description = "Aucune session", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Ni publier ni corriger", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    lecteur: LectureDocuments,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let liste = service::liste(&state, &lecteur.droits, &locale).await?;
    Ok(HttpResponse::Ok().json(liste))
}

#[utoipa::path(
    post,
    description = "`AdminDocumentInput` → `AdminDocument` — crée un **brouillon**, sans source : le PDF se dépose ensuite avec lui pour propriétaire. Titre en français et type exigés.",
    path = "/admin/negotiation/documents",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_creer",
    request_body = Object,
    responses(
        (status = 201, description = "AdminDocument", body = Object),
        (status = 400, description = "Type ou thématique inconnus", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Le document remplacé l'est déjà, ou le remplacement bouclerait", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Titre manquant, ou fichier et lien à la fois", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    entree: web::Json<AdminDocumentInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    let locale = crate::routes::locale_de(&requete);
    let id = service::creer(&state, &ctx, &entree, &locale).await?;
    let droits = service::droits(&state, acteur.person_id).await?;
    let fiche = service::fiche(&state, &droits, id, &locale).await?;
    Ok(HttpResponse::Created().json(fiche))
}

#[utoipa::path(
    get,
    description = "`AdminDocument` — la fiche d'un document, textes non résolus, avec l'état de son extraction.",
    path = "/admin/negotiation/documents/{id}",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 200, description = "AdminDocument", body = Object),
        (status = 403, description = "Ni publier ni corriger", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn fiche(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    lecteur: LectureDocuments,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let locale = crate::routes::locale_de(&requete);
    let fiche = service::fiche(&state, &lecteur.droits, chemin.into_inner(), &locale).await?;
    Ok(HttpResponse::Ok().json(fiche))
}

#[utoipa::path(
    patch,
    description = "`AdminDocumentInput` partiel → `AdminDocument` : un champ absent ne change rien, `null` vide un champ facultatif. Les thématiques se remplacent en bloc. Le lien d'un document déjà publié est figé.",
    path = "/admin/negotiation/documents/{id}",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_modifier",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminDocument", body = Object),
        (status = 400, description = "Type ou thématique inconnus", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Remplacement impossible, ou source figée", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Texte ou source invalides", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifier(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AdminDocumentInput>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    let locale = crate::routes::locale_de(&requete);
    service::modifier(&state, &ctx, id, &entree, &locale).await?;
    rendre_la_fiche(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    delete,
    description = "Supprime un **brouillon jamais publié**, ses liens de thématique et ses images de page. Un document publié se dépublie.",
    path = "/admin/negotiation/documents/{id}",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_supprimer",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 204, description = "Supprimé"),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Document déjà publié", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn supprimer(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::supprimer(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    put,
    description = "`{ asset_id }` → `AdminDocument` — attache le PDF déposé avec ce document pour propriétaire, et met son extraction en file dans la même transaction. Refusé sur un document déjà publié.",
    path = "/admin/negotiation/documents/{id}/file",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_fichier",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminDocument", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Fichier figé", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Fichier inconnu, pas un PDF, ou lien déjà posé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn attacher_le_fichier(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    chemin: web::Path<Uuid>,
    entree: web::Json<AttachFileInput>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::attacher_le_fichier(&state, &ctx, id, entree.asset_id).await?;
    rendre_la_fiche(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    get,
    description = "Le PDF d'origine, lu dans le bucket privé.",
    path = "/admin/negotiation/documents/{id}/file",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_pdf",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 200, description = "application/pdf", content_type = "application/pdf"),
        (status = 403, description = "Ni publier ni corriger", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document ou fichier inconnus", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn pdf(
    state: web::Data<NegotiationState>,
    _lecteur: LectureDocuments,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let (octets, nom) = service::pdf(&state, chemin.into_inner()).await?;
    let nom = nom
        .unwrap_or_else(|| "document.pdf".to_owned())
        .replace(['"', '\\', '\r', '\n'], "");
    Ok(HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, "application/pdf"))
        .insert_header((CACHE_CONTROL, "private, no-store"))
        .insert_header((CONTENT_DISPOSITION, format!("inline; filename=\"{nom}\"")))
        .body(octets))
}

#[utoipa::path(
    post,
    description = "Relance l'extraction du fichier d'un document qui n'a pas encore été publié.",
    path = "/admin/negotiation/documents/{id}/extraction",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_extraire",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 202, description = "Extraction en file"),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Fichier figé", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Aucun fichier", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn relancer(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::relancer_lextraction(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::Accepted().finish())
}

#[utoipa::path(
    put,
    description = "`{ serve_as_is }` → `AdminDocument` — « ouvrir tel quel » : le document se lit en pages d'origine. Se change sans republier.",
    path = "/admin/negotiation/documents/{id}/as-is",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_tel_quel",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    request_body = Object,
    responses(
        (status = 200, description = "AdminDocument", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Aucun fichier extrait", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn tel_quel(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    chemin: web::Path<Uuid>,
    entree: web::Json<ServeAsIsInput>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::ouvrir_tel_quel(&state, &ctx, id, entree.serve_as_is).await?;
    rendre_la_fiche(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    post,
    description = "→ `AdminDocument` — publie. Exige une source, et pour un fichier une extraction prête. Rejouer ne change rien.",
    path = "/admin/negotiation/documents/{id}/publish",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_publier",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 200, description = "AdminDocument", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Extraction pas prête", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Ni fichier ni lien", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn publier(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::publier(&state, &ctx, id).await?;
    rendre_la_fiche(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    post,
    description = "→ `AdminDocument` — dépublie : le document quitte la bibliothèque, sa date de retrait est gardée.",
    path = "/admin/negotiation/documents/{id}/unpublish",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_depublier",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 200, description = "AdminDocument", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn depublier(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    service::depublier(&state, &ctx, id).await?;
    rendre_la_fiche(&state, &requete, acteur.person_id, id).await
}

#[utoipa::path(
    post,
    description = "→ `AdminDocument` — le brouillon d'une nouvelle version, prérempli et désigné comme remplaçant. La version reste à saisir.",
    path = "/admin/negotiation/documents/{id}/new-version",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_nouvelle_version",
    params(("id" = Uuid, Path, description = "Identifiant du document remplacé")),
    responses(
        (status = 201, description = "AdminDocument, le nouveau brouillon", body = Object),
        (status = 403, description = "Sans la permission de publier", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 409, description = "Déjà remplacé", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn nouvelle_version(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<DocumentPublish>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    let locale = crate::routes::locale_de(&requete);
    let nouveau = service::nouvelle_version(&state, &ctx, chemin.into_inner(), &locale).await?;
    let droits = service::droits(&state, acteur.person_id).await?;
    let fiche = service::fiche(&state, &droits, nouveau, &locale).await?;
    Ok(HttpResponse::Created().json(fiche))
}

#[utoipa::path(
    get,
    description = "`AdminDocumentPreview` — le verdict de l'extraction, ses indicateurs, le sommaire, et pour chaque page ses blocs et l'adresse de son image.",
    path = "/admin/negotiation/documents/{id}/preview",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_apercu",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 200, description = "AdminDocumentPreview", body = Object),
        (status = 403, description = "Ni publier ni corriger", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn apercu(
    state: web::Data<NegotiationState>,
    _lecteur: LectureDocuments,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let apercu = service::apercu(&state, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(apercu))
}

#[utoipa::path(
    get,
    description = "L'image JPEG d'une page, **brouillon compris** : la route publique refuse un document non publié.",
    path = "/admin/negotiation/documents/{id}/pages/{index}/image",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_image",
    params(
        ("id" = Uuid, Path, description = "Identifiant du document"),
        ("index" = i32, Path, description = "Page du document, à partir de 1"),
    ),
    responses(
        (status = 200, description = "image/jpeg", content_type = "image/jpeg"),
        (status = 403, description = "Ni publier ni corriger", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document ou page inconnus", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn image(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    _lecteur: LectureDocuments,
    chemin: web::Path<(Uuid, i32)>,
) -> Result<HttpResponse> {
    let (id, index) = chemin.into_inner();
    let mut image = documents::image_de_lapercu(&state, id, index).await?;
    image.reservee = true;
    Ok(crate::routes::documents::servir_image(&requete, image))
}

#[utoipa::path(
    get,
    description = "`AdminCorrectionNoteList` — les notes d'un document, vivantes **et** retirées, avec leurs auteurs et leurs dates.",
    path = "/admin/negotiation/documents/{id}/corrections",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_notes",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    responses(
        (status = 200, description = "AdminCorrectionNoteList", body = Object),
        (status = 403, description = "Ni publier ni corriger", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn notes(
    state: web::Data<NegotiationState>,
    lecteur: LectureDocuments,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let retirer = kernel::auth::has_permission(
        state.pool(),
        lecteur.person_id,
        CORRECTION_WITHDRAW,
        kernel::auth::Scope::Global,
    )
    .await?;
    let notes = corrections::du_document(
        &state,
        chemin.into_inner(),
        lecteur.droits.corriger,
        retirer,
    )
    .await?;
    Ok(HttpResponse::Ok().json(notes))
}

#[utoipa::path(
    post,
    description = "`{ page_index, passage?, body }` → `AdminCorrectionNote` — pose une note sur une page, **sans modifier le texte**. Le français est exigé. Page hors du document : **422**.",
    path = "/admin/negotiation/documents/{id}/corrections",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_document_poser_une_note",
    params(("id" = Uuid, Path, description = "Identifiant du document")),
    request_body = Object,
    responses(
        (status = 201, description = "AdminCorrectionNote", body = Object),
        (status = 403, description = "Sans la permission de poser une note", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Document inconnu", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Page inconnue, ou texte français manquant", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn poser_une_note(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<CorrectionPost>,
    chemin: web::Path<Uuid>,
    entree: web::Json<CorrectionNoteInput>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    let note = corrections::poser(&state, &ctx, chemin.into_inner(), &entree).await?;
    Ok(HttpResponse::Created().json(note))
}

#[utoipa::path(
    post,
    description = "→ `AdminCorrectionNote` — retire une note. **Idempotent** ; jamais de suppression, le retrait se date.",
    path = "/admin/negotiation/corrections/{note_id}/withdraw",
    tag = "Back-office — documents",
    operation_id = "admin_negotiation_retirer_une_note",
    params(("note_id" = Uuid, Path, description = "Identifiant de la note")),
    responses(
        (status = 200, description = "AdminCorrectionNote", body = Object),
        (status = 403, description = "Sans la permission de retirer une note", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Note inconnue", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn retirer_une_note(
    state: web::Data<NegotiationState>,
    requete: HttpRequest,
    acteur: Requires<CorrectionWithdraw>,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ctx = crate::routes::contexte_de(&requete, acteur.person_id);
    let note = corrections::retirer(&state, &ctx, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(note))
}
