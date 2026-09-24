//! **Le back-office face à l'extraction, par les services** : relancer
//! l'extraction, proposer « Texte agrandi », et ce que ce choix change à la
//! publication, à la lecture et à l'audit.

mod commun;

use commun::documents::{administratrice, creer, objet_pdf, passer_lextraction, rendu, PETIT};

const PROTEGE: &[u8] = include_bytes!("fixtures/protege.pdf");
use commun::{traces, Bac};
use kernel::error::{ApiError, ErrorCode, Result};
use negotiation::domain::admin_documents::AdminDocument;
use negotiation::service::admin_documents as admin;
use negotiation::service::documents as public;
use serde_json::{json, Value};
use uuid::Uuid;

const PAS_PRETE: &str = "L'extraction n'est pas terminée. Attendez-la avant de publier.";

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

async fn choisir(bac: &Bac, auteur: Uuid, id: Uuid, choix: Option<bool>) -> Result<()> {
    admin::choisir_le_texte_agrandi(&bac.state, &bac.ctx(auteur), id, choix).await
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

/// Les traces où `large_text_choice` a changé : l'autrice et la valeur posée.
async fn traces_du_choix(bac: &Bac, id: Uuid) -> Vec<(Option<Uuid>, Value)> {
    sqlx::query_as(
        "SELECT actor_id, new_data -> 'large_text_choice'
           FROM platform.audit_log
          WHERE entity_schema = 'negotiation' AND entity_table = 'document_renditions'
            AND (new_data ->> 'document_id')::uuid = $1
            AND action = 'update' AND 'large_text_choice' = ANY (changed_fields)
          ORDER BY occurred_at, id",
    )
    .bind(id)
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'audit")
}

// -----------------------------------------------------------------------------
// Proposer « Texte agrandi »
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_choix_texte_agrandi_attend_un_fichier_extrait_et_se_change_sans_republier() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Guide des négociations", false).await;

    let err = refus(choisir(&bac, ifdd, id, Some(false)).await);
    assert_eq!(err.code, ErrorCode::ValidationFailed, "{err}");
    assert_eq!(err.field.as_deref(), Some("choice"));
    assert_eq!(
        err.message,
        "Ce document n'a pas encore de fichier extrait."
    );
    assert_eq!(rendus_en_base(&bac, id).await, 0);
    let err = refus(choisir(&bac, ifdd, Uuid::now_v7(), Some(false)).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentNotFound, "{err}");

    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    let du_verdict = la_fiche(&bac, ifdd, id)
        .await
        .extraction
        .expect("extraction");
    assert_eq!(du_verdict.status, "ready");
    assert_eq!(du_verdict.large_text_choice, None, "aucun choix posé");
    assert!(du_verdict.has_text);
    assert!(du_verdict.large_text, "le verdict le propose");

    choisir(&bac, ifdd, id, Some(false))
        .await
        .expect("retiré, une fois le fichier extrait");
    let choisi = la_fiche(&bac, ifdd, id).await;
    assert_eq!(choisi.state, "draft", "le choix ne publie rien");
    let extraction = choisi.extraction.expect("extraction");
    assert_eq!(extraction.large_text_choice, Some(false));
    assert!(!extraction.large_text);
    assert_eq!(extraction.status, "ready", "le verdict reste");
    assert_eq!(extraction.is_reflowable, Some(true));
    let apercu = admin::apercu(&bac.state, id).await.expect("aperçu");
    assert!(apercu
        .extraction
        .is_some_and(|e| e.large_text_choice == Some(false) && !e.large_text));

    publier(&bac, ifdd, id)
        .await
        .expect("publication sans « Texte agrandi »");
    let publie = la_fiche(&bac, ifdd, id).await;
    assert_eq!(publie.state, "published");
    let (lecture, empreinte_retire, _) = public::lecture(&bac.state, None, id)
        .await
        .expect("lecture");
    assert!(lecture.has_text && !lecture.large_text);
    assert_eq!(lecture.pages.len(), 4);

    choisir(&bac, ifdd, id, None)
        .await
        .expect("rendu au verdict, document publié");
    let apres = la_fiche(&bac, ifdd, id).await;
    assert_eq!(apres.state, "published");
    assert_eq!(apres.published_at, publie.published_at, "sans republier");
    assert!(apres
        .extraction
        .is_some_and(|e| e.large_text_choice.is_none() && e.large_text));
    let (lecture, empreinte_verdict, _) = public::lecture(&bac.state, None, id)
        .await
        .expect("lecture");
    assert!(lecture.large_text);
    assert_ne!(
        empreinte_verdict, empreinte_retire,
        "le téléphone doit voir que sa copie n'est plus la bonne"
    );
}

/// Le choix suit le document, pas son fichier ; la publication exige toujours
/// une extraction prête.
#[tokio::test]
async fn le_choix_ne_publie_pas_un_fichier_dont_lextraction_a_echoue() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Rapport de session", false).await;
    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    choisir(&bac, ifdd, id, Some(false)).await.expect("retiré");

    deposer(&bac, ifdd, id, "quarantined").await;
    let en_attente = la_fiche(&bac, ifdd, id)
        .await
        .extraction
        .expect("extraction");
    assert_eq!(en_attente.status, "pending");
    assert_eq!(
        en_attente.large_text_choice,
        Some(false),
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
        "les pages du fichier précédent ne tiennent pas lieu d'extraction : {err}"
    );
    assert_eq!(err.message, PAS_PRETE);
    assert_eq!(la_fiche(&bac, ifdd, id).await.state, "draft");

    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    publier(&bac, ifdd, id)
        .await
        .expect("un fichier extrait se publie");
    let publie = la_fiche(&bac, ifdd, id).await;
    assert_eq!(publie.state, "published");
    assert!(publie
        .extraction
        .is_some_and(|e| e.large_text_choice == Some(false)));
}

#[tokio::test]
async fn le_choix_texte_agrandi_laisse_une_trace_au_nom_de_qui_le_fait() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let collegue = administratrice(&bac, "collegue@example.org").await;
    let id = creer(&bac, ifdd, "Guide des négociations", false).await;
    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    assert!(traces_du_choix(&bac, id).await.is_empty());

    choisir(&bac, collegue, id, Some(false))
        .await
        .expect("retiré");
    choisir(&bac, collegue, id, Some(false))
        .await
        .expect("le même choix, rejoué");
    choisir(&bac, ifdd, id, None)
        .await
        .expect("rendu au verdict");

    assert_eq!(
        traces_du_choix(&bac, id).await,
        [(Some(collegue), json!(false)), (Some(ifdd), Value::Null)],
        "une trace par changement, chacune au nom de qui l'a fait"
    );
}

/// Le journal se lit par entité : une trace sans identifiant ne ressort pas
/// dans l'historique du document.
#[tokio::test]
async fn la_trace_du_choix_se_rattache_a_son_document() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Guide des négociations", false).await;
    deposer(&bac, ifdd, id, "ready").await;
    passer_lextraction(&bac).await;
    choisir(&bac, ifdd, id, Some(true)).await.expect("proposé");

    let par_entite = traces(&bac, "document_renditions", id).await;
    assert!(
        par_entite.contains(&("update".to_owned(), Some(ifdd))),
        "{par_entite:?}"
    );
}

/// Un PDF protégé ne s'extrait pas, et l'aperçu doit dire pourquoi : sans quoi
/// l'administratrice recommencerait le dépôt du même fichier.
#[tokio::test]
async fn un_pdf_protege_par_mot_de_passe_echoue_et_le_dit() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Rapport protégé", false).await;
    let asset = objet_pdf(&bac, ifdd, PROTEGE, "ready").await;
    admin::attacher_le_fichier(&bac.state, &bac.ctx(ifdd), id, asset)
        .await
        .expect("fichier attaché");
    passer_lextraction(&bac).await;

    let etat = rendu(&bac, id).await;
    assert_eq!(etat.status, "failed");
    let motif = etat.failure_reason.expect("un motif");
    assert!(
        motif.contains("mot de passe"),
        "le motif nomme la protection : {motif}"
    );
    let apercu = admin::apercu(&bac.state, id).await.expect("aperçu");
    assert_eq!(
        apercu.extraction.and_then(|e| e.failure_reason).as_deref(),
        Some(motif.as_str()),
        "l'aperçu montre ce motif"
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
    choisir(&bac, ifdd, id, Some(false)).await.expect("retiré");
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
    assert_eq!(
        relancee.large_text_choice,
        Some(false),
        "la relance garde le choix « Texte agrandi »"
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
