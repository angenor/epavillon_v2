//! La liste du back-office, et sa garde.
//!
//! **Les trois cas du périmètre restent distincts jusqu'au bout** (FR-043) :
//! global, éditions listées, aucun droit → refus explicite. Les confondre
//! afficherait une liste vide là où il faut un refus d'accès, et personne ne
//! saurait s'il n'y a rien à voir ou s'il n'a pas le droit de voir.
//!
//! **La permission seule ne suffit pas** (écart n° 73) : elle est détenue par le
//! rôle d'utilisateur ordinaire. Il faut la permission **et** un périmètre non
//! vide — exactement la garde posée en B1 sur la liste des utilisateurs.

use kernel::auth::AdminScope;
use kernel::error::Result;
use sqlx::PgPool;

use crate::domain::admin::OrganizationListScreen;
use crate::repo::admin_list;

pub async fn screen(pool: &PgPool, perimetre: &AdminScope) -> Result<OrganizationListScreen> {
    let rows = admin_list::rows(pool, perimetre).await?;
    let (countries, types) = admin_list::facettes(&rows);
    let pending_duplicates = admin_list::paires_ouvertes(pool).await?;

    Ok(OrganizationListScreen {
        rows,
        countries,
        types,
        pending_duplicates,
        // L'écran le dit, plutôt que de laisser croire que la plateforme ne
        // compte que ces fiches.
        scoped_to_events: !perimetre.is_global,
    })
}
