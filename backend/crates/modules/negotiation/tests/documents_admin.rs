//! **Le back-office des documents, par les services** : du brouillon à la
//! dépublication, chaque invariant du modèle rendu sous son code stable, la
//! suppression d'un brouillon, et l'auteur de chaque écriture.

mod commun;

use commun::documents::{
    administratrice, creer, entree, entrepots, fichier_publie, objet_pdf, pages,
    passer_lextraction, rendu, BUCKET_PRIVE, PETIT,
};
use commun::{traces, Bac};
use kernel::error::{ApiError, ErrorCode, Result};
use negotiation::domain::admin_documents::{AdminDocument, AdminDocumentInput};
use negotiation::service::admin_documents as admin;
use negotiation::service::documents as public;
use serde_json::{json, Value};
use uuid::Uuid;

const GRAMMAIRE: [&str; 5] = ["heading", "paragraph", "list_item", "note", "origin"];

fn saisie(v: Value) -> AdminDocumentInput {
    serde_json::from_value(v).expect("entrée de document")
}

fn refus<T: std::fmt::Debug>(r: Result<T>) -> ApiError {
    r.expect_err("un refus était attendu")
}

async fn creer_avec(bac: &Bac, auteur: Uuid, v: Value) -> Result<Uuid> {
    admin::creer(&bac.state, &bac.ctx(auteur), &saisie(v), "fr").await
}

async fn modifier(bac: &Bac, auteur: Uuid, id: Uuid, v: Value) -> Result<()> {
    admin::modifier(&bac.state, &bac.ctx(auteur), id, &saisie(v), "fr").await
}

async fn fiche(bac: &Bac, personne: Uuid, id: Uuid) -> Result<AdminDocument> {
    let droits = admin::droits(&bac.state, personne).await?;
    admin::fiche(&bac.state, &droits, id, "fr").await
}

async fn la_fiche(bac: &Bac, personne: Uuid, id: Uuid) -> AdminDocument {
    fiche(bac, personne, id).await.expect("fiche du document")
}

/// Le PDF d'essai déposé dans le bucket privé, puis attaché au document.
async fn attacher(bac: &Bac, auteur: Uuid, id: Uuid) -> Uuid {
    let asset = objet_pdf(bac, auteur, PETIT, "ready").await;
    admin::attacher_le_fichier(&bac.state, &bac.ctx(auteur), id, asset)
        .await
        .expect("fichier attaché");
    asset
}

async fn liens_de_thematiques(bac: &Bac, id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM reference.entity_terms
          WHERE entity_schema = 'negotiation' AND entity_table = 'documents' AND entity_id = $1",
    )
    .bind(id)
    .fetch_one(bac.pool())
    .await
    .expect("liens de thématiques")
}

async fn documents_en_base(bac: &Bac) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM negotiation.documents")
        .fetch_one(bac.pool())
        .await
        .expect("compte des documents")
}

/// Un objet du média décrit en base seulement : le refus se décide sur sa
/// description, avant toute lecture du stockage.
async fn objet_decrit(bac: &Bac, proprietaire: Uuid, bucket: &str, mime: &str) -> Uuid {
    let id = Uuid::now_v7();
    let visibilite = if bucket == BUCKET_PRIVE {
        "private"
    } else {
        "public"
    };
    sqlx::query(
        "INSERT INTO media.assets
             (id, bucket, object_key, checksum_sha256, mime_type, byte_size, owner_person_id,
              visibility)
         VALUES ($1, $2, $3, $4, $5, 1024, $6, $7::text::media.asset_visibility)",
    )
    .bind(id)
    .bind(bucket)
    .bind(format!("essai/{id}"))
    .bind(format!("{:064x}", id.as_u128()))
    .bind(mime)
    .bind(proprietaire)
    .bind(visibilite)
    .execute(bac.pool())
    .await
    .expect("description de l'objet");
    id
}

/// Chaque renvoi du sommaire, enfants compris ; chaque entrée porte un titre.
fn renvois_du_sommaire(entrees: &Value, acc: &mut Vec<u64>) {
    for e in entrees.as_array().expect("des entrées en tableau") {
        assert!(
            e["title"].as_str().is_some_and(|t| !t.trim().is_empty()),
            "{e}"
        );
        acc.push(e["page_index"].as_u64().expect("page_index"));
        renvois_du_sommaire(&e["children"], acc);
    }
}

// -----------------------------------------------------------------------------
// Le chemin nominal
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_document_va_du_brouillon_a_la_depublication_en_passant_par_une_nouvelle_version() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;

    let id = creer_avec(
        &bac,
        ifdd,
        json!({
            "title": { "fr": "Guide des négociations", "en": "Negotiations guide" },
            "type": "negotiation_guide",
            "themes": ["finance", "adaptation"],
            "version": "2025",
            "publisher": "OIF/IFDD",
        }),
    )
    .await
    .expect("brouillon");

    let brouillon = la_fiche(&bac, ifdd, id).await;
    assert_eq!(brouillon.state, "draft");
    assert_eq!(brouillon.source, None, "un document naît sans source");
    assert!(brouillon.asset_id.is_none() && brouillon.external_url.is_none());
    assert!(brouillon.extraction.is_none());
    assert!(!brouillon.file_locked);
    assert_eq!(brouillon.themes, ["finance", "adaptation"]);
    assert!(brouillon.can_publish);
    let vide = admin::apercu(&bac.state, id).await.expect("aperçu");
    assert!(vide.extraction.is_none() && vide.pages.is_empty());

    let asset = attacher(&bac, ifdd, id).await;
    let depose = la_fiche(&bac, ifdd, id).await;
    assert_eq!(depose.source, Some("file"));
    assert_eq!(depose.asset_id, Some(asset));
    let fichier = depose.file.expect("fichier décrit");
    assert_eq!(fichier.mime_type, "application/pdf");
    assert_eq!(fichier.byte_size, PETIT.len() as i64);
    assert_eq!(
        depose.extraction.map(|e| e.status).as_deref(),
        Some("pending"),
        "l'extraction part dans la transaction qui attache le fichier"
    );
    let en_attente = admin::apercu(&bac.state, id).await.expect("aperçu");
    assert!(
        en_attente.pages.is_empty(),
        "aucune page avant l'extraction"
    );

    let issues = passer_lextraction(&bac).await;
    assert!(
        !issues.is_empty() && issues.iter().all(Result::is_ok),
        "{issues:?}"
    );

    let apercu = admin::apercu(&bac.state, id).await.expect("aperçu");
    let extraction = apercu.extraction.expect("état de l'extraction");
    assert_eq!(extraction.status, "ready");
    assert_eq!(extraction.page_count, Some(4));
    assert_eq!(extraction.is_reflowable, Some(true));
    assert!(extraction.extracted_at.is_some());
    assert!(apercu
        .extractor
        .as_deref()
        .is_some_and(|e| e.starts_with("pdfium")));
    let mut renvois = Vec::new();
    renvois_du_sommaire(&apercu.outline, &mut renvois);
    assert!(
        !renvois.is_empty(),
        "petit.pdf porte un sommaire : {}",
        apercu.outline
    );
    assert!(
        renvois.iter().all(|i| (1..=4).contains(i)),
        "le sommaire renvoie à des pages qui existent : {renvois:?}"
    );
    assert_eq!(
        apercu.pages.iter().map(|p| p.index).collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    let blocs_de = |kind: &str| {
        apercu
            .pages
            .iter()
            .flat_map(|p| p.blocks.as_array().expect("des blocs en tableau"))
            .filter(|b| b["kind"] == kind)
            .count()
    };
    let qualite = apercu.quality.clone().expect("indicateurs du verdict");
    assert_eq!(qualite["pages"], 4, "{qualite}");
    assert_eq!(qualite["pages_avec_texte"], 4, "{qualite}");
    assert_eq!(qualite["pages_a_origine"], 1, "{qualite}");
    assert_eq!(
        qualite["entrees_du_sommaire"],
        renvois.len(),
        "l'indicateur compte le sommaire servi : {qualite}"
    );
    assert!(blocs_de("heading") > 0);
    assert_eq!(qualite["titres"], blocs_de("heading"), "{qualite}");
    assert_eq!(qualite["notes"], blocs_de("note"), "{qualite}");
    assert_eq!(
        qualite["tableaux"].as_u64().unwrap() + qualite["figures"].as_u64().unwrap(),
        blocs_de("origin") as u64,
        "{qualite}"
    );
    for page in &apercu.pages {
        assert_eq!(
            page.label,
            page.index.to_string(),
            "le numéro imprimé de petit.pdf suit l'indice"
        );
        assert_eq!(
            page.image.as_deref(),
            Some(
                format!(
                    "/admin/negotiation/documents/{id}/pages/{}/image",
                    page.index
                )
                .as_str()
            ),
            "l'aperçu montre l'image de chaque page, brouillon compris"
        );
        let blocs = page.blocks.as_array().expect("des blocs en tableau");
        assert!(!blocs.is_empty(), "page {} sans bloc", page.index);
        for bloc in blocs {
            let kind = bloc["kind"].as_str().unwrap_or_default();
            assert!(GRAMMAIRE.contains(&kind), "hors grammaire : {bloc}");
        }
    }
    assert_eq!(
        apercu
            .pages
            .iter()
            .map(|p| p.has_origin_block)
            .collect::<Vec<_>>(),
        [false, false, true, false]
    );
    assert!(apercu.pages[2]
        .blocks
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["kind"] == "origin"));

    admin::publier(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("publication");
    let publie = la_fiche(&bac, ifdd, id).await;
    assert_eq!(publie.state, "published");
    assert!(publie.published_at.is_some() && publie.unpublished_at.is_none());
    assert!(publie.file_locked);
    let (bibliotheque, _) = public::bibliotheque(&bac.state, None, "fr")
        .await
        .expect("bibliothèque");
    assert!(bibliotheque.documents.iter().any(|d| d.id == id));

    let nouveau = admin::nouvelle_version(&bac.state, &bac.ctx(ifdd), id, "fr")
        .await
        .expect("nouvelle version");
    let suite = la_fiche(&bac, ifdd, nouveau).await;
    assert_eq!(suite.state, "draft");
    assert_eq!(suite.supersedes.as_ref().map(|l| l.id), Some(id));
    assert_eq!(suite.title, publie.title);
    assert_eq!(suite.type_code, "negotiation_guide");
    assert_eq!(suite.publisher.as_deref(), Some("OIF/IFDD"));
    assert_eq!(
        suite.themes,
        ["finance", "adaptation"],
        "les thématiques sont recopiées"
    );
    assert_eq!(liens_de_thematiques(&bac, nouveau).await, 2);
    assert_eq!(
        suite.source, None,
        "le fichier de la nouvelle version reste à déposer"
    );
    assert_eq!(
        suite.version, "2025 (nouvelle version)",
        "la version reste à saisir"
    );
    let remplace = la_fiche(&bac, ifdd, id).await;
    assert_eq!(remplace.superseded_by.map(|l| l.id), Some(nouveau));
    assert_eq!(
        remplace.state, "published",
        "un brouillon ne remplace encore rien"
    );

    admin::depublier(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("dépublication");
    let retire = la_fiche(&bac, ifdd, id).await;
    assert_eq!(retire.state, "unpublished");
    assert!(retire.published_at.is_none() && retire.unpublished_at.is_some());
    assert!(retire.file_locked, "dépublié, le fichier reste figé");
    let (bibliotheque, _) = public::bibliotheque(&bac.state, None, "fr")
        .await
        .expect("bibliothèque");
    assert!(
        !bibliotheque.documents.iter().any(|d| d.id == id),
        "un document dépublié quitte la bibliothèque"
    );

    let droits = admin::droits(&bac.state, ifdd).await.expect("droits");
    let liste = admin::liste(&bac.state, &droits, "fr")
        .await
        .expect("liste");
    let ligne = |doc: Uuid| liste.documents.iter().find(|l| l.id == doc).unwrap();
    assert_eq!(ligne(id).state, "unpublished");
    assert_eq!(ligne(id).source, Some("file"));
    assert_eq!(
        ligne(id).extraction.as_ref().map(|e| e.status.as_str()),
        Some("ready")
    );
    assert_eq!(ligne(nouveau).state, "draft");
    assert_eq!(ligne(nouveau).supersedes.as_ref().map(|l| l.id), Some(id));
}

// -----------------------------------------------------------------------------
// Chaque invariant traduit
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_fichier_et_un_lien_ensemble_sont_refuses() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;

    let mut e = entree("Bulletin des négociations", false);
    e.external_url = Some(Some("https://enb.iisd.org/cop30".to_owned()));
    let lien = admin::creer(&bac.state, &bac.ctx(ifdd), &e, "fr")
        .await
        .expect("lien en brouillon");
    let asset = objet_pdf(&bac, ifdd, PETIT, "ready").await;
    let err = refus(admin::attacher_le_fichier(&bac.state, &bac.ctx(ifdd), lien, asset).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentSourceBoth, "{err}");
    assert_eq!(
        err.message,
        "Un document est un fichier ou un lien, jamais les deux."
    );
    let reste = la_fiche(&bac, ifdd, lien).await;
    assert_eq!(reste.asset_id, None);
    assert!(reste.extraction.is_none(), "aucune extraction n'est partie");

    let fichier = creer(&bac, ifdd, "Guide des négociations", false).await;
    attacher(&bac, ifdd, fichier).await;
    let err = refus(
        modifier(
            &bac,
            ifdd,
            fichier,
            json!({ "external_url": "https://unfccc.int/cop30" }),
        )
        .await,
    );
    assert_eq!(err.code, ErrorCode::NegotiationDocumentSourceBoth, "{err}");
    assert_eq!(
        err.message,
        "Un document est un fichier ou un lien, jamais les deux."
    );
    assert_eq!(err.field.as_deref(), Some("external_url"));
    assert_eq!(la_fiche(&bac, ifdd, fichier).await.external_url, None);
}

#[tokio::test]
async fn publier_sans_source_est_refuse() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Guide sans fichier", false).await;

    let err = refus(admin::publier(&bac.state, &bac.ctx(ifdd), id).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentSourceMissing,
        "{err}"
    );
    assert_eq!(
        err.message,
        "Déposez un fichier ou indiquez un lien avant de publier."
    );
    assert_eq!(la_fiche(&bac, ifdd, id).await.state, "draft");
}

#[tokio::test]
async fn publier_avant_que_lextraction_soit_prete_est_refuse() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;

    let en_attente = creer(&bac, ifdd, "Guide des négociations", false).await;
    attacher(&bac, ifdd, en_attente).await;
    let err = refus(admin::publier(&bac.state, &bac.ctx(ifdd), en_attente).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentNotReady, "{err}");
    assert_eq!(
        err.message,
        "L'extraction n'est pas terminée. Attendez-la, ou choisissez « ouvrir tel quel »."
    );
    assert_eq!(la_fiche(&bac, ifdd, en_attente).await.state, "draft");

    let refuse = creer(&bac, ifdd, "Rapport refusé par l'analyse", false).await;
    let quarantaine = objet_pdf(&bac, ifdd, PETIT, "quarantined").await;
    admin::attacher_le_fichier(&bac.state, &bac.ctx(ifdd), refuse, quarantaine)
        .await
        .expect("fichier attaché");

    passer_lextraction(&bac).await;
    assert_eq!(rendu(&bac, refuse).await.status, "failed");
    let err = refus(admin::publier(&bac.state, &bac.ctx(ifdd), refuse).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentNotReady,
        "une extraction en échec ne publie pas : {err}"
    );

    admin::publier(&bac.state, &bac.ctx(ifdd), en_attente)
        .await
        .expect("prête, l'extraction ouvre la publication");
    assert_eq!(la_fiche(&bac, ifdd, en_attente).await.state, "published");
}

#[tokio::test]
async fn un_second_successeur_est_refuse_en_nommant_le_premier() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let ancien = creer(&bac, ifdd, "Guide 2024", false).await;
    let successeur = creer_avec(
        &bac,
        ifdd,
        json!({ "title": { "fr": "Guide 2025" }, "type": "negotiation_guide", "supersedes_id": ancien }),
    )
    .await
    .expect("successeur");
    let attendu = "Ce document est déjà remplacé par « Guide 2025 ».";

    let err = refus(
        creer_avec(
            &bac,
            ifdd,
            json!({ "title": { "fr": "Guide 2025 bis" }, "type": "negotiation_guide", "supersedes_id": ancien }),
        )
        .await,
    );
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentAlreadySuperseded,
        "{err}"
    );
    assert_eq!(err.message, attendu, "à la création");
    assert_eq!(err.field.as_deref(), Some("supersedes_id"));

    let autre = creer(&bac, ifdd, "Guide 2025 ter", false).await;
    let err = refus(modifier(&bac, ifdd, autre, json!({ "supersedes_id": ancien })).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentAlreadySuperseded,
        "{err}"
    );
    assert_eq!(err.message, attendu, "à la modification");

    let err = refus(admin::nouvelle_version(&bac.state, &bac.ctx(ifdd), ancien, "fr").await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentAlreadySuperseded,
        "{err}"
    );
    assert_eq!(err.message, attendu, "par « nouvelle version »");

    assert_eq!(
        la_fiche(&bac, ifdd, ancien)
            .await
            .superseded_by
            .map(|l| l.id),
        Some(successeur)
    );
    assert!(la_fiche(&bac, ifdd, autre).await.supersedes.is_none());
}

#[tokio::test]
async fn une_boucle_de_remplacement_est_refusee() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let a = creer(&bac, ifdd, "Guide A", false).await;
    let b = creer_avec(
        &bac,
        ifdd,
        json!({ "title": { "fr": "Guide B" }, "type": "negotiation_guide", "supersedes_id": a }),
    )
    .await
    .expect("B remplace A");

    let err = refus(modifier(&bac, ifdd, a, json!({ "supersedes_id": b })).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentSupersedeCycle,
        "{err}"
    );
    assert_eq!(err.message, "Ce remplacement formerait une boucle.");

    let c = creer_avec(
        &bac,
        ifdd,
        json!({ "title": { "fr": "Guide C" }, "type": "negotiation_guide", "supersedes_id": b }),
    )
    .await
    .expect("C remplace B");
    let err = refus(modifier(&bac, ifdd, a, json!({ "supersedes_id": c })).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentSupersedeCycle,
        "une boucle de trois : {err}"
    );

    let err = refus(modifier(&bac, ifdd, a, json!({ "supersedes_id": a })).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentSupersedeCycle,
        "se remplacer soi-même : {err}"
    );

    assert!(la_fiche(&bac, ifdd, a).await.supersedes.is_none());
}

#[tokio::test]
async fn une_thematique_inconnue_est_refusee_en_la_nommant() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;

    let err = refus(
        creer_avec(
            &bac,
            ifdd,
            json!({
                "title": { "fr": "Guide des négociations" },
                "type": "negotiation_guide",
                "themes": ["finance", "meteo_spatiale"],
            }),
        )
        .await,
    );
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentUnknownTheme,
        "{err}"
    );
    assert_eq!(
        err.message,
        "Cette thématique n'existe pas : meteo_spatiale."
    );
    assert_eq!(err.field.as_deref(), Some("themes"));
    assert_eq!(
        documents_en_base(&bac).await,
        0,
        "le brouillon ne naît pas sans ses thématiques"
    );

    let id = creer_avec(
        &bac,
        ifdd,
        json!({ "title": { "fr": "Guide" }, "type": "negotiation_guide", "themes": ["finance"] }),
    )
    .await
    .expect("brouillon");
    let err = refus(
        modifier(
            &bac,
            ifdd,
            id,
            json!({ "themes": ["adaptation", "negotiation_guide"] }),
        )
        .await,
    );
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentUnknownTheme,
        "un code d'un autre vocabulaire : {err}"
    );
    assert!(err.message.contains("negotiation_guide"), "{}", err.message);
    assert_eq!(
        la_fiche(&bac, ifdd, id).await.themes,
        ["finance"],
        "le refus laisse les thématiques en place"
    );
}

#[tokio::test]
async fn un_type_inconnu_est_refuse() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;

    let err = refus(
        creer_avec(
            &bac,
            ifdd,
            json!({ "title": { "fr": "Guide" }, "type": "roman" }),
        )
        .await,
    );
    assert_eq!(err.code, ErrorCode::NegotiationDocumentUnknownType, "{err}");
    assert_eq!(err.message, "Ce type de document n'existe pas.");
    assert_eq!(err.field.as_deref(), Some("type"));
    assert_eq!(documents_en_base(&bac).await, 0);

    let id = creer(&bac, ifdd, "Guide", false).await;
    let err = refus(modifier(&bac, ifdd, id, json!({ "type": "finance" })).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentUnknownType,
        "un code d'un autre vocabulaire : {err}"
    );
    assert_eq!(
        la_fiche(&bac, ifdd, id).await.type_code,
        "negotiation_guide"
    );
}

#[tokio::test]
async fn le_fichier_dun_document_publie_ne_change_plus() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let avant = la_fiche(&bac, ifdd, id).await.asset_id;
    let autre = objet_pdf(&bac, ifdd, PETIT, "ready").await;

    let err = refus(admin::attacher_le_fichier(&bac.state, &bac.ctx(ifdd), id, autre).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentFileLocked, "{err}");
    let fige = "Le fichier d'un document publié ne change pas. Publiez une nouvelle version.";
    assert_eq!(err.message, fige);
    assert_eq!(err.field.as_deref(), Some("asset_id"));

    let err = refus(
        modifier(
            &bac,
            ifdd,
            id,
            json!({ "external_url": "https://unfccc.int/cop30" }),
        )
        .await,
    );
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentFileLocked,
        "un lien ne remplace pas davantage le fichier : {err}"
    );
    assert_eq!(err.message, fige);
    assert_eq!(err.field.as_deref(), Some("external_url"));
    modifier(&bac, ifdd, id, json!({ "publisher": "OIF/IFDD" }))
        .await
        .expect("le reste de la fiche se modifie");
    assert_eq!(la_fiche(&bac, ifdd, id).await.external_url, None);

    admin::depublier(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("dépublication");
    let err = refus(admin::attacher_le_fichier(&bac.state, &bac.ctx(ifdd), id, autre).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentFileLocked,
        "dépublié, il reste figé : {err}"
    );
    assert_eq!(la_fiche(&bac, ifdd, id).await.asset_id, avant);
}

#[tokio::test]
async fn un_document_publie_ne_se_supprime_pas() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    modifier(&bac, ifdd, id, json!({ "themes": ["gender"] }))
        .await
        .expect("thématique posée");

    let err = refus(admin::supprimer(&bac.state, &bac.ctx(ifdd), id).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentPublishedUndeletable,
        "{err}"
    );
    assert_eq!(
        err.message,
        "Un document publié ne se supprime pas : dépubliez-le."
    );

    admin::depublier(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("dépublication");
    let err = refus(admin::supprimer(&bac.state, &bac.ctx(ifdd), id).await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentPublishedUndeletable,
        "dépublié, il a été publié : {err}"
    );

    assert_eq!(la_fiche(&bac, ifdd, id).await.state, "unpublished");
    assert_eq!(liens_de_thematiques(&bac, id).await, 1);
}

// -----------------------------------------------------------------------------
// La suppression d'un brouillon
// -----------------------------------------------------------------------------

#[tokio::test]
async fn supprimer_un_brouillon_efface_ses_liens_de_thematiques_et_ses_pages() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer_avec(
        &bac,
        ifdd,
        json!({
            "title": { "fr": "Guide à jeter" },
            "type": "negotiation_guide",
            "themes": ["finance", "gender"],
        }),
    )
    .await
    .expect("brouillon");
    attacher(&bac, ifdd, id).await;
    passer_lextraction(&bac).await;
    let cles: Vec<String> = pages(&bac, id)
        .await
        .into_iter()
        .filter_map(|p| p.2)
        .collect();
    assert_eq!(cles.len(), 4);
    assert_eq!(liens_de_thematiques(&bac, id).await, 2);
    let stockage = entrepots(&bac).du_bucket(BUCKET_PRIVE);
    for cle in &cles {
        assert!(
            stockage.get(cle).await.is_ok(),
            "image de page lisible avant : {cle}"
        );
    }

    admin::supprimer(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("suppression du brouillon");

    assert_eq!(
        liens_de_thematiques(&bac, id).await,
        0,
        "ses liens entity_terms partent avec lui"
    );
    let err = refus(fiche(&bac, ifdd, id).await);
    assert_eq!(err.code, ErrorCode::NegotiationDocumentNotFound);
    for cle in &cles {
        assert!(
            stockage.get(cle).await.is_err(),
            "image de page restée : {cle}"
        );
    }
}

// -----------------------------------------------------------------------------
// Le fichier attaché
// -----------------------------------------------------------------------------

#[tokio::test]
async fn attacher_un_objet_qui_nest_pas_un_pdf_du_bucket_prive_est_refuse_sur_asset_id() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = creer(&bac, ifdd, "Guide des négociations", false).await;

    let image_privee = objet_decrit(&bac, ifdd, BUCKET_PRIVE, "image/png").await;
    let pdf_public = objet_decrit(&bac, ifdd, "epavillon", "application/pdf").await;
    let pas_un_pdf_prive =
        "Déposez un PDF avec ce document pour propriétaire : il est alors gardé à l'abri du web.";
    for (objet, cas, message) in [
        (image_privee, "une image du bucket privé", pas_un_pdf_prive),
        (pdf_public, "un PDF du bucket public", pas_un_pdf_prive),
        (
            Uuid::now_v7(),
            "un objet inconnu",
            "Ce fichier n'existe pas, ou a été supprimé.",
        ),
    ] {
        let err = refus(admin::attacher_le_fichier(&bac.state, &bac.ctx(ifdd), id, objet).await);
        assert_eq!(err.code, ErrorCode::ValidationFailed, "{cas} : {err}");
        assert_eq!(err.field.as_deref(), Some("asset_id"), "{cas}");
        assert_eq!(err.message, message, "{cas}");
    }

    let reste = la_fiche(&bac, ifdd, id).await;
    assert_eq!(reste.asset_id, None);
    assert!(reste.extraction.is_none(), "aucune extraction n'est partie");

    let pdf_prive = objet_decrit(&bac, ifdd, BUCKET_PRIVE, "application/pdf").await;
    admin::attacher_le_fichier(&bac.state, &bac.ctx(ifdd), id, pdf_prive)
        .await
        .expect("un PDF du bucket privé s'attache");
    let attache = la_fiche(&bac, ifdd, id).await;
    assert_eq!(attache.asset_id, Some(pdf_prive));
    assert_eq!(
        attache.extraction.map(|e| e.status).as_deref(),
        Some("pending")
    );
}

// -----------------------------------------------------------------------------
// L'audit
// -----------------------------------------------------------------------------

#[tokio::test]
async fn chaque_ecriture_du_back_office_nomme_son_autrice() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let actions =
        |t: &[(String, Option<Uuid>)]| t.iter().map(|(a, _)| a.clone()).collect::<Vec<_>>();
    let par_elle = |t: &[(String, Option<Uuid>)]| t.iter().all(|(_, acteur)| *acteur == Some(ifdd));

    let id = creer_avec(
        &bac,
        ifdd,
        json!({ "title": { "fr": "Guide" }, "type": "negotiation_guide", "themes": ["finance"] }),
    )
    .await
    .expect("création");
    modifier(
        &bac,
        ifdd,
        id,
        json!({ "summary": { "fr": "Ce qu'il faut savoir avant Belém." } }),
    )
    .await
    .expect("modification");
    attacher(&bac, ifdd, id).await;
    passer_lextraction(&bac).await;
    admin::publier(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("publication");
    let nouveau = admin::nouvelle_version(&bac.state, &bac.ctx(ifdd), id, "fr")
        .await
        .expect("nouvelle version");
    admin::depublier(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("dépublication");

    let sur_le_document = traces(&bac, "documents", id).await;
    assert_eq!(
        actions(&sur_le_document),
        ["insert", "update", "update", "update", "update"],
        "créer, modifier, attacher, publier, dépublier : {sur_le_document:?}"
    );
    assert!(par_elle(&sur_le_document), "{sur_le_document:?}");

    let sur_la_version = traces(&bac, "documents", nouveau).await;
    assert_eq!(actions(&sur_la_version), ["insert"]);
    assert!(par_elle(&sur_la_version), "{sur_la_version:?}");

    admin::supprimer(&bac.state, &bac.ctx(ifdd), nouveau)
        .await
        .expect("suppression du brouillon");
    let apres = traces(&bac, "documents", nouveau).await;
    assert_eq!(actions(&apres), ["insert", "delete"]);
    assert!(par_elle(&apres), "{apres:?}");
}
