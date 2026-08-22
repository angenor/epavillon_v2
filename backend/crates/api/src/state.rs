//! État partagé de l'API : tout ce qu'une requête peut avoir besoin de trouver.
//!
//! Il est bâti **une fois** au démarrage, puis cloné par ouvrier d'Actix. Chaque
//! pièce est aussi enregistrée seule dans `app_data` : les extracteurs du noyau
//! et des modules la cherchent par son type, sans connaître cet état.

use kernel::config::Config;
use kernel::crypto::Passwords;
use kernel::db::Db;
use kernel::error::Result;
use kernel::i18n::Locales;
use kernel::mail::Mailer;
use std::sync::Arc;

use crate::modules::ModuleRegistry;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    pub locales: Locales,
    pub passwords: Arc<Passwords>,
    pub mailer: Arc<dyn Mailer>,
    pub modules: ModuleRegistry,
    pub identity: identity::IdentityState,
    pub org: org::OrgState,
    pub event: event::EventState,
    pub programme: programme::ProgrammeState,
    pub media: media::MediaState,
    pub engagement: engagement::EngagementState,
    /// Origines acceptées sur une écriture. Celle du site, et rien d'autre.
    pub allowed_origins: Vec<String>,
}

impl AppState {
    /// Prend une base déjà ouverte plutôt que de la connecter : c'est ce qui
    /// permet à un test de monter la vraie application sur sa base jetable.
    pub async fn new(db: Db, config: Config) -> Result<Self> {
        let locales = Locales::load(db.pool()).await?;
        let modules = ModuleRegistry::load(db.pool()).await?;
        let passwords = Arc::new(Passwords::new()?);
        // **Le mailer est ENVELOPPÉ, et c'est tout l'écart n° 133.** La liste
        // de suppression et le journal d'expédition s'appliquent dès lors aux
        // six courriels de B1 et B2 **sans qu'aucun module livré ne change
        // d'une ligne** : le décorateur implémente le contrat du noyau, il ne
        // l'étend pas. C'est ce que `kernel::mail` annonçait en B1 — « le jour
        // où l'envoi se réécrit ici, aucun module ne bouge ».
        let mailer = engagement::GardedMailer::envelopper(&config.mail, db.clone());
        let allowed_origins = vec![config.app_public_url.clone()];
        let config = Arc::new(config);
        let identity = identity::IdentityState::new(db.clone(), config.clone(), passwords.clone())?;
        let org = org::OrgState::new(db.clone(), config.clone());
        let event = event::EventState::new(db.clone(), config.clone());
        let programme = programme::ProgrammeState::new(db.clone(), config.clone());
        let media = media::MediaState::new(db.clone(), config.clone());
        let engagement =
            engagement::EngagementState::new(db.clone(), config.clone(), mailer.clone());

        Ok(Self {
            db,
            config,
            locales,
            passwords,
            mailer,
            modules,
            identity,
            org,
            event,
            programme,
            media,
            engagement,
            allowed_origins,
        })
    }
}
