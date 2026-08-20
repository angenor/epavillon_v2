//! Recalcul du score de confiance.
//!
//! **Pas un trigger, et c'est raisonné.** Le score est alimenté par quatre
//! tables, dont les adhésions et les domaines : un trigger recalculerait un
//! agrégat à chaque adhésion approuvée, sur un chemin d'écriture fréquent, pour
//! une valeur qui ne sert qu'à trier une liste de back-office. Le principe VIII
//! ne s'applique pas — rien n'est faux si le score a dix secondes de retard, ce
//! n'est pas un invariant. Et le § 7 du modèle annonçait déjà « recalculé par le
//! worker » : cette décision rend vrai un commentaire qui ne l'était pas
//! (research.md § R12).
//!
//! **L'acteur est absent, et c'est dit.** Un recalcul de système n'a pas
//! d'auteur : c'est la deuxième trace anonyme légitime du projet, après
//! l'inscription de soi-même. Elle est nommée ici pour qu'elle ne se découvre
//! pas en lisant un journal d'audit.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde::Deserialize;
use serde_json::json;
use sqlx::postgres::PgConnection;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::ids::OrganizationId;

pub const RECOMPUTE_TRUST_SCORE: &str = "org.trust_score.recompute";

/// Quelques secondes de délai : le temps que les écritures d'un même geste —
/// vérifier un domaine, poser le sceau, approuver une adhésion — se rangent
/// derrière une seule clé.
const DELAI: Duration = Duration::seconds(5);

/// Met le recalcul en file, **dans la transaction en cours**.
///
/// La clé d'unicité porte l'organisation : cent approbations coup sur coup
/// produisent **un** recalcul (SC-014).
pub async fn planifier(conn: &mut PgConnection, organization_id: OrganizationId) -> Result<()> {
    jobs::enqueue(
        conn,
        NewJob::new(
            RECOMPUTE_TRUST_SCORE,
            json!({ "organization_id": organization_id.as_uuid() }),
        )
        .idempotent(format!("{RECOMPUTE_TRUST_SCORE}:{organization_id}"))
        .at(OffsetDateTime::now_utc() + DELAI),
    )
    .await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct Charge {
    organization_id: Uuid,
}

pub struct RecomputeTrustScore {
    db: Db,
}

impl RecomputeTrustScore {
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobHandler for RecomputeTrustScore {
    fn task(&self) -> &'static str {
        RECOMPUTE_TRUST_SCORE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone())
            .map_err(|e| ApiError::internal(format!("charge utile illisible : {e}")))?;

        let mut tx = self.db.write(&job.context()).await?;

        // **N'écrit que si la valeur change**, et ce n'est pas une optimisation :
        // sans la condition, chaque recalcul poserait une ligne d'audit et
        // remonterait la date de dernière modification de la fiche, donc son
        // rang dans le tri « dernière activité ». L'historique de la fiche se
        // remplirait de lignes que personne n'a écrites.
        let nouveau = sqlx::query_scalar!(
            r#"WITH calcul AS (SELECT org.compute_trust_score($1) AS score)
               UPDATE org.organizations o
                  SET trust_score = c.score
                 FROM calcul c
                WHERE o.id = $1 AND o.trust_score IS DISTINCT FROM c.score
            RETURNING o.trust_score"#,
            charge.organization_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        if let Some(score) = nouveau {
            tracing::info!(
                organisation = %charge.organization_id,
                score,
                "score de confiance recalculé"
            );
        }
        Ok(())
    }
}
