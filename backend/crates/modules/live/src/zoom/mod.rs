//! Zoom : l'API (réunions, inscrits), la signature du Meeting SDK, les
//! notifications entrantes et le chiffrement des secrets de réunion.

pub mod client;
pub mod coffre;
pub mod jwt;
pub mod webhook;

use kernel::config::ZoomConfig;
use kernel::error::Result;

pub use client::{Inscrit, ZoomClient, ZoomError};
pub use coffre::Coffre;

/// Ce que les routes et les travaux partagent quand Zoom est configuré.
pub struct Zoom {
    pub client: ZoomClient,
    pub coffre: Coffre,
    pub config: ZoomConfig,
}

impl Zoom {
    pub fn depuis(config: &ZoomConfig) -> Result<Self> {
        Ok(Self {
            client: ZoomClient::new(config.clone())?,
            coffre: Coffre::depuis_base64(config.secrets_key.expose())?,
            config: config.clone(),
        })
    }
}

/// Le jeton `tk` du lien personnel d'un inscrit, que le Meeting SDK attend.
pub fn jeton_du_lien(lien: &str) -> Option<String> {
    let requete = lien.split_once('?')?.1;
    let requete = requete.split('#').next().unwrap_or_default();
    requete
        .split('&')
        .filter_map(|paire| paire.split_once('='))
        .find(|(cle, _)| *cle == "tk")
        .map(|(_, valeur)| decoder_pourcent(valeur))
        .filter(|v| !v.is_empty())
}

fn decoder_pourcent(valeur: &str) -> String {
    let octets = valeur.as_bytes();
    let mut sortie = Vec::with_capacity(octets.len());
    let mut i = 0;
    while i < octets.len() {
        if octets[i] == b'%' && i + 2 < octets.len() {
            let paire = std::str::from_utf8(&octets[i + 1..i + 3]).unwrap_or_default();
            if let Ok(v) = u8::from_str_radix(paire, 16) {
                sortie.push(v);
                i += 3;
                continue;
            }
        }
        sortie.push(octets[i]);
        i += 1;
    }
    String::from_utf8_lossy(&sortie).into_owned()
}
