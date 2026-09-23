//! Les lectures publiques des documents, et les deux gestes sans conséquence
//! pour personne : compter un téléchargement, poser un favori.
//!
//! **Réservé** veut dire : la liste montre le document à tous, mais sans son
//! résumé ni ses thématiques pour qui n'a pas l'accès ; la forme lisible, les
//! images, le téléchargement et les passages trouvés lui sont refusés (R10).

use std::collections::{BTreeSet, HashMap};

use kernel::auth::{has_permission, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::documents::{
    chemin_image, empreinte_de_lecture, hote, CorrectionNote, CorrectionNoteList,
    DocumentBookmarkList, DocumentLibrary, DocumentReading, DocumentTextHits, LibraryDocument,
    LibraryVocabulary, PageHit, ReadingPage, Successor, TextHit,
};
use crate::domain::permissions::SPACE_ACCESS;
use crate::repo::documents::{self, Lisible};
use crate::repo::{bookmarks, corrections, document_pages, settings};
use crate::state::NegotiationState;

/// L'accès négociateur, sur la portée globale : la règle de 0b.
pub async fn a_lacces(state: &NegotiationState, personne: Option<Uuid>) -> Result<bool> {
    match personne {
        Some(p) => has_permission(state.pool(), p, SPACE_ACCESS, Scope::Global).await,
        None => Ok(false),
    }
}

fn refuse() -> ApiError {
    ApiError::new(ErrorCode::NegotiationDocumentRestricted)
}

fn introuvable() -> ApiError {
    ApiError::new(ErrorCode::NegotiationDocumentNotFound)
}

/// La bibliothèque, et son empreinte — calculée sur ce que la personne voit,
/// l'heure de service exclue.
pub async fn bibliotheque(
    state: &NegotiationState,
    personne: Option<Uuid>,
    locale: &str,
) -> Result<(DocumentLibrary, String)> {
    let acces = a_lacces(state, personne).await?;
    let mut conn = state.pool().acquire().await?;
    let publies = documents::publies(&mut conn, locale).await?;
    let bouts = documents::bouts_de_chaine(&mut conn, locale).await?;

    let documents: Vec<LibraryDocument> = publies
        .into_iter()
        .map(|d| {
            let visible = !d.restricted || acces;
            let fichier = d.asset_id.is_some();
            let rendu = d.rendu.as_ref().filter(|r| r.pret_pour(d.asset_id));
            LibraryDocument {
                id: d.id,
                slug: d.slug,
                version: d.version,
                title: d.title,
                summary: d.summary.filter(|_| visible),
                type_code: d.type_code,
                themes_hidden: !visible,
                themes: if visible { d.themes } else { vec![] },
                cop: d.event_id,
                issued_on: d.issued_on,
                published_at: d.published_at,
                publisher: d.publisher,
                locale: d.locale,
                source: if fichier { "file" } else { "link" },
                link_host: d.external_url.as_deref().and_then(hote),
                // Pour un lien, l'adresse est le contenu (SC-007).
                external_url: d.external_url.filter(|_| visible),
                restricted: d.restricted,
                accessible: visible,
                page_count: rendu.and_then(|r| r.page_count),
                reading_bytes: rendu.and_then(|r| r.reading_bytes),
                mode: rendu.map(|r| if r.serve_as_is { "as_is" } else { "reflow" }),
                reading_etag: rendu.and_then(|r| {
                    r.extracted_at
                        .map(|e| empreinte_de_lecture(d.id, r.asset_id, e, r.serve_as_is))
                }),
                superseded_by: bouts.get(&d.id).map(|s| Successor {
                    id: s.id,
                    title: s.title.clone(),
                    published_at: s.published_at,
                    page_count: s.page_count,
                }),
            }
        })
        .collect();

    let types: BTreeSet<&str> = documents.iter().map(|d| d.type_code.as_str()).collect();
    let themes: BTreeSet<&str> = documents
        .iter()
        .flat_map(|d| d.themes.iter().map(String::as_str))
        .collect();
    let cops: Vec<Uuid> = documents
        .iter()
        .filter_map(|d| d.cop)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let vocabulary = LibraryVocabulary {
        types: documents::vocabulaire(&mut conn, "document_type", locale)
            .await?
            .into_iter()
            .filter(|t| types.contains(t.code.as_str()))
            .collect(),
        themes: documents::vocabulaire(&mut conn, "negotiation_theme", locale)
            .await?
            .into_iter()
            .filter(|t| themes.contains(t.code.as_str()))
            .collect(),
        cops: documents::cops(&mut conn, &cops, locale).await?,
    };

    let empreinte = kernel::empreinte::de(
        &serde_json::to_string(&(&documents, &vocabulary))
            .map_err(|e| ApiError::internal(format!("empreinte de la bibliothèque : {e}")))?,
    );
    Ok((
        DocumentLibrary {
            documents,
            vocabulary,
            served_at: OffsetDateTime::now_utc(),
        },
        empreinte,
    ))
}

/// Les passages trouvés, cinq pages au plus par document. Un réservé sans accès
/// ne rend que son identifiant : dire qu'il existe est permis, le citer non.
pub async fn rechercher(
    state: &NegotiationState,
    personne: Option<Uuid>,
    requete: &str,
) -> Result<DocumentTextHits> {
    let requete = requete.trim();
    if requete.is_empty() {
        return Err(ApiError::validation("Saisissez un mot à chercher.", "q"));
    }
    let acces = a_lacces(state, personne).await?;
    let mut conn = state.pool().acquire().await?;
    let trouvailles = documents::rechercher(&mut conn, requete).await?;

    let mut ordre: Vec<Uuid> = Vec::new();
    let mut par_document: HashMap<Uuid, TextHit> = HashMap::new();
    for t in trouvailles {
        let hit = par_document.entry(t.document_id).or_insert_with(|| {
            ordre.push(t.document_id);
            TextHit {
                document_id: t.document_id,
                pages: vec![],
            }
        });
        if (!t.restricted || acces) && hit.pages.len() < 5 {
            hit.pages.push(PageHit {
                index: t.page_index,
                label: t.label,
                excerpt: t.excerpt,
            });
        }
    }
    Ok(DocumentTextHits {
        query: requete.to_owned(),
        hits: ordre
            .into_iter()
            .filter_map(|id| par_document.remove(&id))
            .collect(),
    })
}

/// Un document publié, lisible par cette personne, et dont le fichier est
/// extrait. Réservé sans accès : 403 ; lien : 409 ; sinon : 404.
async fn lisible(state: &NegotiationState, personne: Option<Uuid>, id: Uuid) -> Result<Lisible> {
    let mut conn = state.pool().acquire().await?;
    let doc = documents::publie(&mut conn, id)
        .await?
        .ok_or_else(introuvable)?;
    drop(conn);
    if doc.restricted && !a_lacces(state, personne).await? {
        return Err(refuse());
    }
    if doc.asset_id.is_none() {
        return Err(ApiError::new(ErrorCode::NegotiationDocumentNotReadable));
    }
    if !doc
        .rendu
        .as_ref()
        .is_some_and(|r| r.pret_pour(doc.asset_id))
    {
        return Err(introuvable());
    }
    Ok(doc)
}

/// La forme lisible entière, et son empreinte figée.
pub async fn lecture(
    state: &NegotiationState,
    personne: Option<Uuid>,
    id: Uuid,
) -> Result<(DocumentReading, String, bool)> {
    let doc = lisible(state, personne, id).await?;
    let rendu = doc.rendu.as_ref().expect("rendu vérifié");
    let tel_quel = rendu.serve_as_is;
    let mut conn = state.pool().acquire().await?;
    let pages = document_pages::lire(&mut conn, id).await?;
    let lecture = DocumentReading {
        id,
        version: doc.version.clone(),
        mode: if tel_quel { "as_is" } else { "reflow" },
        page_count: pages.len() as i32,
        outline: if tel_quel {
            Value::Array(vec![])
        } else {
            doc.outline.clone().unwrap_or(Value::Array(vec![]))
        },
        pages: pages
            .into_iter()
            .map(|p| ReadingPage {
                index: p.index,
                label: p.label,
                image: (tel_quel || p.has_origin_block)
                    .then(|| chemin_image(id, p.index))
                    .filter(|_| p.image_key.is_some()),
                blocks: if tel_quel {
                    Value::Array(vec![])
                } else {
                    p.blocks
                },
            })
            .collect(),
    };
    let empreinte = empreinte_de_lecture(
        id,
        rendu.asset_id,
        rendu.extracted_at.unwrap_or(OffsetDateTime::UNIX_EPOCH),
        tel_quel,
    );
    Ok((lecture, empreinte, doc.restricted))
}

pub struct ImageDePage {
    pub octets: Vec<u8>,
    pub empreinte: String,
    pub reservee: bool,
}

async fn lire_image(
    state: &NegotiationState,
    id: Uuid,
    index: i32,
    reservee: bool,
) -> Result<ImageDePage> {
    let mut conn = state.pool().acquire().await?;
    let cle = document_pages::lire(&mut conn, id)
        .await?
        .into_iter()
        .find(|p| p.index == index)
        .and_then(|p| p.image_key)
        .ok_or_else(ApiError::not_found)?;
    let bucket = settings::bucket_prive(&mut conn).await?;
    drop(conn);
    let octets = state.entrepots().du_bucket(&bucket).get(&cle).await?;
    Ok(ImageDePage {
        octets,
        empreinte: kernel::empreinte::de(&cle),
        reservee,
    })
}

pub async fn image(
    state: &NegotiationState,
    personne: Option<Uuid>,
    id: Uuid,
    index: i32,
) -> Result<ImageDePage> {
    let doc = lisible(state, personne, id).await?;
    lire_image(state, id, index, doc.restricted).await
}

/// L'image d'une page pour l'aperçu du back-office, brouillon compris. La
/// garde est celle de la route.
pub async fn image_de_lapercu(
    state: &NegotiationState,
    id: Uuid,
    index: i32,
) -> Result<ImageDePage> {
    let mut conn = state.pool().acquire().await?;
    let doc = documents::quelconque(&mut conn, id)
        .await?
        .ok_or_else(introuvable)?;
    drop(conn);
    lire_image(state, id, index, doc.restricted).await
}

/// Les notes vivantes de tous les documents publiés ; celles d'un réservé ne
/// vont qu'à qui a l'accès.
pub async fn notes(
    state: &NegotiationState,
    personne: Option<Uuid>,
    locale: &str,
) -> Result<(CorrectionNoteList, String)> {
    let acces = a_lacces(state, personne).await?;
    let mut conn = state.pool().acquire().await?;
    let notes: Vec<CorrectionNote> = corrections::vivantes(&mut conn)
        .await?
        .into_iter()
        .filter(|n| !n.restricted || acces)
        .map(|n| CorrectionNote {
            id: n.id,
            document_id: n.document_id,
            page_index: n.page_index,
            passage: n.passage,
            body: texte_dans(&n.body, locale),
            author_name: n.author_name,
            posted_at: n.created_at,
        })
        .collect();
    let empreinte = kernel::empreinte::de(
        &serde_json::to_string(&notes)
            .map_err(|e| ApiError::internal(format!("empreinte des notes : {e}")))?,
    );
    Ok((CorrectionNoteList { notes }, empreinte))
}

/// Un texte multilingue, dans la langue demandée, repli sur le français.
pub fn texte_dans(texte: &Value, locale: &str) -> String {
    texte
        .get(locale)
        .or_else(|| texte.get("fr"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Le compteur : rien de la personne n'est gardé.
pub async fn compter_un_telechargement(
    state: &NegotiationState,
    personne: Option<Uuid>,
    id: Uuid,
) -> Result<()> {
    let mut conn = state.pool().acquire().await?;
    let doc = documents::publie(&mut conn, id)
        .await?
        .ok_or_else(introuvable)?;
    drop(conn);
    if doc.restricted && !a_lacces(state, personne).await? {
        return Err(refuse());
    }
    let mut conn = state.pool().acquire().await?;
    documents::compter_un_telechargement(&mut conn, id).await
}

pub async fn favoris(
    state: &NegotiationState,
    personne: Uuid,
) -> Result<(DocumentBookmarkList, String)> {
    let mut conn = state.pool().acquire().await?;
    let bookmarks = bookmarks::lister(&mut conn, personne).await?;
    let empreinte = kernel::empreinte::de(
        &bookmarks
            .iter()
            .map(|b| b.document_id.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok((DocumentBookmarkList { bookmarks }, empreinte))
}

/// Idempotent. Un favori sur un réservé sans accès est permis : il ne dévoile
/// rien que la liste ne montre déjà.
pub async fn poser_un_favori(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    documents::publie(&mut tx, id)
        .await?
        .ok_or_else(introuvable)?;
    bookmarks::poser(&mut tx, personne, id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn retirer_un_favori(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    bookmarks::retirer(&mut tx, personne, id).await?;
    tx.commit().await?;
    Ok(())
}
