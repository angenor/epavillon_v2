//! Le contrat d'analyse antivirus — **et l'absence de moteur y est déclarée**.
//!
//! # `none` rend « non pris en charge », jamais « sain »
//!
//! `ck_assets_scan_before_ready` accepte les deux : un objet peut devenir
//! servable avec `clean` comme avec `unsupported`. Mais **`clean` affirmerait
//! qu'un moteur a inspecté le fichier et n'a rien trouvé**. `unsupported` —
//! « aucun moteur ne sait analyser ceci » — est littéralement vrai quand aucun
//! moteur n'est branché, et la colonne `scan_engine` porte alors `none`.
//!
//! Une plateforme institutionnelle doit pouvoir **prouver** ce qui a été
//! inspecté ; écrire « sain » sans avoir regardé rendrait cette preuve fausse
//! (B6, R13).
//!
//! # Pourquoi l'absence de moteur ne bloque pas la mise en service
//!
//! L'environnement de développement n'en a pas. Bloquer signifierait que rien
//! n'est jamais servable en local — et l'on finirait par contourner la garde,
//! ce qui est pire que de la déclarer.

use async_trait::async_trait;
use std::sync::Arc;

pub mod clamd;
pub mod none;

/// Le verdict, tel que `media.scan_verdict` le nomme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// `clean`, `infected`, `unsupported` ou `error`.
    pub verdict: &'static str,
    /// Le nom du moteur, écrit dans `scan_engine`. **Toujours renseigné** — y
    /// compris `none` : la trace doit dire qui a regardé, ou que personne ne
    /// l'a fait.
    pub engine: String,
    /// Détail rendu par le moteur : le nom de la signature trouvée, le motif du
    /// refus. Part dans `scan_details`.
    pub details: Option<String>,
}

impl Verdict {
    pub fn propre(engine: impl Into<String>) -> Self {
        Self {
            verdict: "clean",
            engine: engine.into(),
            details: None,
        }
    }

    pub fn infecte(engine: impl Into<String>, signature: impl Into<String>) -> Self {
        Self {
            verdict: "infected",
            engine: engine.into(),
            details: Some(signature.into()),
        }
    }

    pub fn non_pris_en_charge(engine: impl Into<String>, motif: impl Into<String>) -> Self {
        Self {
            verdict: "unsupported",
            engine: engine.into(),
            details: Some(motif.into()),
        }
    }

    pub fn en_erreur(engine: impl Into<String>, erreur: impl Into<String>) -> Self {
        Self {
            verdict: "error",
            engine: engine.into(),
            details: Some(erreur.into()),
        }
    }

    /// Un objet ne devient servable qu'avec l'un des deux verdicts que
    /// `ck_assets_scan_before_ready` accepte. Ce prédicat **traduit** la
    /// contrainte, il ne la remplace pas : c'est la base qui refuse.
    pub fn autorise_la_mise_en_service(&self) -> bool {
        matches!(self.verdict, "clean" | "unsupported")
    }
}

#[async_trait]
pub trait Scanner: Send + Sync {
    /// N'échoue jamais : une panne du moteur devient un verdict `error`, qui
    /// laisse l'objet hors service **avec son motif**. Une erreur remontée
    /// ferait mourir le travail de traitement sur une cause qu'aucune colonne ne
    /// porterait.
    async fn analyser(&self, contenu: &[u8]) -> Verdict;

    fn engine(&self) -> &'static str;
}

pub fn build(cfg: &kernel::config::MediaConfig) -> Arc<dyn Scanner> {
    match cfg.scanner {
        kernel::config::MediaScanner::None => Arc::new(none::AucunMoteur),
        kernel::config::MediaScanner::Clamd => Arc::new(clamd::Clamd::new(
            cfg.clamd_addr.clone(),
            cfg.scan_max_bytes,
        )),
    }
}
