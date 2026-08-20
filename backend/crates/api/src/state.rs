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
        let mailer = kernel::mail::build(&config.mail);
        let allowed_origins = vec![config.app_public_url.clone()];
        let config = Arc::new(config);
        let identity = identity::IdentityState::new(db.clone(), config.clone(), passwords.clone())?;
        let org = org::OrgState::new(db.clone(), config.clone());

        Ok(Self {
            db,
            config,
            locales,
            passwords,
            mailer,
            modules,
            identity,
            org,
            allowed_origins,
        })
    }
}
