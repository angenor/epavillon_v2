//! Ce dont les routes du module ont besoin, et rien de plus.
//!
//! Le codec de jeton d'accès est construit **une fois** : il porte la clé de
//! signature, et la dériver à chaque requête coûterait sans rien apporter.

use kernel::config::Config;
use kernel::crypto::Passwords;
use kernel::db::Db;
use kernel::error::Result;
use sqlx::PgPool;
use std::sync::Arc;

use crate::domain::access_token::AccessTokenCodec;

#[derive(Clone)]
pub struct IdentityState {
    db: Db,
    config: Arc<Config>,
    passwords: Arc<Passwords>,
    tokens: Arc<AccessTokenCodec>,
}

impl IdentityState {
    pub fn new(db: Db, config: Arc<Config>, passwords: Arc<Passwords>) -> Result<Self> {
        let tokens = Arc::new(AccessTokenCodec::new(
            &config.auth.signing_key,
            config.auth.access_token_ttl,
        )?);

        Ok(Self {
            db,
            config,
            passwords,
            tokens,
        })
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn pool(&self) -> &PgPool {
        self.db.pool()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn passwords(&self) -> &Passwords {
        &self.passwords
    }

    /// Le hachage part sur un fil d'exécution dédié : trente à cent
    /// millisecondes de calcul sur le fil du réacteur y bloqueraient toutes les
    /// autres requêtes.
    pub fn shared_passwords(&self) -> Arc<Passwords> {
        self.passwords.clone()
    }

    pub fn tokens(&self) -> &AccessTokenCodec {
        &self.tokens
    }

    /// L'intergiciel de session de l'API n'a besoin que de cela : il vérifie une
    /// signature, il ne connaît pas le module.
    pub fn token_codec(&self) -> Arc<AccessTokenCodec> {
        self.tokens.clone()
    }
}
