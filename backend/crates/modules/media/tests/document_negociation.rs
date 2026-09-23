//! **Le PDF d'un document de Guide Négo** (specs/011 R5) : la permission de
//! publier sur la portée globale, un PDF seul, et un dépôt toujours privé.

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use media::service::upload::MetadonneesDepot;
use uuid::Uuid;

async fn document(bac: &Bac) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO negotiation.documents (slug, title, document_type_term_id, external_url)
         SELECT 'guide-essai', '{\"fr\":\"Guide d''essai\"}'::jsonb::platform.i18n_text, t.id,
                'https://example.org/guide.pdf'
           FROM reference.taxonomy_terms t
          WHERE t.taxonomy_code = 'document_type'
          LIMIT 1
         RETURNING id",
    )
    .fetch_one(bac.pool())
    .await
    .expect("document de négociation")
}

async fn administratrice_globale(bac: &Bac) -> Uuid {
    let personne = commun::personne(bac, "ifdd@example.org", "Nadia", "Koné").await;
    commun::attribuer(bac, personne, "admin", "global", None).await;
    personne
}

fn pour_le_document(fichier: &commun::Fichier, document: Uuid) -> MetadonneesDepot {
    MetadonneesDepot {
        owner_schema: Some("negotiation".to_owned()),
        owner_table: Some("documents".to_owned()),
        owner_id: Some(document),
        ..commun::metadonnees(fichier)
    }
}

#[tokio::test]
async fn le_pdf_dun_document_se_depose_prive_meme_demande_public() {
    let bac = Bac::monter().await;
    let document = document(&bac).await;
    let ifdd = administratrice_globale(&bac).await;
    let pdf = commun::document_pdf();

    let resultat = commun::deposer(
        &bac,
        ifdd,
        &pdf,
        MetadonneesDepot {
            visibility: Some("public".to_owned()),
            ..pour_le_document(&pdf, document)
        },
    )
    .await
    .expect("dépôt du PDF");

    assert_eq!(resultat.asset.visibility, "private");
    assert_eq!(resultat.asset.bucket, "epavillon-prive");
}

#[tokio::test]
async fn un_fichier_qui_nest_pas_un_pdf_est_refuse() {
    let bac = Bac::monter().await;
    let document = document(&bac).await;
    let ifdd = administratrice_globale(&bac).await;
    let image = commun::vignette_1_1();

    let erreur = commun::deposer(&bac, ifdd, &image, pour_le_document(&image, document))
        .await
        .expect_err("une image n'est pas un document");

    assert_eq!(erreur.code, ErrorCode::MediaMimeNotAllowed);
    assert_eq!(erreur.field.as_deref(), Some("file"));
}

/// Pour Guide Négo, « son périmètre » veut dire la plateforme entière :
/// l'administratrice d'une seule édition porte la permission sur sa COP, et ne
/// passe pas.
#[tokio::test]
async fn sans_la_permission_globale_le_depot_est_refuse_comme_un_inexistant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let document = document(&bac).await;
    let pdf = commun::document_pdf();

    for personne in [terrain.administratrice, terrain.etrangere] {
        let erreur = commun::deposer(&bac, personne, &pdf, pour_le_document(&pdf, document))
            .await
            .expect_err("sans la permission globale");
        assert_eq!(erreur.code, ErrorCode::NotFound);
    }

    let ifdd = administratrice_globale(&bac).await;
    let inexistant = commun::deposer(&bac, ifdd, &pdf, pour_le_document(&pdf, Uuid::now_v7()))
        .await
        .expect_err("un document inexistant");
    assert_eq!(inexistant.code, ErrorCode::NotFound);
}
