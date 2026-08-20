//! Noyau technique : contexte de requête, erreurs, accès base, événements,
//! travaux différés, autorisation. Aucune connaissance métier.
//!
//! Il ne dépend d'aucun crate de module, et aucun module ne dépend d'un autre :
//! c'est le principe II, et c'est ce qui rend l'extraction d'un service
//! possible sans archéologie.

pub mod auth;
pub mod config;
pub mod context;
pub mod crypto;
pub mod db;
pub mod error;
pub mod events;
pub mod i18n;
pub mod jobs;
pub mod mail;
pub mod net;
pub mod pg_error;
pub mod telemetry;
pub mod tokens;

#[cfg(feature = "testing")]
pub mod testing;

pub use config::Config;
pub use context::RequestContext;
pub use db::{Db, WriteTx};
pub use error::{ApiError, ErrorCode, Result};
