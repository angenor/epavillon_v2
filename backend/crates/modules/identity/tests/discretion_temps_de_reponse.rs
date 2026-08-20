//! SC-001 : une adresse inconnue et un mot de passe faux doivent rendre la même
//! réponse **et le même temps**.
//!
//! Le message identique ne suffit pas. Argon2id coûte quelques dizaines de
//! millisecondes ; abandonner avant de le calculer, parce que l'adresse est
//! inconnue, rend la réponse dix à cent fois plus rapide — et le formulaire de
//! connexion redevient l'annuaire des comptes qu'on voulait fermer. C'est
//! l'empreinte factice du noyau qui paie ce coût-là (research.md § R5).
//!
//! La mesure porte sur la **médiane** et non sur la moyenne : une pause du
//! ramasse-miettes ou un verrou de la base suffirait sinon à faire échouer un
//! test qui n'a rien à dire sur ces cas-là.

mod commun;

use commun::{semer, Bac, Compte};
use identity::domain::login::LoginOutcome;
use identity::service::auth::{login, LoginRequest};
use identity::service::session::Device;
use std::time::{Duration, Instant};

const TENTATIVES: usize = 100;
const ECART_TOLERE: f64 = 0.10;

async fn mesurer(bac: &Bac, email: &str) -> Duration {
    let debut = Instant::now();
    let reponse = login(
        &bac.state,
        &bac.ctx(),
        LoginRequest {
            email,
            password: "nimportequoi",
            remember_me: false,
            device: Device::default(),
        },
    )
    .await
    .expect("connexion");
    let duree = debut.elapsed();

    assert!(
        matches!(reponse.outcome, LoginOutcome::InvalidCredentials),
        "les deux cas doivent être indiscernables : {:?}",
        reponse.outcome
    );
    duree
}

fn mediane(mut mesures: Vec<Duration>) -> f64 {
    mesures.sort_unstable();
    mesures[mesures.len() / 2].as_secs_f64()
}

#[tokio::test]
async fn adresse_inconnue_et_mot_de_passe_faux_prennent_le_meme_temps() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif("awa.diallo@example.org")).await;

    // Deux tours à vide : la première connexion paie la mise en cache des
    // requêtes préparées, et fausserait la première mesure de chaque série.
    for _ in 0..2 {
        mesurer(&bac, "personne.inconnue@example.org").await;
        mesurer(&bac, "awa.diallo@example.org").await;
    }

    let mut inconnues = Vec::with_capacity(TENTATIVES);
    let mut connues = Vec::with_capacity(TENTATIVES);

    // Les deux séries sont entrelacées : mesurées l'une après l'autre, une
    // dérive de charge de la machine se lirait comme un écart de traitement.
    for _ in 0..TENTATIVES {
        inconnues.push(mesurer(&bac, "personne.inconnue@example.org").await);
        connues.push(mesurer(&bac, "awa.diallo@example.org").await);
    }

    let inconnue = mediane(inconnues);
    let connue = mediane(connues);
    let ecart = (inconnue - connue).abs() / inconnue.max(connue);

    assert!(
        ecart < ECART_TOLERE,
        "écart de {:.1} % entre une adresse inconnue ({:.1} ms) et un mot de passe faux ({:.1} ms) : \
         l'empreinte factice ne paie pas le même coût",
        ecart * 100.0,
        inconnue * 1000.0,
        connue * 1000.0
    );
}
