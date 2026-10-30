//! Temps réel : `/realtime/stream` (ligne ouverte, personnes connectées) et
//! `/realtime/pulse` (vérification périodique, public et repli).
//!
//! `platform.signal()` → `LISTEN platform_signal` → concentrateur (`hub`) →
//! flux filtrés par sujet autorisé, regroupés sur 500 ms. Le signal dit « a
//! changé », la page relit sa route REST. Étude et décisions :
//! `docs/progression/decisions/2026-10-31-temps-reel.md` et `-besoins.md`.
//!
//! Brancher un écran :
//! 1. SQL ou Rust, dans la transaction du changement :
//!    `kernel::realtime::signal(&mut tx, "proposal:<id>", None)` ;
//! 2. dans le module, un `TopicAuthorizer` pour ses sujets, enregistré dans
//!    `autorisations()` ci-dessous ;
//! 3. dans la page, `useRealtime(['proposal:<id>'], relire)`.

pub mod hub;
pub mod pulse;
pub mod stream;

use actix_web::web;
use kernel::error::{ApiError, Result};
use kernel::realtime::{
    Audience, PersonTopics, PublicEditionTopics, Topic, TopicAccess, TopicRegistry, Viewer,
};
use sqlx::PgPool;
use std::collections::BTreeMap;

pub use hub::Hub;

/// Plafond de sujets par requête : une page en suit trois ou quatre.
pub const SUJETS_MAX: usize = 32;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/realtime/stream", web::get().to(stream::flux))
        .route("/realtime/pulse", web::get().to(pulse::pouls));
}

/// Les autorisations de sujets, assemblées ici et nulle part ailleurs.
///
/// Le socle répond de `person:<soi>` et de `edition:<id>:public…`. Chaque
/// module ajoute les siens quand son écran est branché, par exemple
/// `.register_all(programme::topic_authorizers())` pour `proposal:` et
/// `organization:`, `engagement` pour `mailing:` et `event:<id>:mailings`,
/// `programme` pour `event:<id>:planner`, `programme` pour `session:<id>`.
pub fn autorisations() -> TopicRegistry {
    TopicRegistry::new()
        .register(PersonTopics)
        .register(PublicEditionTopics)
}

/// `?topics=a,b,c` : sujets valides, dédoublonnés, triés. Un sujet mal formé
/// est un 422 ; il n'y a rien à autoriser.
pub fn lire_sujets(brut: &str) -> Result<Vec<Topic>> {
    let mut sujets = Vec::new();
    for morceau in brut.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let sujet = Topic::parse(morceau).ok_or_else(|| {
            ApiError::validation(
                format!("Sujet de mise à jour invalide : « {morceau} »."),
                "topics",
            )
        })?;
        sujets.push(sujet);
    }
    sujets.sort();
    sujets.dedup();
    if sujets.is_empty() {
        return Err(ApiError::validation("Aucun sujet demandé.", "topics"));
    }
    if sujets.len() > SUJETS_MAX {
        return Err(ApiError::validation(
            format!("Au plus {SUJETS_MAX} sujets à la fois."),
            "topics",
        ));
    }
    Ok(sujets)
}

/// Sépare les sujets accordés (avec leur audience) des refusés.
pub async fn trier(
    registre: &TopicRegistry,
    pool: &PgPool,
    viewer: &Viewer,
    sujets: Vec<Topic>,
    cache: Option<&Hub>,
) -> Result<(BTreeMap<Topic, Audience>, Vec<String>)> {
    let mut accordes = BTreeMap::new();
    let mut refuses = Vec::new();
    for sujet in sujets {
        let en_cache = cache.and_then(|h| h.autorisation_en_cache(viewer.session_id, &sujet));
        let acces = match en_cache {
            Some(acces) => acces,
            None => {
                let acces = registre.authorize(pool, viewer, &sujet).await?;
                if let Some(h) = cache {
                    h.retenir_autorisation(viewer.session_id, &sujet, acces.clone());
                }
                acces
            }
        };
        match acces {
            TopicAccess::Granted(audience) => {
                accordes.insert(sujet, audience);
            }
            TopicAccess::Denied => refuses.push(sujet.as_str().to_owned()),
        }
    }
    Ok((accordes, refuses))
}
