//! Les services : ce que les routes appellent, et ce que les tests éprouvent.
//!
//! [`authz`] applique la table de gardes de `domain/guards.rs` — la déclaration
//! est là-bas, la résolution ici.

pub mod admin;
pub mod attach;
pub mod authz;
pub mod read;
pub mod stream;
pub mod upload;
