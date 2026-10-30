//! Le concentrateur : une écoute `LISTEN platform_signal`, une version par
//! sujet, une diffusion vers les flux ouverts.
//!
//! Tout est en mémoire : ni table, ni Valkey. Une API redémarrée repart de zéro
//! avec un autre `boot`, et le site relit tout en le voyant changer.

use kernel::realtime::{Audience, Topic, TopicAccess, SIGNAL_CHANNEL};
use serde::Deserialize;
use sqlx::postgres::{PgListener, PgPool, PgPoolOptions};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, Notify};
use uuid::Uuid;

/// Au-delà, le plus ancien flux de la session est fermé (`event: evicted`).
pub const FLUX_PAR_SESSION: usize = 5;
/// Durée d'une décision d'autorisation du pouls, comme la revalidation du flux.
pub const CACHE_AUTORISATION: Duration = Duration::from_secs(60);
const CACHE_PLAFOND: usize = 20_000;
/// Un flux trop lent qui accuse ce retard reçoit `resync` au lieu des signaux.
const FILE_DIFFUSION: usize = 1024;
const REPRISE_ECOUTE: Duration = Duration::from_secs(2);
const JOURNAL_COMPTEURS: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
pub enum HubMessage {
    Signal {
        topic: Arc<str>,
        audience: Option<Arc<str>>,
    },
    /// Signaux perdus (écoute reprise) : chaque abonné relit tout.
    Resync,
    /// Arrêt de l'API : les flux se ferment, le site revient avec une gigue.
    Restart,
}

#[derive(Default)]
struct Versions {
    sans_audience: u64,
    par_audience: HashMap<String, u64>,
}

impl Versions {
    /// La version vue par un abonné : seulement ce qu'il a le droit de voir
    /// changer. Sans cela, le pouls trahirait un échange interne au comité.
    fn pour(&self, audience: &Audience) -> u64 {
        self.sans_audience
            + self
                .par_audience
                .iter()
                .filter(|(a, _)| audience.admits(Some(a)))
                .map(|(_, n)| n)
                .sum::<u64>()
    }
}

#[derive(Default)]
pub struct Compteurs {
    pub flux_ouverts: AtomicU64,
    pub flux_total: AtomicU64,
    pub signaux_recus: AtomicU64,
    pub signaux_livres: AtomicU64,
    pub pouls_servis: AtomicU64,
    pub pouls_inchanges: AtomicU64,
    pub refus: AtomicU64,
    pub reprises_ecoute: AtomicU64,
}

impl Compteurs {
    pub fn plus(compteur: &AtomicU64, n: u64) {
        compteur.fetch_add(n, Ordering::Relaxed);
    }

    fn lire(compteur: &AtomicU64) -> u64 {
        compteur.load(Ordering::Relaxed)
    }
}

/// Flux ouverts d'une session, du plus ancien au plus récent.
type FluxDeSession = VecDeque<(u64, Arc<Notify>)>;
/// (session, sujet) → (décidé à, décision). Session nulle : anonyme.
type CacheAutorisations = HashMap<(Option<Uuid>, String), (Instant, TopicAccess)>;

pub struct Hub {
    boot: RwLock<String>,
    versions: Mutex<HashMap<String, Versions>>,
    diffusion: broadcast::Sender<HubMessage>,
    flux: Mutex<HashMap<Uuid, FluxDeSession>>,
    prochain_flux: AtomicU64,
    autorisations: Mutex<CacheAutorisations>,
    pub compteurs: Compteurs,
}

impl Default for Hub {
    fn default() -> Self {
        let (diffusion, _) = broadcast::channel(FILE_DIFFUSION);
        Self {
            boot: RwLock::new(nouveau_boot()),
            versions: Mutex::default(),
            diffusion,
            flux: Mutex::default(),
            prochain_flux: AtomicU64::new(1),
            autorisations: Mutex::default(),
            compteurs: Compteurs::default(),
        }
    }
}

fn nouveau_boot() -> String {
    Uuid::now_v7().simple().to_string()
}

#[derive(Deserialize)]
struct Notification {
    topic: String,
    payload: Option<serde_json::Value>,
}

impl Hub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn boot(&self) -> String {
        self.boot.read().map(|b| b.clone()).unwrap_or_default()
    }

    pub fn abonner(&self) -> broadcast::Receiver<HubMessage> {
        self.diffusion.subscribe()
    }

    pub fn version(&self, topic: &Topic, audience: &Audience) -> u64 {
        self.versions
            .lock()
            .ok()
            .and_then(|v| v.get(topic.as_str()).map(|x| x.pour(audience)))
            .unwrap_or(0)
    }

    pub fn versions(&self, sujets: &BTreeMap<Topic, Audience>) -> BTreeMap<String, u64> {
        sujets
            .iter()
            .map(|(t, a)| (t.as_str().to_owned(), self.version(t, a)))
            .collect()
    }

    /// Un signal reçu de la base : la version monte, les flux sont prévenus.
    /// Public pour les tests, qui peuvent ainsi se passer de la base.
    pub fn recevoir(&self, topic: &str, audience: Option<&str>) {
        Compteurs::plus(&self.compteurs.signaux_recus, 1);
        if let Ok(mut versions) = self.versions.lock() {
            let entree = versions.entry(topic.to_owned()).or_default();
            match audience {
                None => entree.sans_audience += 1,
                Some(a) => *entree.par_audience.entry(a.to_owned()).or_default() += 1,
            }
        }
        let _ = self.diffusion.send(HubMessage::Signal {
            topic: Arc::from(topic),
            audience: audience.map(Arc::from),
        });
    }

    /// Les signaux de l'intervalle sont perdus : un nouveau `boot` fait relire
    /// les pages au pouls, `Resync` celles qui tiennent un flux.
    fn perte(&self) {
        if let Ok(mut boot) = self.boot.write() {
            *boot = nouveau_boot();
        }
        let _ = self.diffusion.send(HubMessage::Resync);
    }

    /// À l'arrêt (SIGTERM) : les flux se ferment d'eux-mêmes, sans quoi Actix
    /// attendrait des réponses qui ne finissent jamais.
    pub fn annoncer_arret(&self) {
        let _ = self.diffusion.send(HubMessage::Restart);
    }

    /// Inscrit un flux de la session ; ferme le plus ancien au-delà de
    /// `FLUX_PAR_SESSION`. Rend l'identifiant du flux et son signal de fermeture.
    pub fn ouvrir_flux(&self, session: Uuid) -> (u64, Arc<Notify>) {
        let id = self.prochain_flux.fetch_add(1, Ordering::Relaxed);
        let fermeture = Arc::new(Notify::new());
        if let Ok(mut flux) = self.flux.lock() {
            let file = flux.entry(session).or_default();
            file.push_back((id, fermeture.clone()));
            while file.len() > FLUX_PAR_SESSION {
                if let Some((_, plus_ancien)) = file.pop_front() {
                    plus_ancien.notify_one();
                }
            }
        }
        Compteurs::plus(&self.compteurs.flux_ouverts, 1);
        Compteurs::plus(&self.compteurs.flux_total, 1);
        (id, fermeture)
    }

    pub fn fermer_flux(&self, session: Uuid, id: u64) {
        if let Ok(mut flux) = self.flux.lock() {
            if let Some(file) = flux.get_mut(&session) {
                file.retain(|(i, _)| *i != id);
                if file.is_empty() {
                    flux.remove(&session);
                }
            }
        }
        self.compteurs.flux_ouverts.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn autorisation_en_cache(&self, session: Option<Uuid>, topic: &Topic) -> Option<TopicAccess> {
        let cache = self.autorisations.lock().ok()?;
        let (quand, acces) = cache.get(&(session, topic.as_str().to_owned()))?;
        (quand.elapsed() < CACHE_AUTORISATION).then(|| acces.clone())
    }

    pub fn retenir_autorisation(&self, session: Option<Uuid>, topic: &Topic, acces: TopicAccess) {
        if let Ok(mut cache) = self.autorisations.lock() {
            if cache.len() >= CACHE_PLAFOND {
                cache.retain(|_, (quand, _)| quand.elapsed() < CACHE_AUTORISATION);
            }
            if cache.len() < CACHE_PLAFOND {
                cache.insert((session, topic.as_str().to_owned()), (Instant::now(), acces));
            }
        }
    }

    /// Lance l'écoute de la base et le journal des compteurs.
    ///
    /// Connexion dédiée, hors du pool des requêtes : une écoute qui attend ne
    /// doit pas priver une route de connexion.
    pub fn ecouter(self: &Arc<Self>, pool: &PgPool) {
        let options = (*pool.connect_options()).clone();
        let hub = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                match ecoute(&hub, &options).await {
                    Ok(()) => {}
                    Err(e) => tracing::warn!(erreur = %e, "temps réel : écoute de la base rompue"),
                }
                Compteurs::plus(&hub.compteurs.reprises_ecoute, 1);
                hub.perte();
                tokio::time::sleep(REPRISE_ECOUTE).await;
            }
        });

        let hub = Arc::clone(self);
        tokio::spawn(async move {
            let mut minuterie = tokio::time::interval(JOURNAL_COMPTEURS);
            minuterie.tick().await;
            loop {
                minuterie.tick().await;
                let c = &hub.compteurs;
                tracing::info!(
                    flux_ouverts = Compteurs::lire(&c.flux_ouverts),
                    flux_total = Compteurs::lire(&c.flux_total),
                    signaux_recus = Compteurs::lire(&c.signaux_recus),
                    signaux_livres = Compteurs::lire(&c.signaux_livres),
                    pouls_servis = Compteurs::lire(&c.pouls_servis),
                    pouls_inchanges = Compteurs::lire(&c.pouls_inchanges),
                    refus = Compteurs::lire(&c.refus),
                    reprises_ecoute = Compteurs::lire(&c.reprises_ecoute),
                    "temps réel : compteurs"
                );
            }
        });
    }
}

/// Une session d'écoute, jusqu'à sa rupture. `try_recv` plutôt que `recv` :
/// `recv` se reconnecte seul et tairait la perte des signaux de l'intervalle.
async fn ecoute(
    hub: &Hub,
    options: &sqlx::postgres::PgConnectOptions,
) -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await?;
    let mut listener = PgListener::connect_with(&pool).await?;
    listener.listen(SIGNAL_CHANNEL).await?;
    tracing::info!(canal = SIGNAL_CHANNEL, "temps réel : écoute ouverte");

    loop {
        match listener.try_recv().await? {
            Some(notification) => {
                match serde_json::from_str::<Notification>(notification.payload()) {
                    Ok(n) => {
                        let audience = n
                            .payload
                            .as_ref()
                            .and_then(|p| p.get("audience"))
                            .and_then(|a| a.as_str());
                        hub.recevoir(&n.topic, audience);
                    }
                    Err(e) => tracing::warn!(erreur = %e, "temps réel : signal illisible"),
                }
            }
            None => return Ok(()),
        }
    }
}
