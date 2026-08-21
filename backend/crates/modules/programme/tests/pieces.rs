//! **Rattacher, lire, détacher — et l'objet stocké toujours là après.**
//!
//! Le module ne détruit pas ce qu'il n'a pas créé : `media.assets` a son propre
//! cycle de vie, et un même objet peut être rattaché à deux dossiers. Le test
//! qui compte ici est le dernier — celui qui relit l'objet **après** le
//! détachement.

mod commun;

use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::ProposalId;
use programme::service::documents::{self, AttachDocumentPayload};
use uuid::Uuid;

/// Un objet stocké, prêt à être servi.
async fn objet(bac: &Bac, terrain: &Terrain, nom: &str, statut: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (bucket, object_key, checksum_sha256, mime_type, byte_size,
                original_filename, owner_organization_id, status, scan_verdict)
           VALUES ('epavillon', '2027/03/' || gen_random_uuid()::text || '/' || $1,
                   md5(gen_random_uuid()::text) || md5(gen_random_uuid()::text),
                   'application/pdf', 2048, $1, $2,
                   $3::text::media.asset_status,
                   CASE WHEN $3 = 'ready' THEN 'clean' ELSE 'pending' END::media.scan_verdict)
        RETURNING id"#,
        nom,
        terrain.organisation,
        statut
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'objet stocké")
}

#[tokio::test]
async fn une_piece_se_rattache_se_lit_et_se_detache_sans_detruire_lobjet() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;
    let id = ProposalId(dossier);
    let asset = objet(&bac, &terrain, "note-de-cadrage.pdf", "ready").await;

    let piece = documents::rattacher(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        id,
        AttachDocumentPayload {
            asset_id: asset,
            title: String::new(),
            document_type_code: None,
            is_public: false,
            sort_order: 0,
        },
    )
    .await
    .expect("le rattachement");

    // **Le titre par défaut est le nom du fichier** : une pièce sans titre
    // s'affiche « Document » dans une liste, et personne ne sait laquelle
    // ouvrir.
    assert_eq!(piece.title["fr"], "note-de-cadrage.pdf");
    assert!(!piece.is_public, "une pièce est interne par défaut");

    let pieces = documents::lister(&bac.state, terrain.deposante, id)
        .await
        .expect("la lecture");
    assert_eq!(pieces.len(), 1);
    assert!(
        pieces[0].url.is_some(),
        "l'objet est servi, l'adresse existe"
    );
    assert_eq!(
        pieces[0].asset.as_ref().map(|a| a.status.as_str()),
        Some("ready")
    );

    documents::detacher(&bac.state, &bac.ctx(), terrain.deposante, id, piece.id)
        .await
        .expect("le détachement");

    assert!(documents::lister(&bac.state, terrain.deposante, id)
        .await
        .expect("la lecture")
        .is_empty());

    // **L'OBJET EST TOUJOURS LÀ.** C'est l'assertion pour laquelle ce test
    // existe : détruire ici effacerait la pièce d'un autre dossier sans le
    // savoir.
    let survivant = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM media.assets WHERE id = $1 AND deleted_at IS NULL
           ) AS "existe!""#,
        asset
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture de l'objet");
    assert!(survivant, "le module ne détruit pas ce qu'il n'a pas créé");

    // Détacher deux fois rend un refus, pas un succès silencieux.
    let refus = documents::detacher(&bac.state, &bac.ctx(), terrain.deposante, id, piece.id)
        .await
        .expect_err("la pièce n'existe plus");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

/// Un objet **inconnu** est refusé en nommant le champ — la clé étrangère
/// refuserait aussi, mais son message ne dirait pas lequel.
#[tokio::test]
async fn un_objet_inconnu_est_refuse_en_nommant_le_champ() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;

    let refus = documents::rattacher(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(dossier),
        AttachDocumentPayload {
            asset_id: Uuid::now_v7(),
            title: "Une pièce".to_owned(),
            document_type_code: None,
            is_public: false,
            sort_order: 0,
        },
    )
    .await
    .expect_err("un objet inconnu est refusé");

    assert_eq!(refus.code, ErrorCode::ProposalUnknownReference);
    assert_eq!(refus.field.as_deref(), Some("asset_id"));
}

/// **Une personne sans accès au dossier est refusée**, et une pièce interne ne
/// sort par aucune de ses lectures.
#[tokio::test]
async fn une_personne_sans_acces_ne_lit_ni_ne_rattache() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;
    let id = ProposalId(dossier);
    let asset = objet(&bac, &terrain, "interne.pdf", "ready").await;

    documents::rattacher(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        id,
        AttachDocumentPayload {
            asset_id: asset,
            title: "Pièce interne".to_owned(),
            document_type_code: None,
            is_public: false,
            sort_order: 0,
        },
    )
    .await
    .expect("le rattachement");

    let quidam = commun::personne(&bac, "quidam@example.org", "Quid", "Am").await;
    let refus = documents::lister(&bac.state, quidam, id)
        .await
        .expect_err("une personne sans accès ne lit pas les pièces");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let refus = documents::rattacher(
        &bac.state,
        &bac.ctx(),
        quidam,
        id,
        AttachDocumentPayload {
            asset_id: asset,
            title: "Une pièce".to_owned(),
            document_type_code: None,
            is_public: false,
            sort_order: 0,
        },
    )
    .await
    .expect_err("ni n'en rattache");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // **Le comité, lui, y accède** — la seconde voie.
    let droits = commun::droits(&bac, &terrain).await;
    let pieces = documents::lister(&bac.state, droits.decideur, id)
        .await
        .expect("le comité lit les pièces du dossier");
    assert_eq!(pieces.len(), 1);
    assert!(
        !pieces[0].document.is_public,
        "la pièce reste interne : c'est le comité qui la lit, pas le public"
    );
}
