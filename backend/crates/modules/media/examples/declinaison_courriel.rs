//! Fabrique la déclinaison `courriel` d'une image par le code du worker, sans
//! base ni stockage : pour regarder son poids, et pour les courriels d'essai.
//!
//! `cargo run -p media --example declinaison_courriel -- <entrée> <sortie.jpeg>`

use media::domain::asset::RenditionFormat;
use media::domain::{imaging, variants};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (entree, sortie) = (&args[1], &args[2]);
    let octets = std::fs::read(entree).expect("entrée lisible");
    let v = variants::VARIANTES
        .iter()
        .find(|v| v.code == "courriel")
        .expect("variante courriel");
    let faite = imaging::decliner(
        &octets,
        v.largeur_max,
        v.rapport,
        v.qualite,
        RenditionFormat::Jpeg,
    )
    .expect("déclinaison");
    std::fs::write(sortie, &faite.octets).expect("sortie écrite");
    println!(
        "{} x {}, {} octets",
        faite.width,
        faite.height,
        faite.octets.len()
    );
}
