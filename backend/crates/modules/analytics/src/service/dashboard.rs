//! **Tout l'écran en une réponse et un instant.**
//!
//! # POURQUOI UNE COMPOSITION ET NON DIX LECTURES
//!
//! L'écran ouvre sur cinq familles d'alerte, quatre projections et une vue de
//! santé. Lues séparément par le site, elles produiraient **dix allers-retours**
//! au chargement d'une page qu'on consulte vingt fois par jour, et **dix
//! instants de mesure différents dans un même écran** — les incidents actifs
//! comptés à une seconde, l'échéance calculée à une autre.
//!
//! # UNE TRANSACTION DE LECTURE, UN INSTANT
//!
//! `now()` vaut `transaction_timestamp()` : il est **constant** pour toute la
//! transaction. Les dix lectures parlent donc du même instant **sans qu'on passe
//! un horodatage de main en main**, et `REPEATABLE READ` y ajoute un instantané
//! unique — un dossier accepté pendant la composition ne peut pas être compté
//! par l'entonnoir sans l'être par la famille d'alerte.
//!
//! Un point à surveiller : la composition tient une connexion de lecture pendant
//! une dizaine de requêtes. Elle est en lecture seule et son isolation ne bloque
//! rien, mais elle n'a pas à s'allonger.

use kernel::auth::{require_permission, Perimeter, PermissionSpec, Scope};
use kernel::error::{ApiError, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::authz::DashboardRead;
use crate::domain::dashboard::AdminDashboard;
use crate::repo;
use crate::repo::cross::{event, live};
use crate::state::AnalyticsState;

/// L'écran, **gardé par le périmètre ET par `analytics.dashboard.read` sur
/// l'édition demandée**.
///
/// Les trois cas du périmètre restent distincts : aucun droit → **403**
/// explicite et jamais un écran vide ; hors périmètre → **404**, indiscernable
/// d'une édition inexistante ; permission absente → **403**.
///
/// **La permission se teste sur l'ÉDITION**, pas « quelle que soit la portée » :
/// un compte qui administre la COP31 ne lit pas le tableau de bord de la COP30.
/// Le rôle `programmer` la détient depuis le 27/08, sur la portée de son
/// attribution.
///
/// **Le tableau de bord n'a pas d'issue de contrat** : il s'ouvre, ou il se
/// refuse — à la différence des écritures d'incident, dont les dix issues
/// sortent en 200.
pub async fn ecran(
    state: &AnalyticsState,
    perimetre: &Perimeter,
    event_id: Uuid,
) -> Result<AdminDashboard> {
    if perimetre.scope.is_empty() {
        return Err(ApiError::forbidden());
    }
    perimetre.ensure(event_id)?;

    require_permission(
        state.pool(),
        perimetre.person_id,
        DashboardRead::CODE,
        Scope::Event(event_id),
    )
    .await?;

    composer(state.pool(), event_id).await
}

pub async fn composer(pool: &PgPool, event_id: Uuid) -> Result<AdminDashboard> {
    let mut tx = repo::lecture(pool).await?;

    let edition = event::edition(&mut tx, event_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let call = event::appel(&mut tx, event_id).await?;

    let actions = crate::service::actions::composer(&mut tx, event_id).await?;
    let figures =
        crate::service::figures::composer(&mut tx, event_id, call.as_ref().map(|c| c.id)).await?;
    let health = repo::health::sante(&mut tx).await?;
    let incidents = live::incidents_actifs(&mut tx, event_id).await?;

    tx.commit().await?;

    Ok(AdminDashboard {
        timezone: edition.timezone.clone(),
        edition,
        call,
        actions,
        figures,
        health,
        incidents,
    })
}
