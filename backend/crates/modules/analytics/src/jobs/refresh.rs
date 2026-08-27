//! Le rafraîchissement périodique des huit projections.
//!
//! # LA MISE EN FILE PASSE PAR LA FONCTION DU MODÈLE
//!
//! `analytics.enqueue_refresh()`, **jamais `kernel::jobs::enqueue`** : c'est
//! elle qui pose la file `analytics`, la tâche, la priorité 200, les trois
//! tentatives et la **clé d'anti-rebond**. La recopier ici ferait une seconde
//! définition de la mise en file, et la première évolution du SQL les ferait
//! diverger.
//!
//! # LE PIÈGE QU'ELLE TEND, ET QUI EST ENTIÈREMENT SILENCIEUX
//!
//! La clé d'unicité est `refresh_all:<tranche>`, la tranche étant l'horloge
//! arrondie au pas d'anti-rebond. Le conflit de `platform.jobs` porte sur
//! `(task, idempotency_key)` **quel que soit l'état du travail**, `cancelled`
//! excepté : **un travail déjà réussi bloque une nouvelle mise en file de la
//! même tranche.**
//!
//! Si l'intervalle n'excédait pas la fenêtre d'anti-rebond, la chaîne se
//! dédoublonnerait donc **contre elle-même** et s'arrêterait — sans erreur, sans
//! trace, et sans que rien à l'écran ne le dise, sinon une fraîcheur qui cesse
//! d'avancer. `kernel::config` refuse cette configuration au démarrage.
//!
//! # LE RAFRAÎCHISSEMENT EST HORS TRANSACTION D'ÉCRITURE
//!
//! `analytics.refresh_all(true)` journalise elle-même, n'écrit dans **aucune
//! table auditée**, et rafraîchit en mode concurrent — donc sans bloquer
//! aucune lecture. L'envelopper dans `Db::write()` tiendrait une connexion
//! d'écriture et ses verrous pour rien.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::Result;
use kernel::jobs::{ClaimedJob, JobHandler};
use sqlx::postgres::PgConnection;
use std::time::Duration;
use time::OffsetDateTime;

pub const REFRESH_ALL: &str = "analytics.refresh_all";
pub const QUEUE: &str = "analytics";

pub struct RefreshAll {
    db: Db,
    intervalle: Duration,
    anti_rebond: Duration,
}

impl RefreshAll {
    pub fn new(db: Db, intervalle: Duration, anti_rebond: Duration) -> Self {
        Self {
            db,
            intervalle,
            anti_rebond,
        }
    }
}

#[async_trait]
impl JobHandler for RefreshAll {
    fn task(&self) -> &'static str {
        REFRESH_ALL
    }

    /// **C'est cette déclaration qui fait écouter la file.**
    /// `JobRegistry::queues()` est construite à partir des files que les
    /// gestionnaires nomment, et `platform.claim_jobs()` filtre strictement.
    fn queue(&self) -> &'static str {
        QUEUE
    }

    /// **Non.** La charge utile — `{ concurrently, tranche }` — est la seule
    /// matière de diagnostic d'un rafraîchissement mort : l'effacer priverait de
    /// tout moyen de savoir quelle tranche a fauté.
    fn carries_secret(&self) -> bool {
        false
    }

    async fn run(&self, _job: &ClaimedJob) -> Result<()> {
        let echecs = rafraichir(self.db.pool()).await?;

        // **Un échec de vue n'est pas un échec de travail.** La fonction isole
        // les vues les unes des autres : un tableau de bord partiellement à jour
        // vaut mieux qu'un tableau de bord entièrement périmé parce qu'une seule
        // agrégation a fauté. C'est la décision du modèle, pas la nôtre — et
        // `refreshed_at` ne bougera pas pour ces vues-là, puisque la fraîcheur
        // se lit sur les succès.
        if !echecs.is_empty() {
            tracing::warn!(
                vues = ?echecs,
                "rafraîchissement analytique partiel : ces projections ont fauté"
            );
        }

        // La suivante naît **après** le rafraîchissement, dans sa propre
        // transaction d'écriture : une chaîne rompue arrêterait la fraîcheur
        // sans rien dire.
        let mut tx = self
            .db
            .write(&kernel::context::RequestContext::background("jobs"))
            .await?;
        planifier(&mut tx, self.intervalle, self.anti_rebond).await?;
        tx.commit().await?;

        Ok(())
    }
}

/// Appelle `analytics.refresh_all(true)` **sur le pool, hors transaction
/// d'écriture**, et rend le nom des vues en échec.
///
/// Le mode concurrent depuis une fonction et depuis un bloc transactionnel a été
/// **mesuré** sur la base du dépôt — huit vues, huit succès, dans les deux cas.
/// Le contraire aurait journalisé huit échecs **sans lever**, l'exception étant
/// avalée vue par vue, et le tableau de bord aurait vieilli en silence pendant
/// que le worker croyait travailler.
pub async fn rafraichir(pool: &sqlx::PgPool) -> Result<Vec<String>> {
    let lignes = sqlx::query!(
        r#"SELECT vue AS "vue?", succes AS "succes?", erreur
             FROM analytics.refresh_all(true)"#
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .filter(|l| !l.succes.unwrap_or(false))
        .filter_map(|l| l.vue)
        .collect())
}

/// Pose la prochaine demande. **Vrai** : elle a été posée. **Faux** : une
/// demande de la même tranche existait déjà — ce n'est pas une erreur, c'est le
/// résultat attendu de l'anti-rebond.
///
/// Dix redémarrages dans la même tranche n'arment donc pas dix rafraîchissements.
pub async fn planifier(
    conn: &mut PgConnection,
    intervalle: Duration,
    anti_rebond: Duration,
) -> Result<bool> {
    let delai = intervalle.as_secs_f64();
    let debounce = anti_rebond.as_secs().max(1) as i32;

    let id = sqlx::query_scalar!(
        r#"SELECT analytics.enqueue_refresh(true, make_interval(secs => $1), $2) AS "id?""#,
        delai,
        debounce
    )
    .fetch_one(conn)
    .await?;

    Ok(id.is_some())
}

/// Le prochain créneau de la grille, ancrée à l'époque Unix.
///
/// L'ancrage sur une grille plutôt que sur « dans un quart d'heure » a la même
/// raison qu'ailleurs dans le dépôt : sans lui, le créneau dériverait d'un
/// redémarrage à l'autre. Ici il sert à **dater** la chaîne dans les traces —
/// l'unicité, elle, est portée par la tranche d'anti-rebond de la fonction.
pub fn prochaine_occurrence(depuis: OffsetDateTime, intervalle: Duration) -> OffsetDateTime {
    let pas = intervalle.as_secs().max(1) as i64;
    let suivant = (depuis.unix_timestamp().div_euclid(pas) + 1) * pas;
    OffsetDateTime::from_unix_timestamp(suivant).unwrap_or(depuis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn le_creneau_suivant_tombe_sur_la_grille() {
        let quart_dheure = Duration::from_secs(900);
        assert_eq!(
            prochaine_occurrence(datetime!(2026-08-27 10:17:33 UTC), quart_dheure),
            datetime!(2026-08-27 10:30:00 UTC)
        );
    }

    /// Deux démarrages du même créneau visent le même instant : c'est ce qui
    /// fait qu'un seul travail naît.
    #[test]
    fn deux_demarrages_du_meme_creneau_visent_le_meme_instant() {
        let quart_dheure = Duration::from_secs(900);
        assert_eq!(
            prochaine_occurrence(datetime!(2026-08-27 10:00:01 UTC), quart_dheure),
            prochaine_occurrence(datetime!(2026-08-27 10:14:59 UTC), quart_dheure)
        );
    }
}
