//! Travaux différés du module. Le worker les monte sans les connaître : il lit
//! le nom de la tâche dans `platform.jobs` et cherche son gestionnaire.
//!
//! **Aucun consommateur d'événement ici, et c'est un choix.** Les trois travaux
//! de fond sont mis en file **dans la transaction qui les rend nécessaires**, ce
//! qui est plus simple et plus sûr : si la transaction est annulée, le travail
//! ne naît pas. Un consommateur ne se justifierait que pour un effet appartenant
//! à un **autre** module — il n'y en a aucun dans ce jalon. Le jour où B6
//! enverra les courriels d'adhésion depuis `engagement`, ce sera par un
//! consommateur des six événements, et rien d'ici n'aura à changer.

pub mod duplicates;
pub mod emails;
pub mod scorecard;
pub mod trust_score;

use kernel::config::Config;
use kernel::error::Result;
use sqlx::postgres::PgConnection;

use crate::domain::ids::OrganizationId;

/// Ce que toute écriture affectant une fiche met en file : le recalcul de son
/// score, et le rafraîchissement de la projection analytique.
///
/// Les deux sont **coalescés**, mais pas sur la même clé : le score l'est par
/// organisation — cent adhésions approuvées coup sur coup produisent un
/// recalcul —, la projection sur une fenêtre de temps, parce qu'elle est
/// globale et se compte en secondes.
pub async fn planifier_apres_ecriture(
    conn: &mut PgConnection,
    config: &Config,
    organization_id: OrganizationId,
) -> Result<()> {
    trust_score::planifier(conn, organization_id).await?;
    scorecard::planifier(conn, config.org.scorecard_refresh_window).await?;
    Ok(())
}
