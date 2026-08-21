//! Les pièces du dossier, et **l'adresse composée en base**.
//!
//! # L'adresse est nulle quand l'objet n'est pas servi, et c'est le message
//!
//! `media.assets.status` ne vaut `ready` qu'après l'analyse antivirus — la
//! contrainte `ck_assets_scan_before_ready` l'impose. Une pièce en
//! quarantaine, purgée, ou dont le téléversement n'est pas achevé doit être
//! **annoncée** au comité et non proposée au téléchargement : il doit savoir
//! qu'une pièce manque à son dossier, pas cliquer sur un lien mort.
//!
//! C'est cette nullité qui commande l'avertissement plutôt que le bouton, et
//! c'est pourquoi l'adresse est calculée **avec** la condition, en base, et non
//! recomposée par un composant à partir du couple `(bucket, object_key)`.
//!
//! # Ce module ne pose ni ne détruit l'objet
//!
//! Le cycle de vie du fichier appartient à B6. Ici, on rattache, on détache, et
//! on lit.

use kernel::error::Result;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::ProposalId;

/// Une pièce, son objet stocké et son adresse — `ProposalDocumentEntry`.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct PieceDuDossier {
    pub document: Piece,
    pub asset: Option<ObjetStocke>,
    /// **Nulle quand l'objet n'est pas servi.**
    pub url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct Piece {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub asset_id: Uuid,
    pub title: serde_json::Value,
    pub document_type_code: Option<String>,
    pub is_public: bool,
    pub uploaded_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub uploaded_at: OffsetDateTime,
    pub sort_order: i16,
}

/// Ce que ce module lit d'un objet stocké — et rien de plus. La fiche complète
/// de l'objet appartient au module Médias.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct ObjetStocke {
    pub id: Uuid,
    pub original_filename: Option<String>,
    pub mime_type: String,
    pub byte_size: i64,
    pub status: String,
    /// `pending`, `clean`, `infected`, `unsupported` — c'est lui que l'écran
    /// nomme quand l'adresse est nulle.
    pub scan_verdict: String,
}

/// Les pièces d'un dossier, avec leur objet et leur adresse.
pub async fn du_dossier<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<PieceDuDossier>> {
    let lignes = sqlx::query!(
        r#"SELECT d.id, d.proposal_id, d.asset_id, d.title, d.document_type_code,
                  d.is_public, d.uploaded_by, d.uploaded_at, d.sort_order,
                  a.id AS "asset_id_reel?", a.original_filename,
                  a.mime_type AS "mime_type?", a.byte_size AS "byte_size?",
                  a.status::text AS "status?",
                  a.scan_verdict::text AS "scan_verdict?",
                  CASE WHEN a.status = 'ready' AND a.deleted_at IS NULL
                       THEN media.object_url(a.bucket, a.object_key) END AS "url?"
             FROM programme.proposal_documents d
             LEFT JOIN media.assets a ON a.id = d.asset_id
            WHERE d.proposal_id = $1
            ORDER BY d.sort_order, d.uploaded_at"#,
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PieceDuDossier {
            document: Piece {
                id: l.id,
                proposal_id: l.proposal_id,
                asset_id: l.asset_id,
                title: l.title,
                document_type_code: l.document_type_code,
                is_public: l.is_public,
                uploaded_by: l.uploaded_by,
                uploaded_at: l.uploaded_at,
                sort_order: l.sort_order,
            },
            asset: match (
                l.asset_id_reel,
                l.mime_type,
                l.byte_size,
                l.status,
                l.scan_verdict,
            ) {
                (Some(id), Some(mime_type), Some(byte_size), Some(status), Some(scan_verdict)) => {
                    Some(ObjetStocke {
                        id,
                        original_filename: l.original_filename,
                        mime_type,
                        byte_size,
                        status,
                        scan_verdict,
                    })
                }
                _ => None,
            },
            url: l.url,
        })
        .collect())
}

// -----------------------------------------------------------------------------
// Rattacher et détacher — **jamais poser ni détruire l'objet**
// -----------------------------------------------------------------------------

/// Ce qu'un rattachement pose.
pub struct NouvellePiece<'a> {
    pub asset_id: Uuid,
    pub title: serde_json::Value,
    pub document_type_code: Option<&'a str>,
    pub is_public: bool,
    pub sort_order: i16,
}

/// Rattacher un objet **déjà stocké** au dossier.
///
/// Le téléversement appartient à B6 : ce module reçoit un identifiant d'objet
/// et le rattache. Une clé étrangère inconnue rend `PROPOSAL_UNKNOWN_REFERENCE`,
/// traduit par le service en nommant le champ.
pub async fn rattacher(
    conn: &mut sqlx::postgres::PgConnection,
    dossier: ProposalId,
    par: Uuid,
    nouvelle: &NouvellePiece<'_>,
) -> Result<Piece> {
    let ligne = sqlx::query!(
        r#"INSERT INTO programme.proposal_documents
               (proposal_id, asset_id, title, document_type_code, is_public,
                uploaded_by, sort_order)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, proposal_id, asset_id, title, document_type_code,
                  is_public, uploaded_by, uploaded_at, sort_order"#,
        dossier.as_uuid(),
        nouvelle.asset_id,
        nouvelle.title,
        nouvelle.document_type_code,
        nouvelle.is_public,
        par,
        nouvelle.sort_order
    )
    .fetch_one(conn)
    .await?;

    Ok(Piece {
        id: ligne.id,
        proposal_id: ligne.proposal_id,
        asset_id: ligne.asset_id,
        title: ligne.title,
        document_type_code: ligne.document_type_code,
        is_public: ligne.is_public,
        uploaded_by: ligne.uploaded_by,
        uploaded_at: ligne.uploaded_at,
        sort_order: ligne.sort_order,
    })
}

/// **Détacher la pièce — l'objet stocké demeure.**
///
/// Le module ne détruit pas ce qu'il n'a pas créé : `media.assets` a son propre
/// cycle de vie — suppression logique, date de purge, worker de purge — et un
/// même objet peut être rattaché ailleurs. Détruire ici, c'est effacer la pièce
/// d'un autre dossier sans le savoir.
pub async fn detacher(
    conn: &mut sqlx::postgres::PgConnection,
    dossier: ProposalId,
    piece: Uuid,
) -> Result<bool> {
    let effacees = sqlx::query!(
        "DELETE FROM programme.proposal_documents WHERE id = $1 AND proposal_id = $2",
        piece,
        dossier.as_uuid()
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(effacees == 1)
}
