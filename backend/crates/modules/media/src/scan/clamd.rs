//! ClamAV par son protocole `INSTREAM`, sur TCP — **aucune dépendance
//! nouvelle**.
//!
//! Le protocole tient en quelques lignes : `zINSTREAM\0`, puis des tranches
//! précédées de leur longueur sur quatre octets en gros-boutien, puis une
//! longueur nulle pour clore. La réponse est une ligne de texte terminée par un
//! octet nul — `stream: OK` ou `stream: <signature> FOUND`.
//!
//! # Le plafond n'est pas une optimisation
//!
//! Au-delà de `MEDIA_SCAN_MAX_BYTES`, le verdict est **« non pris en charge »**
//! plutôt qu'une analyse de cinq minutes qui bloque un fil du worker et fait
//! expirer le bail du travail — lequel serait alors repris et analysé une
//! seconde fois, en parallèle du premier.

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::{Scanner, Verdict};

/// Tranches de 32 Kio : le protocole en accepte de plus grosses, `clamd` refuse
/// au-delà de sa propre limite, et 32 Kio passe partout.
const TRANCHE: usize = 32 * 1024;

pub struct Clamd {
    adresse: String,
    plafond: u64,
}

impl Clamd {
    pub fn new(adresse: String, plafond: u64) -> Self {
        Self { adresse, plafond }
    }

    async fn interroger(&self, contenu: &[u8]) -> std::io::Result<String> {
        let mut flux = TcpStream::connect(&self.adresse).await?;
        flux.write_all(b"zINSTREAM\0").await?;

        for tranche in contenu.chunks(TRANCHE) {
            flux.write_all(&(tranche.len() as u32).to_be_bytes())
                .await?;
            flux.write_all(tranche).await?;
        }
        flux.write_all(&0u32.to_be_bytes()).await?;
        flux.flush().await?;

        let mut reponse = Vec::new();
        flux.read_to_end(&mut reponse).await?;
        Ok(String::from_utf8_lossy(&reponse)
            .trim_end_matches('\0')
            .trim()
            .to_owned())
    }
}

#[async_trait]
impl Scanner for Clamd {
    async fn analyser(&self, contenu: &[u8]) -> Verdict {
        if contenu.len() as u64 > self.plafond {
            return Verdict::non_pris_en_charge(
                self.engine(),
                format!(
                    "fichier de {} octets au-delà du plafond d'analyse de {}",
                    contenu.len(),
                    self.plafond
                ),
            );
        }

        match self.interroger(contenu).await {
            Ok(reponse) => lire_verdict(&reponse, self.engine()),
            // Une panne du moteur ne fait pas mourir le traitement : elle laisse
            // l'objet hors service avec son motif, ce qu'aucune erreur remontée
            // ne porterait.
            Err(e) => Verdict::en_erreur(self.engine(), e.to_string()),
        }
    }

    fn engine(&self) -> &'static str {
        "clamd"
    }
}

/// `stream: OK` · `stream: Eicar-Test-Signature FOUND` · `… ERROR`.
fn lire_verdict(reponse: &str, moteur: &str) -> Verdict {
    if reponse.ends_with("OK") {
        Verdict::propre(moteur)
    } else if let Some(signature) = reponse
        .strip_suffix(" FOUND")
        .and_then(|r| r.split_once(": "))
        .map(|(_, s)| s)
    {
        Verdict::infecte(moteur, signature)
    } else {
        Verdict::en_erreur(moteur, reponse.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_reponse_du_protocole_est_lue() {
        assert_eq!(lire_verdict("stream: OK", "clamd").verdict, "clean");

        let infecte = lire_verdict("stream: Eicar-Test-Signature FOUND", "clamd");
        assert_eq!(infecte.verdict, "infected");
        assert_eq!(infecte.details.as_deref(), Some("Eicar-Test-Signature"));

        assert_eq!(
            lire_verdict("stream: size limit exceeded ERROR", "clamd").verdict,
            "error"
        );
    }

    #[tokio::test]
    async fn au_dela_du_plafond_rien_nest_analyse() {
        // Adresse volontairement injoignable : le plafond doit trancher avant
        // toute tentative de connexion.
        let moteur = Clamd::new("127.0.0.1:1".to_owned(), 4);
        let verdict = moteur.analyser(b"douze octets").await;
        assert_eq!(verdict.verdict, "unsupported");
    }
}
