//! Pool, et l'**unique** façon d'ouvrir une transaction en écriture.
//!
//! Principe VII : une écriture sans `app.actor_id` ni `app.request_id` n'échoue
//! pas — elle écrit une trace anonyme, et rien ne le signale. La seule porte
//! d'écriture pose donc le contexte elle-même : l'oublier demanderait d'écrire
//! du code exprès.

use sqlx::postgres::{PgConnection, PgPool, PgPoolOptions};
use sqlx::{Postgres, Transaction};
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use uuid::Uuid;

use crate::context::RequestContext;
use crate::error::{ApiError, Result};

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Lectures, et les mécanismes qui exigent un vrai pool — écoute
    /// `LISTEN/NOTIFY`, chargement des tables de référence, harnais de test.
    ///
    /// Rien dans les types n'empêche d'y ouvrir une transaction : la porte
    /// unique est une discipline, pas une clôture. Toute écriture d'une table
    /// **auditée** passe par `write()`, sans quoi sa trace est anonyme et rien
    /// ne le signale.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Ouvre une transaction en écriture, contexte d'audit déjà posé.
    pub async fn write(&self, ctx: &RequestContext) -> Result<WriteTx> {
        let mut tx = self.pool.begin().await?;
        set_context(&mut tx, ctx).await?;
        Ok(WriteTx { tx })
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// `SET LOCAL` n'accepte pas de paramètre lié ; `set_config(_, _, true)` est
/// son équivalent paramétrable, et reste borné à la transaction.
async fn set_context(tx: &mut Transaction<'static, Postgres>, ctx: &RequestContext) -> Result<()> {
    sqlx::query!(
        "SELECT set_config('app.actor_id', $1, true) AS acteur,
                 set_config('app.request_id', $2, true) AS requete",
        ctx.actor_id.map(|id| id.to_string()).unwrap_or_default(),
        ctx.request_id
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(())
}

/// Repose `app.actor_id` **en cours de transaction**, quand l'acteur ne se
/// connaît qu'après la première lecture.
///
/// Le cas est celui de la réinitialisation de mot de passe : la personne n'a pas
/// de session, et son identifiant sort du jeton consommé — donc de l'intérieur
/// de la transaction. Sans cela, l'événement de domaine et l'audit porteraient
/// un acteur nul pour un changement qu'elle a bel et bien fait.
pub async fn set_actor(conn: &mut PgConnection, actor_id: Uuid) -> Result<()> {
    sqlx::query!(
        "SELECT set_config('app.actor_id', $1, true) AS acteur",
        actor_id.to_string()
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

pub struct WriteTx {
    tx: Transaction<'static, Postgres>,
}

impl WriteTx {
    pub async fn commit(self) -> Result<()> {
        self.tx.commit().await.map_err(ApiError::from)
    }

    pub async fn rollback(self) -> Result<()> {
        self.tx.rollback().await.map_err(ApiError::from)
    }
}

impl Deref for WriteTx {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

impl DerefMut for WriteTx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tx
    }
}
