//! Un compte repris de la v1, sans empreinte, doit répondre comme un mot de
//! passe faux : même issue, **même temps** (FR-019, FR-020). Il paie le hachage
//! factice du noyau, comme une adresse inconnue. Une empreinte factice en base
//! aurait répondu plus vite qu'Argon2id et trahi le compte à la montre.
//!
//! Binaire à part : les tests d'un même binaire tournent en parallèle, et une
//! mesure de temps n'a rien à faire à côté d'autres hachages.

mod commun;

use commun::{semer, Bac, Compte};
use identity::domain::login::LoginOutcome;
use identity::service::auth::{login, LoginRequest};
use identity::service::session::Device;
use std::time::{Duration, Instant};

const TENTATIVES: usize = 100;
const ECART_TOLERE: f64 = 0.10;
const CONNU: &str = "awa.diallo@example.org";
const REPRIS: &str = "fatou.sow@example.org";

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
async fn compte_repris_et_mot_de_passe_faux_prennent_le_meme_temps() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(CONNU)).await;
    semer(&bac, Compte::repris(REPRIS)).await;

    for _ in 0..2 {
        mesurer(&bac, REPRIS).await;
        mesurer(&bac, CONNU).await;
    }

    let mut repris = Vec::with_capacity(TENTATIVES);
    let mut connus = Vec::with_capacity(TENTATIVES);
    for _ in 0..TENTATIVES {
        repris.push(mesurer(&bac, REPRIS).await);
        connus.push(mesurer(&bac, CONNU).await);
    }

    let repris = mediane(repris);
    let connu = mediane(connus);
    let ecart = (repris - connu).abs() / repris.max(connu);

    assert!(
        ecart < ECART_TOLERE,
        "écart de {:.1} % entre un compte repris ({:.1} ms) et un mot de passe faux ({:.1} ms)",
        ecart * 100.0,
        repris * 1000.0,
        connu * 1000.0
    );
}
