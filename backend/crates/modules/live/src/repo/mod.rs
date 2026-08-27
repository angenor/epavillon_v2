//! Les lectures du module, et l'ouverture d'une **transaction de lecture**.

pub mod active;
pub mod cross;
pub mod incidents;
pub mod kinds;

use kernel::error::Result;
use sqlx::{PgPool, Postgres, Transaction};

/// Ouvre une transaction de lecture — **un instantané, un instant**.
///
/// `now()` vaut `transaction_timestamp()` : toutes les parties d'une composition
/// parlent donc du même instant **sans qu'on passe un horodatage de main en
/// main**, et `REPEATABLE READ` y ajoute un instantané unique. C'est la réponse
/// exacte aux « neuf instants de mesure » qu'interdit le contrat du site.
///
/// **Pas `Db::write()`** : il prend une connexion d'écriture et pose un contexte
/// d'audit pour une requête qui n'écrit rien.
pub async fn lecture(pool: &PgPool) -> Result<Transaction<'_, Postgres>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
