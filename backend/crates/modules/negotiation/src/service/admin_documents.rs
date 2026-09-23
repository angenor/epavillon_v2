//! Le back-office des documents : brouillon, fichier, extraction, publication,
//! nouvelle version, suppression d'un brouillon, aperçu.
//!
//! **Portée globale partout.** Les écritures demandent `negotiation.document.publish`
//! ; la liste, la fiche, l'aperçu, le PDF et les images s'ouvrent aussi à
//! l'expert (`negotiation.correction.post`), qui y pose ses notes.

use kernel::auth::{has_permission, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::admin_documents::{
    etat, AdminDocument, AdminDocumentFile, AdminDocumentInput, AdminDocumentList,
    AdminDocumentPreview, AdminDocumentRow, AdminPreviewPage, DocumentLink, ExtractionState,
};
use crate::domain::permissions::{CORRECTION_POST, DOCUMENT_PUBLISH};
use crate::jobs::extract;
use crate::repo::documents::{self, Fiche, Lien, Modification, Nouveau};
use crate::repo::renditions::{self, Rendu};
use crate::repo::{document_pages, document_themes, objets, settings};
use crate::state::NegotiationState;

pub struct Droits {
    pub publier: bool,
    pub corriger: bool,
}

pub async fn droits(state: &NegotiationState, personne: Uuid) -> Result<Droits> {
    Ok(Droits {
        publier: has_permission(state.pool(), personne, DOCUMENT_PUBLISH, Scope::Global).await?,
        corriger: has_permission(state.pool(), personne, CORRECTION_POST, Scope::Global).await?,
    })
}

/// Les lectures ouvertes à qui publie **ou** corrige.
pub async fn exiger_la_lecture(state: &NegotiationState, personne: Uuid) -> Result<Droits> {
    let d = droits(state, personne).await?;
    if d.publier || d.corriger {
        Ok(d)
    } else {
        Err(ApiError::forbidden())
    }
}

fn introuvable() -> ApiError {
    ApiError::new(ErrorCode::NegotiationDocumentNotFound)
}

fn extraction(r: Option<&Rendu>, asset_id: Option<Uuid>) -> Option<ExtractionState> {
    r.filter(|r| Some(r.asset_id) == asset_id)
        .map(|r| ExtractionState {
            status: r.status.clone(),
            page_count: r.page_count,
            is_reflowable: r.is_reflowable,
            serve_as_is: r.serve_as_is,
            failure_reason: r.failure_reason.clone(),
            reading_bytes: r.reading_bytes,
            extracted_at: r.extracted_at,
        })
}

fn lien(l: Option<Lien>) -> Option<DocumentLink> {
    l.map(|l| DocumentLink {
        id: l.id,
        title: l.title,
        version: l.version,
    })
}

fn source(f: &Fiche) -> Option<&'static str> {
    match (f.asset_id, &f.external_url) {
        (Some(_), _) => Some("file"),
        (None, Some(_)) => Some("link"),
        (None, None) => None,
    }
}

pub async fn liste(
    state: &NegotiationState,
    droits: &Droits,
    locale: &str,
) -> Result<AdminDocumentList> {
    let mut conn = state.pool().acquire().await?;
    let fiches = documents::fiches(&mut conn, None, locale).await?;
    let mut lignes = Vec::with_capacity(fiches.len());
    for f in fiches {
        let rendu = renditions::lire(&mut conn, f.id).await?;
        lignes.push(AdminDocumentRow {
            id: f.id,
            title: f.title_resolu.clone(),
            type_code: f.type_code.clone(),
            version: f.version.clone(),
            state: etat(f.published_at, f.unpublished_at),
            source: source(&f),
            restricted: f.restricted,
            published_at: f.published_at,
            updated_at: f.updated_at,
            extraction: extraction(rendu.as_ref(), f.asset_id),
            supersedes: lien(f.supersedes),
            superseded_by: lien(f.superseded_by),
        });
    }
    Ok(AdminDocumentList {
        documents: lignes,
        can_publish: droits.publier,
        can_correct: droits.corriger,
    })
}

pub async fn fiche(
    state: &NegotiationState,
    droits: &Droits,
    id: Uuid,
    locale: &str,
) -> Result<AdminDocument> {
    let mut conn = state.pool().acquire().await?;
    let f = documents::fiches(&mut conn, Some(id), locale)
        .await?
        .into_iter()
        .next()
        .ok_or_else(introuvable)?;
    let themes = document_themes::codes(&mut conn, id).await?;
    let rendu = renditions::lire(&mut conn, id).await?;
    let file = match f.asset_id {
        Some(a) => objets::emplacement(&mut conn, a)
            .await?
            .map(|o| AdminDocumentFile {
                filename: o.original_filename,
                byte_size: o.byte_size,
                mime_type: o.mime_type,
            }),
        None => None,
    };
    Ok(AdminDocument {
        id: f.id,
        source: source(&f),
        state: etat(f.published_at, f.unpublished_at),
        file_locked: f.published_at.is_some() || f.unpublished_at.is_some(),
        extraction: extraction(rendu.as_ref(), f.asset_id),
        slug: f.slug,
        title: f.title,
        summary: f.summary,
        type_code: f.type_code,
        themes,
        cop: f.event_id,
        version: f.version,
        issued_on: f.issued_on,
        publisher: f.publisher,
        locale: f.locale,
        asset_id: f.asset_id,
        file,
        external_url: f.external_url,
        supersedes: lien(f.supersedes),
        superseded_by: lien(f.superseded_by),
        restricted: f.restricted,
        rag_eligible: f.rag_eligible,
        published_at: f.published_at,
        unpublished_at: f.unpublished_at,
        created_at: f.created_at,
        updated_at: f.updated_at,
        can_publish: droits.publier,
        can_correct: droits.corriger,
    })
}

/// Un texte multilingue recevable : un objet de chaînes, le français non vide.
fn texte_valide(v: &Value, champ: &str) -> Result<()> {
    let fr = v.get("fr").and_then(Value::as_str).unwrap_or("").trim();
    let objet_de_chaines = v
        .as_object()
        .is_some_and(|o| o.values().all(Value::is_string));
    if fr.is_empty() || !objet_de_chaines {
        return Err(ApiError::validation(
            "Le texte en français est obligatoire.",
            champ,
        ));
    }
    Ok(())
}

async fn resoudre_le_type(conn: &mut sqlx::PgConnection, code: &str) -> Result<Uuid> {
    documents::type_id(conn, code)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationDocumentUnknownType).field("type"))
}

async fn verifier_la_cop(conn: &mut sqlx::PgConnection, cop: Option<Uuid>) -> Result<()> {
    if let Some(id) = cop {
        if !documents::edition_existe(conn, id).await? {
            return Err(ApiError::validation("Cette COP n'existe pas.", "cop"));
        }
    }
    Ok(())
}

/// Le refus d'un second remplaçant nomme celui qui existe déjà.
async fn nommer_le_successeur(
    state: &NegotiationState,
    erreur: ApiError,
    remplace: Option<Uuid>,
    locale: &str,
) -> ApiError {
    if erreur.code != ErrorCode::NegotiationDocumentAlreadySuperseded {
        return erreur;
    }
    let Some(ancien) = remplace else {
        return erreur;
    };
    let Ok(mut conn) = state.pool().acquire().await else {
        return erreur;
    };
    match documents::successeur_direct(&mut conn, ancien, locale).await {
        Ok(Some(titre)) => deja_remplace(&titre),
        _ => erreur,
    }
}

fn deja_remplace(titre: &str) -> ApiError {
    ApiError::with_message(
        ErrorCode::NegotiationDocumentAlreadySuperseded,
        format!("Ce document est déjà remplacé par « {titre} »."),
    )
    .field("supersedes_id")
}

pub async fn creer(
    state: &NegotiationState,
    ctx: &RequestContext,
    entree: &AdminDocumentInput,
    locale: &str,
) -> Result<Uuid> {
    let auteur = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let title = entree
        .title
        .as_ref()
        .ok_or_else(|| ApiError::validation("Le titre est obligatoire.", "title"))?;
    texte_valide(title, "title")?;
    let summary = entree.summary.clone().flatten();
    if let Some(s) = &summary {
        texte_valide(s, "summary")?;
    }
    let type_code = entree
        .type_code
        .as_deref()
        .ok_or_else(|| ApiError::validation("Le type est obligatoire.", "type"))?;
    let cop = entree.cop.flatten();
    let remplace = entree.supersedes_id.flatten();

    let mut tx = state.db().write(ctx).await?;
    let type_id = resoudre_le_type(&mut tx, type_code).await?;
    verifier_la_cop(&mut tx, cop).await?;
    let publisher = entree.publisher.clone().flatten();
    let external_url = entree.external_url.clone().flatten();
    let resultat = documents::creer(
        &mut tx,
        &Nouveau {
            title,
            summary: summary.as_ref(),
            type_id,
            event_id: cop,
            version: entree
                .version
                .as_deref()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or("1"),
            issued_on: entree.issued_on.flatten(),
            publisher: publisher.as_deref(),
            locale: entree.locale.as_deref().unwrap_or("fr"),
            supersedes_id: remplace,
            restricted: entree.restricted.unwrap_or(false),
            rag_eligible: entree.rag_eligible.unwrap_or(false),
            external_url: external_url.as_deref(),
            auteur,
        },
    )
    .await;
    let id = match resultat {
        Ok(id) => id,
        Err(e) => return Err(nommer_le_successeur(state, e, remplace, locale).await),
    };
    if let Some(themes) = &entree.themes {
        document_themes::remplacer(&mut tx, id, themes).await?;
    }
    tx.commit().await?;
    Ok(id)
}

pub async fn modifier(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    entree: &AdminDocumentInput,
    locale: &str,
) -> Result<()> {
    if let Some(t) = &entree.title {
        texte_valide(t, "title")?;
    }
    if let Some(Some(s)) = &entree.summary {
        texte_valide(s, "summary")?;
    }
    let mut tx = state.db().write(ctx).await?;
    let etat = documents::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    if etat.deja_publie() && entree.external_url.is_some() {
        return Err(ApiError::new(ErrorCode::NegotiationDocumentFileLocked).field("external_url"));
    }
    let type_id = match &entree.type_code {
        Some(code) => Some(resoudre_le_type(&mut tx, code).await?),
        None => None,
    };
    if let Some(cop) = entree.cop {
        verifier_la_cop(&mut tx, cop).await?;
    }
    let summary = entree.summary.as_ref().map(|s| s.as_ref());
    let publisher = entree.publisher.as_ref().map(|p| p.as_deref());
    let external_url = entree.external_url.as_ref().map(|u| u.as_deref());
    let resultat = documents::modifier(
        &mut tx,
        id,
        &Modification {
            title: entree.title.as_ref(),
            summary,
            type_id,
            event_id: entree.cop,
            version: entree.version.as_deref().filter(|v| !v.trim().is_empty()),
            issued_on: entree.issued_on,
            publisher,
            locale: entree.locale.as_deref(),
            supersedes_id: entree.supersedes_id,
            restricted: entree.restricted,
            rag_eligible: entree.rag_eligible,
            external_url,
        },
    )
    .await;
    if let Err(e) = resultat {
        return Err(nommer_le_successeur(state, e, entree.supersedes_id.flatten(), locale).await);
    }
    if let Some(themes) = &entree.themes {
        document_themes::remplacer(&mut tx, id, themes).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Attache le PDF déposé par la garde média, et met l'extraction en file dans
/// la même transaction. Le fichier d'un document déjà publié est figé.
pub async fn attacher_le_fichier(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    asset_id: Uuid,
) -> Result<()> {
    let auteur = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let mut tx = state.db().write(ctx).await?;
    let etat = documents::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    if etat.deja_publie() {
        return Err(ApiError::new(ErrorCode::NegotiationDocumentFileLocked).field("asset_id"));
    }
    let objet = objets::emplacement(&mut tx, asset_id)
        .await?
        .ok_or_else(|| {
            ApiError::validation("Ce fichier n'existe pas, ou a été supprimé.", "asset_id")
        })?;
    let prive = settings::bucket_prive(&mut tx).await?;
    if objet.mime_type != "application/pdf" || objet.bucket != prive {
        return Err(ApiError::validation(
            "Déposez un PDF avec ce document pour propriétaire : il est alors gardé à l'abri du web.",
            "asset_id",
        ));
    }
    documents::attacher_le_fichier(&mut tx, id, asset_id).await?;
    extract::mettre_en_file(&mut tx, id, asset_id, auteur, Uuid::now_v7()).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn relancer_lextraction(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
) -> Result<()> {
    let auteur = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let mut tx = state.db().write(ctx).await?;
    let etat = documents::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    if etat.deja_publie() {
        return Err(ApiError::new(ErrorCode::NegotiationDocumentFileLocked));
    }
    let asset_id = etat.asset_id.ok_or_else(|| {
        ApiError::validation("Ce document n'a pas de fichier à extraire.", "asset_id")
    })?;
    extract::mettre_en_file(&mut tx, id, asset_id, auteur, Uuid::now_v7()).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn ouvrir_tel_quel(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    tel_quel: bool,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    documents::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    if !renditions::poser_tel_quel(&mut tx, id, tel_quel).await? {
        return Err(ApiError::validation(
            "Ce document n'a pas encore de fichier extrait.",
            "serve_as_is",
        ));
    }
    tx.commit().await?;
    Ok(())
}

/// Un lien se publie tel quel ; un fichier, une fois son extraction prête.
pub async fn publier(state: &NegotiationState, ctx: &RequestContext, id: Uuid) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    let etat = documents::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    if etat.published_at.is_some() {
        return Ok(());
    }
    if let Some(asset_id) = etat.asset_id {
        let pret = renditions::lire(&mut tx, id)
            .await?
            .is_some_and(|r| r.asset_id == asset_id && r.status == "ready");
        if !pret {
            return Err(ApiError::new(ErrorCode::NegotiationDocumentNotReady));
        }
    }
    documents::publier(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn depublier(state: &NegotiationState, ctx: &RequestContext, id: Uuid) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    documents::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    documents::depublier(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn nouvelle_version(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    locale: &str,
) -> Result<Uuid> {
    let auteur = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let mut tx = state.db().write(ctx).await?;
    documents::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    let ancienne = documents::fiches(&mut tx, Some(id), locale)
        .await?
        .into_iter()
        .next()
        .ok_or_else(introuvable)?;
    // Avant l'écriture : sa version recopiée heurterait l'unicité du slug avant
    // celle du successeur, et le refus ne nommerait pas le remplaçant.
    if let Some(titre) = documents::successeur_direct(&mut tx, id, locale).await? {
        return Err(deja_remplace(&titre));
    }
    let version = format!("{} (nouvelle version)", ancienne.version);
    match documents::nouvelle_version(&mut tx, id, &version, auteur).await {
        Ok(nouveau) => {
            tx.commit().await?;
            Ok(nouveau)
        }
        Err(e) => Err(nommer_le_successeur(state, e, Some(id), locale).await),
    }
}

/// Un brouillon jamais publié se supprime, avec ses liens de thématique et ses
/// images de page ; un document publié se dépublie.
pub async fn supprimer(state: &NegotiationState, ctx: &RequestContext, id: Uuid) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    let etat = documents::verrouiller(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    if etat.deja_publie() {
        return Err(ApiError::new(
            ErrorCode::NegotiationDocumentPublishedUndeletable,
        ));
    }
    let images = document_pages::cles_d_images(&mut tx, id).await?;
    let bucket = settings::bucket_prive(&mut tx).await?;
    document_themes::retirer(&mut tx, id).await?;
    if !documents::supprimer_le_brouillon(&mut tx, id).await? {
        return Err(ApiError::new(
            ErrorCode::NegotiationDocumentPublishedUndeletable,
        ));
    }
    tx.commit().await?;
    let stockage = state.entrepots().du_bucket(&bucket);
    for cle in images {
        if let Err(erreur) = stockage.delete(&cle).await {
            tracing::warn!(%cle, %erreur, "image de page d'un brouillon supprimé non effacée");
        }
    }
    Ok(())
}

/// L'aperçu : le verdict, les indicateurs, le sommaire, et chaque page.
pub async fn apercu(state: &NegotiationState, id: Uuid) -> Result<AdminDocumentPreview> {
    let mut conn = state.pool().acquire().await?;
    let doc = documents::quelconque(&mut conn, id)
        .await?
        .ok_or_else(introuvable)?;
    let rendu = renditions::lire(&mut conn, id).await?;
    let courant = rendu.as_ref().filter(|r| Some(r.asset_id) == doc.asset_id);
    let pages = if courant.is_some_and(|r| r.status == "ready") {
        document_pages::lire(&mut conn, id).await?
    } else {
        vec![]
    };
    Ok(AdminDocumentPreview {
        id,
        extraction: extraction(rendu.as_ref(), doc.asset_id),
        quality: courant.and_then(|r| r.quality.clone()),
        extractor: courant.and_then(|r| r.extractor.clone()),
        outline: courant
            .and_then(|r| r.outline.clone())
            .unwrap_or(Value::Array(vec![])),
        pages: pages
            .into_iter()
            .map(|p| AdminPreviewPage {
                index: p.index,
                label: p.label,
                blocks: p.blocks,
                image: p
                    .image_key
                    .as_ref()
                    .map(|_| format!("/admin/negotiation/documents/{id}/pages/{}/image", p.index)),
                has_origin_block: p.has_origin_block,
            })
            .collect(),
    })
}

/// Le PDF d'origine, lu dans le bucket privé.
pub async fn pdf(state: &NegotiationState, id: Uuid) -> Result<(Vec<u8>, Option<String>)> {
    let mut conn = state.pool().acquire().await?;
    let doc = documents::quelconque(&mut conn, id)
        .await?
        .ok_or_else(introuvable)?;
    let asset_id = doc.asset_id.ok_or_else(ApiError::not_found)?;
    let objet = objets::emplacement(&mut conn, asset_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    drop(conn);
    let octets = state
        .entrepots()
        .du_bucket(&objet.bucket)
        .get(&objet.object_key)
        .await?;
    Ok((octets, objet.original_filename))
}
