//! Le décor des documents de Guide Négo : un brouillon, un PDF déposé dans le
//! bucket privé, et le passage du travail d'extraction.

use kernel::jobs::{self, JobHandler};
use kernel::storage::Entrepots;
use negotiation::jobs::extract::ExtractDocument;
use uuid::Uuid;

use super::Bac;

pub const BUCKET_PRIVE: &str = "epavillon-prive";
pub const PETIT: &[u8] = include_bytes!("../fixtures/petit.pdf");

/// Un brouillon, sans source : c'est ainsi qu'un document naît.
pub async fn brouillon(bac: &Bac, slug: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO negotiation.documents (slug, title, document_type_term_id)
           SELECT $1::text::platform.slug, '{"fr":"Guide d''essai"}'::jsonb::platform.i18n_text, t.id
             FROM reference.taxonomy_terms t
            WHERE t.taxonomy_code = 'document_type' AND t.code = 'negotiation_guide'
           RETURNING id"#,
        slug
    )
    .fetch_one(bac.pool())
    .await
    .expect("brouillon")
}

pub fn entrepots(bac: &Bac) -> Entrepots {
    Entrepots::new(&bac.config.media)
}

/// Un PDF déposé dans le bucket privé, décrit en base comme le média le ferait,
/// dans l'état voulu (`ready`, `scanning`, `quarantined`…).
pub async fn objet_pdf(bac: &Bac, proprietaire: Uuid, octets: &[u8], statut: &str) -> Uuid {
    let id = Uuid::now_v7();
    let cle = format!("essai/{id}.pdf");
    entrepots(bac)
        .du_bucket(BUCKET_PRIVE)
        .put(&cle, "application/pdf", octets.to_vec())
        .await
        .expect("dépôt de l'objet");
    let verdict = if statut == "ready" {
        "clean"
    } else {
        "pending"
    };
    sqlx::query!(
        "INSERT INTO media.assets
             (id, bucket, object_key, checksum_sha256, mime_type, byte_size, owner_person_id,
              visibility, status, scan_verdict)
         VALUES ($1, $2, $3, $4, 'application/pdf', $5, $6, 'private',
                 $7::text::media.asset_status, $8::text::media.scan_verdict)",
        id,
        BUCKET_PRIVE,
        cle,
        format!("{:064x}", id.as_u128()),
        octets.len() as i64,
        proprietaire,
        statut,
        verdict
    )
    .execute(bac.pool())
    .await
    .expect("description de l'objet");
    id
}

pub fn extracteur(bac: &Bac) -> ExtractDocument {
    ExtractDocument::new(
        bac.db(),
        entrepots(bac),
        bac.config.negotiation.pdfium_lib_path.clone(),
    )
}

/// Met en file l'extraction, comme le fait l'attache du fichier.
pub async fn demander_lextraction(bac: &Bac, document: Uuid, asset: Uuid, acteur: Uuid) {
    let mut tx = bac.db().write(&bac.ctx(acteur)).await.expect("transaction");
    negotiation::jobs::extract::mettre_en_file(&mut tx, document, asset, acteur, Uuid::now_v7())
        .await
        .expect("mise en file");
    tx.commit().await.expect("validation");
}

/// Un passage du worker sur la file par défaut, avec le seul extracteur : rend
/// l'issue de chaque travail exécuté.
pub async fn passer_lextraction(bac: &Bac) -> Vec<kernel::error::Result<()>> {
    let gestionnaire = extracteur(bac);
    let mut issues = Vec::new();
    loop {
        let mut tx = bac
            .db()
            .write(&bac.ctx_anonyme())
            .await
            .expect("transaction");
        let travaux = jobs::claim(&mut tx, gestionnaire.queue(), "test-worker", 10)
            .await
            .expect("réservation");
        tx.commit().await.expect("validation");
        if travaux.is_empty() {
            break;
        }
        for travail in travaux
            .into_iter()
            .filter(|t| t.task == gestionnaire.task())
        {
            let issue = gestionnaire.run(&travail).await;
            let mut tx = bac
                .db()
                .write(&bac.ctx_anonyme())
                .await
                .expect("transaction");
            match &issue {
                Ok(()) => jobs::succeed(&mut tx, travail.id).await.expect("succès"),
                Err(e) => jobs::fail(&mut tx, travail.id, &e.to_string())
                    .await
                    .expect("échec"),
            }
            tx.commit().await.expect("validation");
            issues.push(issue);
        }
    }
    issues
}

pub struct EtatDuRendu {
    pub status: String,
    pub page_count: Option<i32>,
    pub failure_reason: Option<String>,
    pub is_reflowable: Option<bool>,
    pub reading_bytes: Option<i64>,
}

pub async fn rendu(bac: &Bac, document: Uuid) -> EtatDuRendu {
    let l = sqlx::query!(
        r#"SELECT status::text AS "status!", page_count, failure_reason, is_reflowable, reading_bytes
             FROM negotiation.document_renditions WHERE document_id = $1"#,
        document
    )
    .fetch_one(bac.pool())
    .await
    .expect("rendu");
    EtatDuRendu {
        status: l.status,
        page_count: l.page_count,
        failure_reason: l.failure_reason,
        is_reflowable: l.is_reflowable,
        reading_bytes: l.reading_bytes,
    }
}

/// Les pages en base : index, étiquette, clé d'image, page d'origine.
pub async fn pages(bac: &Bac, document: Uuid) -> Vec<(i32, String, Option<String>, bool)> {
    sqlx::query!(
        "SELECT page_index, label, image_key, has_origin_block
           FROM negotiation.document_pages WHERE document_id = $1 ORDER BY page_index",
        document
    )
    .fetch_all(bac.pool())
    .await
    .expect("pages")
    .into_iter()
    .map(|l| (l.page_index, l.label, l.image_key, l.has_origin_block))
    .collect()
}

// -----------------------------------------------------------------------------
// Les profils et les documents publiés
// -----------------------------------------------------------------------------

/// Une administratrice de la plateforme entière : elle publie, et ne corrige pas.
pub async fn administratrice(bac: &Bac, email: &str) -> Uuid {
    let p = super::personne(bac, email).await;
    super::attribuer(bac, p, "admin", "global", None).await;
    p
}

/// Une personne qui a l'accès négociateur de Guide Négo.
pub async fn negociatrice(bac: &Bac, email: &str) -> Uuid {
    let p = super::personne(bac, email).await;
    super::attribuer(bac, p, "negotiator", "global", None).await;
    p
}

/// Un expert : il pose et retire les notes, et ne publie rien.
pub async fn expert(bac: &Bac, email: &str) -> Uuid {
    let p = super::personne(bac, email).await;
    super::attribuer(bac, p, "expert", "global", None).await;
    p
}

pub fn entree(
    titre: &str,
    restreint: bool,
) -> negotiation::domain::admin_documents::AdminDocumentInput {
    serde_json::from_value(serde_json::json!({
        "title": { "fr": titre },
        "type": "negotiation_guide",
        "restricted": restreint,
    }))
    .expect("entrée de document")
}

/// Un brouillon créé par le service, comme au back-office.
pub async fn creer(bac: &Bac, admin: Uuid, titre: &str, restreint: bool) -> Uuid {
    negotiation::service::admin_documents::creer(
        &bac.state,
        &bac.ctx(admin),
        &entree(titre, restreint),
    )
    .await
    .expect("création du brouillon")
}

/// Un document fichier publié : brouillon, PDF déposé dans le bucket privé,
/// fichier attaché, extraction passée, publication.
pub async fn fichier_publie(bac: &Bac, admin: Uuid, titre: &str, restreint: bool) -> Uuid {
    let id = creer(bac, admin, titre, restreint).await;
    let asset = objet_pdf(bac, admin, PETIT, "ready").await;
    negotiation::service::admin_documents::attacher_le_fichier(
        &bac.state,
        &bac.ctx(admin),
        id,
        asset,
    )
    .await
    .expect("fichier attaché");
    let issues = passer_lextraction(bac).await;
    assert!(issues.iter().all(Result::is_ok), "{issues:?}");
    negotiation::service::admin_documents::publier(&bac.state, &bac.ctx(admin), id)
        .await
        .expect("publication");
    id
}

/// Un lien externe publié.
pub async fn lien_publie(bac: &Bac, admin: Uuid, titre: &str, url: &str, restreint: bool) -> Uuid {
    let mut e = entree(titre, restreint);
    e.external_url = Some(Some(url.to_owned()));
    let id = negotiation::service::admin_documents::creer(&bac.state, &bac.ctx(admin), &e)
        .await
        .expect("création du lien");
    negotiation::service::admin_documents::publier(&bac.state, &bac.ctx(admin), id)
        .await
        .expect("publication du lien");
    id
}
