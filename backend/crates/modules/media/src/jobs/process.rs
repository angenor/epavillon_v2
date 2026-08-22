//! **Le traitement d'un objet déposé** — analyse, relevé, déclinaisons, mise en
//! service.
//!
//! # Ce travail est CONSOMMÉ, jamais créé
//!
//! `media.tg_enqueue_processing()` le met en file à l'insertion de l'objet, avec
//! l'identifiant de celui-ci pour clé d'unicité, **et** émet `media.asset.uploaded`
//! dans le même geste. Ce fichier n'enfile rien et n'émet rien : il exécute.
//!
//! # La file est « au moins une fois », et ce travail en tient compte
//!
//! Un worker tué entre le travail et son marquage rejoue le travail entier —
//! c'est écrit dans `worker/src/jobs.rs`, et le point de contrôle du quickstart
//! le provoque exprès. Trois gardes se superposent, chacune à un endroit
//! différent :
//!
//! 1. un objet déjà servable ou en quarantaine sort tout de suite ;
//! 2. les déclinaisons déjà écrites ne sont ni refabriquées ni redéposées ;
//! 3. `ON CONFLICT DO NOTHING` empêche la seconde ligne si deux workers passent
//!    ensemble.
//!
//! Le test qui compte les mesure : `count(*)` sur `media.renditions` rend le
//! nombre de déclinaisons configurées, jamais le double.
//!
//! # Le redimensionnement vit sur une TÂCHE BLOQUANTE
//!
//! Décoder puis rééchantillonner une photographie de conférence coûte quelques
//! centaines de millisecondes de calcul pur. Laissé sur le fil asynchrone, il
//! bloquerait tout ce que ce fil sert — c'est exactement le raisonnement qui a
//! mis le hachage de mot de passe sur `spawn_blocking` en B1.
//!
//! # L'échec définitif écrit son motif, et se distingue d'une absence
//!
//! Un échec passager rend l'erreur : `platform.fail_job()` replanifie avec son
//! délai croissant. Au **dernier essai** — `attempts >= max_attempts` —, l'objet
//! passe en échec et la déclinaison fautive écrit son motif, avant que l'erreur
//! ne soit rendue une dernière fois. Sans cela, « en cours » et « en échec » se
//! liraient tous les deux « pas encore là » (FR-032).

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{ClaimedJob, JobHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::asset::RenditionFormat;
use crate::domain::{duration, imaging, variants};
use crate::repo::assets::{self, AObjetTraiter};
use crate::repo::renditions::{self, DeclinaisonPrete};
use crate::scan::Scanner;
use crate::storage::ObjectStore;

/// Le nom de la tâche, tel que le déclencheur du modèle l'écrit. Toute
/// divergence rendrait le travail sans gestionnaire — et le worker le dirait,
/// mais après coup.
pub const PROCESS_ASSET: &str = "media.process_asset";

/// La file, telle que le déclencheur la nomme. Le worker n'écoute que les files
/// que ses gestionnaires déclarent : une file inécoutée laisserait les travaux
/// s'empiler sans erreur ni trace.
pub const QUEUE: &str = "media";

pub struct ProcessAsset {
    db: Db,
    storage: Arc<dyn ObjectStore>,
    scanner: Arc<dyn Scanner>,
}

impl ProcessAsset {
    pub fn new(db: Db, storage: Arc<dyn ObjectStore>, scanner: Arc<dyn Scanner>) -> Self {
        Self {
            db,
            storage,
            scanner,
        }
    }
}

#[async_trait]
impl JobHandler for ProcessAsset {
    fn task(&self) -> &'static str {
        PROCESS_ASSET
    }

    fn queue(&self) -> &'static str {
        QUEUE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let asset_id = identifiant(job)?;

        let Some(objet) = assets::pour_traitement(self.db.pool(), asset_id).await? else {
            // Supprimé entre la mise en file et le passage du worker. Le travail
            // n'a plus d'objet ; ce n'est pas un échec.
            tracing::info!(%asset_id, "objet absent : traitement sans effet");
            return Ok(());
        };

        // Garde n° 1 de la reprise : les deux états terminaux ne se retraitent
        // pas. Sans elle, un rejeu refabriquerait des déclinaisons déjà servies.
        if matches!(objet.status.as_str(), "ready" | "quarantined") {
            tracing::debug!(%asset_id, etat = %objet.status, "traitement déjà abouti");
            return Ok(());
        }

        match self.traiter(job, &objet).await {
            Ok(()) => Ok(()),
            Err(erreur) => {
                if job.attempts >= job.max_attempts {
                    let mut tx = self.db.write(&job.context()).await?;
                    assets::marquer_echec(&mut tx, asset_id).await?;
                    tx.commit().await?;
                    tracing::error!(%asset_id, erreur = %erreur, "traitement en échec définitif");
                }
                Err(erreur)
            }
        }
    }
}

impl ProcessAsset {
    async fn traiter(&self, job: &ClaimedJob, objet: &AObjetTraiter) -> Result<()> {
        let ctx = job.context();

        let mut tx = self.db.write(&ctx).await?;
        assets::poser_etat(&mut tx, objet.id, "scanning").await?;
        tx.commit().await?;

        // **Une seule copie des octets, partagée** : le relevé et les trois
        // redimensionnements tournent chacun sur une tâche bloquante, et leur
        // donner chacun sa copie tiendrait quatre fois l'image en mémoire.
        let octets = Arc::new(self.storage.get(&objet.object_key).await?);

        // ------------------------------------------------------------------
        // L'analyse. `none` rend « non pris en charge » et son nom, jamais
        // « sain » : écrire « sain » sans avoir regardé rendrait fausse la
        // preuve d'inspection que doit pouvoir fournir une plateforme
        // institutionnelle (R13).
        // ------------------------------------------------------------------
        let verdict = self.scanner.analyser(&octets).await;

        let mut tx = self.db.write(&ctx).await?;
        assets::enregistrer_analyse(
            &mut tx,
            objet.id,
            verdict.verdict,
            &verdict.engine,
            verdict.details.as_deref(),
        )
        .await?;

        if verdict.verdict == "infected" {
            assets::mettre_en_quarantaine(&mut tx, objet.id).await?;
            tx.commit().await?;
            tracing::warn!(asset = %objet.id, moteur = %verdict.engine, "objet mis en quarantaine");
            return Ok(());
        }
        tx.commit().await?;

        if !verdict.autorise_la_mise_en_service() {
            // Une panne du moteur n'est pas un verdict : le travail est repris,
            // et le motif est déjà écrit sur l'objet.
            return Err(ApiError::internal(format!(
                "analyse indisponible ({}) : {}",
                verdict.engine,
                verdict.details.as_deref().unwrap_or("sans détail")
            )));
        }

        let mut tx = self.db.write(&ctx).await?;
        assets::poser_etat(&mut tx, objet.id, "processing").await?;
        tx.commit().await?;

        // ------------------------------------------------------------------
        // Le relevé. Une image se mesure, un média temporel se date, un
        // document n'a ni l'un ni l'autre — et cela ne l'empêche pas de devenir
        // servable (FR-030).
        // ------------------------------------------------------------------
        let mesure = if objet.mime_type.starts_with("image/") {
            let contenu = Arc::clone(&octets);
            tache_bloquante(move || imaging::mesurer(&contenu)).await?
        } else {
            None
        };
        let duree = if duration::est_temporel(&objet.mime_type) {
            duration::duree_secondes(&octets).map(|d| format!("{d:.3}"))
        } else {
            None
        };

        let mut tx = self.db.write(&ctx).await?;
        assets::enregistrer_mesure(
            &mut tx,
            objet.id,
            mesure.map(|m| (m.width as i32, m.height as i32)),
            duree,
        )
        .await?;
        tx.commit().await?;

        if let Some(mesure) = mesure {
            self.fabriquer_les_declinaisons(job, objet, &octets, mesure)
                .await?;
        }

        let mut tx = self.db.write(&ctx).await?;
        assets::marquer_servable(&mut tx, objet.id).await?;
        tx.commit().await?;

        tracing::info!(asset = %objet.id, moteur = %verdict.engine, "objet servable");
        Ok(())
    }

    /// Fabrique ce qui manque, et **seulement** ce qui manque.
    async fn fabriquer_les_declinaisons(
        &self,
        job: &ClaimedJob,
        objet: &AObjetTraiter,
        octets: &Arc<Vec<u8>>,
        mesure: imaging::Dimensions,
    ) -> Result<()> {
        let format = variants::format_pour(mesure.porte_transparence);
        let attendues = variants::attendues(mesure.width);
        if attendues.is_empty() {
            return Ok(());
        }

        let ctx = job.context();
        let mut tx = self.db.write(&ctx).await?;
        let faites = renditions::deja_faites(&mut tx, objet.id).await?;
        tx.commit().await?;

        for variante in attendues {
            // Garde n° 2 de la reprise : ni refabrication, ni second dépôt sur
            // le stockage.
            if faites
                .iter()
                .any(|(code, f)| code == variante.code && f == format.as_str())
            {
                continue;
            }

            let cle = variants::cle_declinaison(objet.id, variante.code, format);
            let contenu = Arc::clone(octets);
            let largeur = variante.largeur_max;

            // **La tâche bloquante dédiée** : quelques centaines de
            // millisecondes de calcul pur qui ne doivent pas tenir le fil
            // asynchrone.
            let faite =
                match tache_bloquante(move || imaging::redimensionner(&contenu, largeur, format))
                    .await?
                {
                    Ok(faite) => faite,
                    Err(motif) => {
                        return self
                            .echec_de_declinaison(job, objet.id, variante.code, format, &cle, motif)
                            .await
                    }
                };

            let poids = faite.octets.len() as i64;
            self.storage
                .put(&cle, mime_de(format), faite.octets)
                .await?;

            let mut tx = self.db.write(&ctx).await?;
            let ecrite = renditions::ecrire_prete(
                &mut tx,
                &DeclinaisonPrete {
                    asset_id: objet.id,
                    variant_code: variante.code.to_owned(),
                    format: format.as_str().to_owned(),
                    width: faite.width as i32,
                    height: faite.height as i32,
                    object_key: cle.clone(),
                    byte_size: poids,
                },
            )
            .await?;
            tx.commit().await?;

            if !ecrite {
                tracing::debug!(asset = %objet.id, variante = variante.code, "déclinaison déjà écrite");
            }
        }

        Ok(())
    }

    /// Une déclinaison impossible à fabriquer. Le motif n'est écrit qu'au
    /// **dernier essai** : l'écrire plus tôt figerait la ligne — `ux_renditions`
    /// occupe la place — et empêcherait la reprise de la refaire.
    async fn echec_de_declinaison(
        &self,
        job: &ClaimedJob,
        asset_id: Uuid,
        code: &str,
        format: RenditionFormat,
        cle: &str,
        motif: String,
    ) -> Result<()> {
        if job.attempts >= job.max_attempts {
            let mut tx = self.db.write(&job.context()).await?;
            renditions::ecrire_echec(&mut tx, asset_id, code, format.as_str(), cle, &motif).await?;
            tx.commit().await?;
        }

        Err(ApiError::internal(format!(
            "déclinaison « {code} » impossible : {motif}"
        )))
    }
}

/// Le type MIME d'une déclinaison, tel que le stockage le rendra au navigateur.
fn mime_de(format: RenditionFormat) -> &'static str {
    match format {
        RenditionFormat::Png => "image/png",
        _ => "image/jpeg",
    }
}

/// Exécute un calcul lourd hors du fil asynchrone.
async fn tache_bloquante<T, F>(travail: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(travail)
        .await
        .map_err(|e| ApiError::internal(format!("tâche de traitement interrompue : {e}")))
}

/// L'identifiant porté par la charge utile du déclencheur.
fn identifiant(job: &ClaimedJob) -> Result<Uuid> {
    job.payload
        .get("asset_id")
        .and_then(|v| v.as_str())
        .and_then(|v| Uuid::parse_str(v).ok())
        .ok_or_else(|| ApiError::internal("charge utile sans « asset_id » exploitable".to_owned()))
}
