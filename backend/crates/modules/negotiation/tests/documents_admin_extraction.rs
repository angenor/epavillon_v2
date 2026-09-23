//! **Le back-office face à l'extraction, par les services** : relancer
//! l'extraction, « ouvrir tel quel », et ce que ce choix change à la
//! publication, à la lecture et à l'audit.

mod commun;

use commun::documents::{administratrice, creer, objet_pdf, passer_lextraction, rendu, PETIT};
use commun::{traces, Bac};
use kernel::error::{ApiError, ErrorCode, Result};
use negotiation::domain::admin_documents::AdminDocument;
use negotiation::service::admin_documents as admin;
use negotiation::service::documents as public;
use serde_json::{json, Value};
use uuid::Uuid;

const PAS_PRETE: &str =
    "L'extraction n'est pas terminée. Attendez-la, ou choisissez « ouvrir tel quel ».";

fn refus<T: std::fmt::Debug>(r: Result<T>) -> ApiError {
    r.expect_err("un refus était attendu")
}

async fn la_fiche(bac: &Bac, personne: Uuid, id: Uuid) -> AdminDocument {
    let droits = admin::droits(&bac.state, personne).await.expect("droits");
    admin::fiche(&bac.state, &droits, id, "fr")
        .await
        .expect("fiche du document")
}

/// Un PDF dans l'état voulu du média, attaché au document.
async fn deposer(bac: &Bac, auteur: Uuid, id: Uuid, statut: &str) -> Uuid {
    let asset = objet_pdf(bac, auteur, PETIT, statut).await;
    admin::attacher_le_fichier(&bac.state, &bac.ctx(auteur), id, asset)
        .await
        .expect("fichier attaché");
    asset
}

async fn tel_quel(bac: &Bac, auteur: Uuid, id: Uuid, oui: bool) -> Result<()> {
    admin::ouvrir_tel_quel(&bac.state, &bac.ctx(auteur), id, oui).await
}

async fn publier(bac: &Bac, auteur: Uuid, id: Uuid) -> Result<()> {
    admin::publier(&bac.state, &bac.ctx(auteur), id).await
}

async fn relancer(bac: &Bac, auteur: Uuid, id: Uuid) -> Result<()> {
    admin::relancer_lextraction(&bac.state, &bac.ctx(auteur), id).await
}

async fn rendus_en_base(bac: &Bac, id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM negotiation.document_renditions WHERE document_id = $1",
    )
    .bind(id)
    .fetch_one(bac.pool())
    .await
    .expect("compte des extractions")
}

/// La demande d'extraction en cours : seul son travail peut conclure.
async fn demande_en_cours(bac: &Bac, id: Uuid) -> Uuid {
    sqlx::query_scalar(
        "SELECT request_id FROM negotiation.document_renditions WHERE document_id = $1",
    )
    .bind(id)
    .fetch_one(bac.pool())
    .await
    .expect("demande d'extraction")
}

/// Les traces où `serve_as_is` a changé : l'autrice et la valeur posée.
async fn traces_du_tel_quel(bac: &Bac, id: Uuid) -> Vec<(Option<Uuid>, Value)> {
    sqlx::query_as(
        "SELECT actor_id, new_data -> 'serve_as_is'
           FROM platform.audit_log
          WHERE entity_schema = 'negotiation' AND entity_table = 'document_renditions'
            AND (new_data ->> 'document_id')::uuid = $1
            AND action = 'update' AND 'serve_as_is' = ANY (changed_fields)
          ORDER BY occurred_at, id",
    )
    .bind(id)
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'audit")
}

// -----------------------------------------------------------------------------
// « Ouvrir tel quel »
// -----------------------------------------------------------------------------

#[tokio::test]
async fn ouvrir_tel_quel_attend_un_fichier_extrait_puis_se_publie_et_se_defait_sans_republier() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Guide des négociations", false).await;

    let err = refus(tel_quel(&bac, ifdd, id, true).await);
    assert_eq!(err.code, ErrorCode::ValidationFailed, "{err}");
    assert_eq!(err.field.as_deref(), Some("serve_as_is"));
    assert_eq!(
        err.message,
        "Ce document n'a pas encore de fichier extrait."
    );
    assert_eq!(rendus_en_base(&bac, id).await, 0);
    let err = refus(tel_quel(&bac, ifdd, Uuid::now_v7(), true).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentNotFound, "{err}");

    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    let recompose = la_fiche(&bac, ifdd, id)
        .await
        .extraction
        .expect("extraction");
    assert_eq!(recompose.status, "ready");
    assert!(
        !recompose.serve_as_is,
        "recomposé tant qu'elle n'a rien choisi"
    );

    tel_quel(&bac, ifdd, id, true)
        .await
        .expect("tel quel, une fois le fichier extrait");
    let choisi = la_fiche(&bac, ifdd, id).await;
    assert_eq!(choisi.state, "draft", "le choix ne publie rien");
    let extraction = choisi.extraction.expect("extraction");
    assert!(extraction.serve_as_is);
    assert_eq!(extraction.status, "ready", "le verdict reste");
    assert_eq!(extraction.is_reflowable, Some(true));
    let apercu = admin::apercu(&bac.state, id).await.expect("aperçu");
    assert!(apercu.extraction.is_some_and(|e| e.serve_as_is));

    publier(&bac, ifdd, id).await.expect("publication tel quel");
    let publie = la_fiche(&bac, ifdd, id).await;
    assert_eq!(publie.state, "published");
    assert!(publie.extraction.is_some_and(|e| e.serve_as_is));
    let (lecture, empreinte_tel_quel, _) = public::lecture(&bac.state, None, id)
        .await
        .expect("lecture");
    assert_eq!(lecture.mode, "as_is");
    assert_eq!(lecture.pages.len(), 4);
    assert!(
        lecture.pages.iter().all(|p| p.image.is_some()),
        "tel quel, chaque page se lit en image"
    );

    tel_quel(&bac, ifdd, id, false)
        .await
        .expect("retour au recomposé, document publié");
    let apres = la_fiche(&bac, ifdd, id).await;
    assert_eq!(apres.state, "published");
    assert_eq!(apres.published_at, publie.published_at, "sans republier");
    assert!(apres.extraction.is_some_and(|e| !e.serve_as_is));
    let (lecture, empreinte_recomposee, _) = public::lecture(&bac.state, None, id)
        .await
        .expect("lecture");
    assert_eq!(lecture.mode, "reflow");
    assert_ne!(
        empreinte_recomposee, empreinte_tel_quel,
        "le téléphone doit voir que sa copie n'est plus la bonne"
    );
}

/// « Tel quel » lit les images de la dernière extraction : sans elle, rien ne
/// se publie.
#[tokio::test]
async fn ouvrir_tel_quel_ne_publie_pas_un_fichier_dont_lextraction_a_echoue() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Rapport de session", false).await;
    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    tel_quel(&bac, ifdd, id, true).await.expect("tel quel");

    deposer(&bac, ifdd, id, "quarantined").await;
    let en_attente = la_fiche(&bac, ifdd, id)
        .await
        .extraction
        .expect("extraction");
    assert_eq!(en_attente.status, "pending");
    assert!(
        en_attente.serve_as_is,
        "le choix suit le document, pas son fichier"
    );
    let err = refus(publier(&bac, ifdd, id).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentNotReady, "{err}");

    passer_lextraction(&bac).await;
    assert_eq!(rendu(&bac, id).await.status, "failed");
    let err = refus(publier(&bac, ifdd, id).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentNotReady,
        "les pages du fichier précédent ne tiennent pas lieu d'images : {err}"
    );
    assert_eq!(err.message, PAS_PRETE);
    assert_eq!(la_fiche(&bac, ifdd, id).await.state, "draft");

    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    publier(&bac, ifdd, id)
        .await
        .expect("un fichier extrait se publie tel quel");
    let publie = la_fiche(&bac, ifdd, id).await;
    assert_eq!(publie.state, "published");
    assert!(publie.extraction.is_some_and(|e| e.serve_as_is));
}

#[tokio::test]
async fn le_choix_tel_quel_laisse_une_trace_au_nom_de_qui_la_fait() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let collegue = administratrice(&bac, "collegue@example.org").await;
    let id = creer(&bac, ifdd, "Guide des négociations", false).await;
    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    assert!(traces_du_tel_quel(&bac, id).await.is_empty());

    tel_quel(&bac, collegue, id, true).await.expect("tel quel");
    tel_quel(&bac, collegue, id, true)
        .await
        .expect("le même choix, rejoué");
    tel_quel(&bac, ifdd, id, false).await.expect("recomposé");

    assert_eq!(
        traces_du_tel_quel(&bac, id).await,
        [(Some(collegue), json!(true)), (Some(ifdd), json!(false))],
        "une trace par changement, chacune au nom de qui l'a fait"
    );
}

/// Le journal se lit par entité : une trace sans identifiant ne ressort pas
/// dans l'historique du document.
#[tokio::test]
async fn la_trace_du_choix_tel_quel_se_rattache_a_son_document() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Guide des négociations", false).await;
    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    tel_quel(&bac, ifdd, id, true).await.expect("tel quel");

    let par_entite = traces(&bac, "document_renditions", id).await;
    assert!(
        par_entite.contains(&("update".to_owned(), Some(ifdd))),
        "{par_entite:?}"
    );
}

// -----------------------------------------------------------------------------
// Relancer l'extraction
// -----------------------------------------------------------------------------

#[tokio::test]
async fn relancer_lextraction_repart_de_zero_sur_un_brouillon_et_se_refuse_une_fois_publie() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Guide des négociations", false).await;

    let err = refus(relancer(&bac, ifdd, id).await);
    assert_eq!(err.code, ErrorCode::ValidationFailed, "{err}");
    assert_eq!(err.field.as_deref(), Some("asset_id"));
    assert_eq!(err.message, "Ce document n'a pas de fichier à extraire.");
    assert_eq!(rendus_en_base(&bac, id).await, 0);
    let err = refus(relancer(&bac, ifdd, Uuid::now_v7()).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentNotFound, "{err}");

    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    tel_quel(&bac, ifdd, id, true).await.expect("tel quel");
    let premiere = demande_en_cours(&bac, id).await;

    relancer(&bac, ifdd, id)
        .await
        .expect("relance d'un brouillon");
    let relancee = la_fiche(&bac, ifdd, id)
        .await
        .extraction
        .expect("extraction");
    assert_eq!(relancee.status, "pending");
    assert_eq!(relancee.page_count, None, "le verdict précédent s'efface");
    assert!(relancee.extracted_at.is_none());
    assert!(
        relancee.serve_as_is,
        "la relance garde le choix « tel quel »"
    );
    assert_ne!(demande_en_cours(&bac, id).await, premiere);
    let err = refus(publier(&bac, ifdd, id).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentNotReady,
        "rien ne se publie pendant la relance : {err}"
    );

    let issues = passer_lextraction(&bac).await;
    assert!(
        !issues.is_empty() && issues.iter().all(Result::is_ok),
        "la relance a mis un travail en file : {issues:?}"
    );
    let refaite = rendu(&bac, id).await;
    assert_eq!(refaite.status, "ready");
    assert_eq!(refaite.page_count, Some(4));
    publier(&bac, ifdd, id).await.expect("publication");

    let demande = demande_en_cours(&bac, id).await;
    let err = refus(relancer(&bac, ifdd, id).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentFileLocked, "{err}");
    assert_eq!(
        err.message,
        "Le fichier d'un document publié ne change pas. Publiez une nouvelle version."
    );
    admin::depublier(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("dépublication");
    let err = refus(relancer(&bac, ifdd, id).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentFileLocked,
        "dépublié, il reste figé : {err}"
    );
    assert_eq!(rendu(&bac, id).await.status, "ready");
    assert_eq!(
        demande_en_cours(&bac, id).await,
        demande,
        "aucune demande nouvelle"
    );
    assert!(
        passer_lextraction(&bac).await.is_empty(),
        "aucun travail en file"
    );
}
