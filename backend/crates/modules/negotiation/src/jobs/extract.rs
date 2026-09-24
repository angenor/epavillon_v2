//! L'extraction d'un document : du PDF déposé à la forme lisible en base et aux
//! images de ses pages dans le bucket privé (R6 de specs/011).
//!
//! **Mise en file dans la transaction qui attache le fichier.** Le travail
//! attend que le média ait analysé l'objet : tant qu'il ne l'a pas fait, il se
//! replanifie, de plus en plus tard. Une analyse négative, ou un fichier qui ne
//! s'ouvre pas, conclut « échec » avec un motif lisible à l'aperçu.

use std::sync::Arc;

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use kernel::storage::{Entrepots, ObjectStore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::documents::serialiser_la_lecture;
use crate::domain::extraction::{self, forme::Block, Extraction};
use crate::pdf::LecteurPdf;
use crate::repo::{document_pages, documents, objets, renditions};
use crate::service::documents as service;

pub const EXTRACT_DOCUMENT: &str = "negotiation.document.extract";

/// Combien de fois le travail attend l'analyse du fichier : de quinze secondes
/// en doublant, plafonné à dix minutes — un peu plus d'une heure en tout.
const ATTENTES_MAX: u32 = 12;

#[derive(Debug, Serialize, Deserialize)]
struct Charge {
    document_id: Uuid,
    asset_id: Uuid,
    /// La personne qui a attaché le fichier : l'extraction écrit en son nom.
    acteur: Uuid,
    /// Seule la dernière demande conclut : une relance rend ce travail caduc.
    #[serde(default)]
    demande: Uuid,
    #[serde(default)]
    attente: u32,
}

/// Met en file l'extraction d'un fichier. La clé d'unicité porte le document,
/// l'objet et la demande : une relance est une demande de plus.
pub async fn mettre_en_file(
    conn: &mut PgConnection,
    document_id: Uuid,
    asset_id: Uuid,
    acteur: Uuid,
    demande: Uuid,
) -> Result<()> {
    renditions::demander(conn, document_id, asset_id, demande).await?;
    jobs::enqueue(
        conn,
        NewJob::new(
            EXTRACT_DOCUMENT,
            json!(Charge {
                document_id,
                asset_id,
                acteur,
                demande,
                attente: 0
            }),
        )
        .idempotent(format!(
            "{EXTRACT_DOCUMENT}:{document_id}:{asset_id}:{demande}"
        )),
    )
    .await?;
    Ok(())
}

fn delai(attente: u32) -> Duration {
    Duration::seconds((15_i64 << attente.min(6)).min(600))
}

pub struct ExtractDocument {
    db: Db,
    entrepots: Entrepots,
    pdfium: Option<String>,
    bucket_prive: std::sync::OnceLock<String>,
}

impl ExtractDocument {
    pub fn new(db: Db, entrepots: Entrepots, pdfium: Option<String>) -> Self {
        Self {
            db,
            entrepots,
            pdfium,
            bucket_prive: std::sync::OnceLock::new(),
        }
    }

    /// PDFium se charge au premier document, et une seule fois.
    fn lecteur(&self) -> std::result::Result<Arc<LecteurPdf>, String> {
        let dossier = self
            .pdfium
            .as_deref()
            .ok_or("PDFIUM_LIB_PATH n'est pas renseigné : le worker ne peut pas lire de PDF")?;
        LecteurPdf::partage(dossier)
    }

    async fn bucket_prive(&self) -> Result<String> {
        if let Some(b) = self.bucket_prive.get() {
            return Ok(b.clone());
        }
        let bucket = sqlx::query_scalar!(
            "SELECT s.value #>> '{}' FROM platform.settings s WHERE s.key = 'media.private_bucket'"
        )
        .fetch_optional(self.db.pool())
        .await?
        .flatten()
        .ok_or_else(|| ApiError::internal("réglage media.private_bucket absent"))?;
        Ok(self.bucket_prive.get_or_init(|| bucket).clone())
    }

    async fn conclure_en_echec(&self, job: &ClaimedJob, c: &Charge, motif: &str) -> Result<()> {
        let mut tx = self.db.write(&job.context().with_actor(c.acteur)).await?;
        renditions::echouer(&mut tx, c.document_id, c.asset_id, c.demande, motif).await?;
        tx.commit().await?;
        tracing::warn!(document = %c.document_id, motif, "extraction en échec");
        Ok(())
    }
}

#[async_trait]
impl JobHandler for ExtractDocument {
    fn task(&self) -> &'static str {
        EXTRACT_DOCUMENT
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|e| ApiError::internal(format!("charge d'extraction illisible : {e}")))?;
        // Au-delà du dernier essai, seul un travail repris après la chute du
        // worker revient ici : le fichier l'a fait tomber, on n'y retouche pas.
        if job.attempts > job.max_attempts {
            self.effacer_lessai_tombe(&charge).await;
            self.conclure_en_echec(
                job,
                &charge,
                "Le fichier a interrompu l'extraction à plusieurs reprises ; il est à vérifier.",
            )
            .await?;
            return Err(ApiError::internal(
                "extraction abandonnée : le worker est tombé sur ce fichier",
            ));
        }
        match self.extraire(job, &charge).await {
            Ok(()) => Ok(()),
            Err(erreur) => {
                if job.attempts >= job.max_attempts {
                    self.conclure_en_echec(
                        job,
                        &charge,
                        "L'extraction a échoué plusieurs fois ; elle peut être relancée.",
                    )
                    .await?;
                }
                Err(erreur)
            }
        }
    }
}

impl ExtractDocument {
    async fn extraire(&self, job: &ClaimedJob, c: &Charge) -> Result<()> {
        let mut conn = self.db.pool().acquire().await?;
        let Some(objet) = objets::emplacement(&mut conn, c.asset_id).await? else {
            return self
                .conclure_en_echec(job, c, "Le fichier a été supprimé.")
                .await;
        };
        match objet.status.as_str() {
            "ready" => {}
            "quarantined" => {
                return self
                    .conclure_en_echec(job, c, "L'analyse antivirus a refusé ce fichier.")
                    .await
            }
            "failed" => {
                return self
                    .conclure_en_echec(job, c, "Le traitement du fichier a échoué au dépôt.")
                    .await
            }
            _ if c.attente >= ATTENTES_MAX => {
                return self
                    .conclure_en_echec(job, c, "L'analyse du fichier n'a pas abouti à temps.")
                    .await
            }
            _ => {
                // Pas encore analysé : on repasse plus tard, sans consommer d'essai.
                let suivante = Charge {
                    attente: c.attente + 1,
                    ..*c
                };
                jobs::enqueue(
                    &mut conn,
                    NewJob::new(EXTRACT_DOCUMENT, json!(suivante))
                        .idempotent(format!("{}:attente-{}", job.id, suivante.attente))
                        .at(OffsetDateTime::now_utc() + delai(c.attente)),
                )
                .await?;
                return Ok(());
            }
        }
        drop(conn);

        let mut tx = self.db.write(&job.context().with_actor(c.acteur)).await?;
        let concerne = renditions::commencer(&mut tx, c.document_id, c.asset_id, c.demande).await?;
        let anciennes = document_pages::cles_d_images(&mut tx, c.document_id).await?;
        tx.commit().await?;
        if !concerne {
            return Ok(());
        }

        let octets = self
            .entrepots
            .du_bucket(&objet.bucket)
            .get(&objet.object_key)
            .await?;
        let lecteur = match self.lecteur() {
            Ok(l) => l,
            Err(motif) => return Err(ApiError::internal(motif)),
        };
        let bucket = self.bucket_prive().await?;
        let stockage = self.entrepots.du_bucket(&bucket);

        // Chaque image part au bucket dès qu'elle est rendue.
        let (envoi, mut rendues) = tokio::sync::mpsc::channel::<Vec<u8>>(2);
        let lecture = tokio::task::spawn_blocking(move || {
            let texte = lecteur.lire(&octets);
            let rendu = lecteur.rendre(&octets, |jpeg| {
                envoi
                    .blocking_send(jpeg)
                    .map_err(|_| "dépôt des images interrompu".to_owned())
            });
            (texte, rendu)
        });
        let mut cles = Vec::new();
        let mut tailles = Vec::new();
        let mut depot = Ok(());
        while let Some(jpeg) = rendues.recv().await {
            let cle = format!(
                "documents/{}/{}/{}.jpg",
                c.document_id,
                c.asset_id,
                cles.len() + 1
            );
            let taille = jpeg.len() as i32;
            if let Err(erreur) = stockage.put(&cle, "image/jpeg", jpeg).await {
                depot = Err(erreur);
                break;
            }
            cles.push(cle);
            tailles.push(taille);
        }
        drop(rendues);
        let lu = lecture
            .await
            .map_err(|e| ApiError::internal(format!("extraction interrompue : {e}")));
        if depot.is_err() || lu.is_err() {
            effacer(stockage.as_ref(), &cles).await;
        }
        depot?;
        let (texte, rendu) = lu?;
        if let Err(motif) = rendu {
            effacer(stockage.as_ref(), &cles).await;
            return self
                .conclure_en_echec(job, c, &format!("Le fichier est illisible : {motif}"))
                .await;
        }
        let extraction = match texte {
            Ok(brut) => extraction::extraire(&brut),
            // Le texte ne se tire pas, les pages se rendent : le document se
            // lit sur ses pages, sans « Texte agrandi », et l'aperçu le dit.
            Err(motif) => {
                tracing::warn!(document = %c.document_id, %motif, "texte illisible, pages rendues");
                sans_texte(cles.len())
            }
        };

        match self
            .conclure(job, c, &extraction, &tailles, &cles, objet.byte_size)
            .await
        {
            Ok(true) => {}
            // Le fichier ou la demande a changé pendant l'extraction : elle ne
            // vaut plus rien.
            Ok(false) => {
                effacer(stockage.as_ref(), &cles).await;
                return Ok(());
            }
            Err(erreur) => {
                effacer(stockage.as_ref(), &cles).await;
                return Err(erreur);
            }
        }

        let obsoletes: Vec<String> = anciennes
            .into_iter()
            .filter(|a| !cles.contains(a))
            .collect();
        effacer(stockage.as_ref(), &obsoletes).await;
        tracing::info!(document = %c.document_id, pages = cles.len(), "document extrait");
        Ok(())
    }

    /// Les pages et le verdict, dans une transaction. Faux : ce travail n'est
    /// plus celui de la demande en cours, et rien n'est écrit.
    async fn conclure(
        &self,
        job: &ClaimedJob,
        c: &Charge,
        extraction: &Extraction,
        tailles: &[i32],
        cles: &[String],
        octets_du_pdf: i64,
    ) -> Result<bool> {
        let pages = pages_a_ecrire(extraction, tailles, cles)?;
        let outline = serde_json::to_value(&extraction.outline)
            .map_err(|e| ApiError::internal(format!("sommaire : {e}")))?;
        let quality = serde_json::to_value(&extraction.quality)
            .map_err(|e| ApiError::internal(format!("indicateurs : {e}")))?;

        let mut tx = self.db.write(&job.context().with_actor(c.acteur)).await?;
        document_pages::remplacer(&mut tx, c.document_id, &pages).await?;
        let pose = renditions::reussir(
            &mut tx,
            c.document_id,
            c.asset_id,
            c.demande,
            &renditions::Verdict {
                page_count: pages.len() as i32,
                outline: &outline,
                is_reflowable: extraction.is_reflowable,
                quality: &quality,
                extractor: &format!(
                    "pdfium-render 0.9.4 / pdfium chromium-7881 / règles {}",
                    env!("CARGO_PKG_VERSION")
                ),
            },
        )
        .await?;
        if !pose {
            tx.rollback().await?;
            return Ok(false);
        }
        // La lecture pesée est celle que l'API servira, relue dans cette
        // transaction : la règle « Texte agrandi » y vient de la base.
        let doc = documents::quelconque(&mut tx, c.document_id)
            .await?
            .ok_or_else(|| ApiError::internal("document disparu pendant l'extraction"))?;
        let rendu = doc
            .rendu
            .as_ref()
            .ok_or_else(|| ApiError::internal("extraction disparue pendant sa conclusion"))?;
        let lecture = service::forme_lisible(
            &mut tx,
            c.document_id,
            &doc.version,
            doc.outline.clone(),
            rendu.has_text,
            rendu.large_text,
        )
        .await?;
        let poids = octets_du_pdf + serialiser_la_lecture(&lecture).len() as i64;
        renditions::poser_le_poids(&mut tx, c.document_id, poids).await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Les images qu'un essai tombé a déposées avant sa chute : elles se suivent
    /// depuis la première, et aucune page ne les désigne. Celles d'une
    /// extraction antérieure du même fichier, encore en base, restent. Au
    /// mieux : l'échec se conclut quoi qu'il arrive ici.
    async fn effacer_lessai_tombe(&self, c: &Charge) {
        let Ok(mut conn) = self.db.pool().acquire().await else {
            return;
        };
        let Ok(gardees) = document_pages::cles_d_images(&mut conn, c.document_id).await else {
            return;
        };
        drop(conn);
        let Ok(bucket) = self.bucket_prive().await else {
            return;
        };
        let stockage = self.entrepots.du_bucket(&bucket);
        for n in 1..=crate::pdf::PAGES_MAX {
            let cle = format!("documents/{}/{}/{n}.jpg", c.document_id, c.asset_id);
            match stockage.head(&cle).await {
                Ok(_) if gardees.contains(&cle) => {}
                Ok(_) => effacer(stockage.as_ref(), std::slice::from_ref(&cle)).await,
                Err(kernel::storage::StorageError::NotFound(_)) => break,
                Err(erreur) => {
                    tracing::warn!(%cle, %erreur, "images d'un essai tombé non effacées");
                    break;
                }
            }
        }
    }
}

/// Un document dont le texte ne se tire pas : des pages sans blocs, à lire en
/// images.
fn sans_texte(pages: usize) -> Extraction {
    Extraction {
        pages: (1..=pages)
            .map(|i| extraction::PageExtraite {
                index: i,
                label: i.to_string(),
                blocks: vec![],
                plain_text: String::new(),
                has_origin_block: false,
            })
            .collect(),
        outline: vec![],
        is_reflowable: false,
        quality: extraction::Qualite {
            pages,
            ..Default::default()
        },
    }
}

/// Au mieux : une image qui reste n'est qu'un objet de trop dans le bucket privé.
async fn effacer(stockage: &dyn ObjectStore, cles: &[String]) {
    for cle in cles {
        if let Err(erreur) = stockage.delete(cle).await {
            tracing::warn!(%cle, %erreur, "image de page non effacée");
        }
    }
}

fn pages_a_ecrire(
    extraction: &Extraction,
    tailles: &[i32],
    cles: &[String],
) -> Result<Vec<document_pages::PageAEcrire>> {
    extraction
        .pages
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let blocks: &[Block] = &p.blocks;
            Ok(document_pages::PageAEcrire {
                index: p.index as i32,
                label: p.label.clone(),
                blocks: serde_json::to_value(blocks)
                    .map_err(|e| ApiError::internal(format!("page {} : {e}", p.index)))?,
                plain_text: p.plain_text.clone(),
                image_key: cles.get(i).cloned(),
                image_bytes: tailles.get(i).copied(),
                has_origin_block: p.has_origin_block,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattente_double_et_plafonne_a_dix_minutes() {
        assert_eq!(delai(0), Duration::seconds(15));
        assert_eq!(delai(1), Duration::seconds(30));
        assert_eq!(delai(5), Duration::seconds(480));
        assert_eq!(delai(6), Duration::seconds(600));
        assert_eq!(delai(11), Duration::seconds(600));
    }
}
