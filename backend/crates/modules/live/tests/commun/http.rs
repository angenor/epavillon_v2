//! Un petit serveur HTTP local qui imite l'API Zoom, sans crate de doublure.

use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub type Journal = Arc<Mutex<Vec<String>>>;
pub type Repondeur = fn(&str, &str) -> (u16, String);

/// Sert `repondre(méthode, chemin avec requête, corps)` et note chaque requête.
pub async fn serveur(repondre: Repondeur) -> (String, Journal) {
    let ecoute = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = format!("http://{}", ecoute.local_addr().unwrap());
    let journal: Journal = Arc::default();
    let note = journal.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut flux, _)) = ecoute.accept().await else {
                return;
            };
            let note = note.clone();
            tokio::spawn(async move {
                let mut lu = Vec::new();
                let mut tampon = [0u8; 4096];
                let (entete, mut corps) = loop {
                    let n = flux.read(&mut tampon).await.unwrap();
                    lu.extend_from_slice(&tampon[..n]);
                    if let Some(i) = lu.windows(4).position(|w| w == b"\r\n\r\n") {
                        break (
                            String::from_utf8_lossy(&lu[..i]).into_owned(),
                            lu[i + 4..].to_vec(),
                        );
                    }
                };
                let longueur = entete
                    .lines()
                    .find_map(|l| {
                        let (k, v) = l.split_once(':')?;
                        k.eq_ignore_ascii_case("content-length")
                            .then(|| v.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                while corps.len() < longueur {
                    let n = flux.read(&mut tampon).await.unwrap();
                    corps.extend_from_slice(&tampon[..n]);
                }
                let ligne = entete.lines().next().unwrap_or_default().to_owned();
                let mut morceaux = ligne.split_whitespace();
                let methode = morceaux.next().unwrap_or_default();
                let chemin = morceaux.next().unwrap_or_default();
                note.lock().unwrap().push(format!(
                    "{methode} {chemin} {}",
                    String::from_utf8_lossy(&corps)
                ));
                let (statut, reponse) = repondre(methode, chemin);
                let brut = format!(
                    "HTTP/1.1 {statut} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reponse}",
                    reponse.len()
                );
                flux.write_all(brut.as_bytes()).await.unwrap();
                flux.shutdown().await.ok();
            });
        }
    });
    (adresse, journal)
}
