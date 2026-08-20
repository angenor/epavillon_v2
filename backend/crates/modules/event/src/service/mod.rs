//! Les règles du module. Un fichier par geste.
//!
//! Ce fichier-ci ne porte que ce qui précède **tous** les gestes du back-office :
//! remonter à l'édition, puis vérifier le périmètre.

pub mod call;
pub mod channels;
pub mod committee;
pub mod days;
pub mod detail;
pub mod edition_read;
pub mod edition_write;
pub mod public_read;
pub mod publication;
pub mod tabs;
pub mod tracks;
pub mod venues;

use kernel::auth::{Perimeter, Scope};
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::PgPool;

use crate::domain::ids::{CallId, ChannelId, EventDayId, EventId, RoomId, TrackId, VenueId};
use crate::repo::cross;

/// Ce qu'une route paramétrée désigne. Six enfants de l'édition, plus l'édition
/// elle-même : c'est la liste exhaustive de ce par quoi une URL peut entrer.
#[derive(Debug, Clone, Copy)]
pub enum Cible {
    Edition(EventId),
    Fil(TrackId),
    Lieu(VenueId),
    Salle(RoomId),
    Appel(CallId),
    Journee(EventDayId),
}

/// Ce qu'un identifiant de canal désigne. Un canal **général de la plateforme**
/// n'appartient à aucune édition : il n'est ni introuvable ni hors périmètre,
/// et son refus est celui du contrat (`platform_channel`), pas celui du garde.
#[derive(Debug, Clone, Copy)]
pub enum CanalCible {
    Edition(EventId),
    Plateforme,
}

/// **Résoudre l'ascendance, PUIS vérifier le périmètre.** L'ordre est imposé :
/// vérifier d'abord reviendrait à croire l'édition que le client annonce.
///
/// Deux choses valent d'être dites ici plutôt que dans chaque gestionnaire.
///
/// **L'`event_id` du corps de requête est ignoré.** Le front l'envoie sur les
/// routes de suppression (`{ event_id: eventId }`) et dans ses charges utiles
/// d'écriture ; c'est un droit *déclaré par le client*. L'édition vient
/// toujours de l'ascendance en base — même motif que les paramètres
/// `personId` / `actorId` écartés en B1 et B2 (principe V).
///
/// **L'absence de l'objet produit le MÊME refus que l'échec du périmètre.** Un
/// identifiant inexistant et un identifiant hors périmètre sont indiscernables
/// par la forme de la réponse : sans cela, une URL forgée dirait à qui la forge
/// si l'objet existe (principe IX, research.md § R2).
pub async fn edition_dans_le_perimetre(
    pool: &PgPool,
    perimetre: &Perimeter,
    cible: Cible,
) -> Result<EventId> {
    let edition = match cible {
        Cible::Edition(id) => cross::event_exists(pool, id).await?.then_some(id),
        Cible::Fil(id) => cross::event_id_of_track(pool, id).await?,
        Cible::Lieu(id) => cross::event_id_of_venue(pool, id).await?,
        Cible::Salle(id) => cross::event_id_of_room(pool, id).await?,
        Cible::Appel(id) => cross::event_id_of_call(pool, id).await?,
        Cible::Journee(id) => cross::event_id_of_day(pool, id).await?,
    };

    let edition = edition.ok_or_else(ApiError::not_found)?;
    perimetre.ensure(edition.as_uuid())?;
    Ok(edition)
}

/// Le cas du canal, qui a une issue de plus que les six autres.
pub async fn canal_dans_le_perimetre(
    pool: &PgPool,
    perimetre: &Perimeter,
    canal: ChannelId,
) -> Result<CanalCible> {
    match cross::event_id_of_channel(pool, canal).await? {
        None => Err(ApiError::not_found()),
        Some(None) => Ok(CanalCible::Plateforme),
        Some(Some(edition)) => {
            perimetre.ensure(edition.as_uuid())?;
            Ok(CanalCible::Edition(edition))
        }
    }
}

/// **La création d'une édition exige la portée GLOBALE, et pas une autre**
/// (FR-011).
///
/// Une édition qui n'existe pas encore n'offre aucune portée où vérifier un
/// droit : exiger la permission « sur cette édition » reviendrait à la vérifier
/// sur un identifiant que personne ne détient — donc à l'accorder à tout le
/// monde, ou à personne, selon le sens du test.
///
/// Le refus est **distinct de `FORBIDDEN`** parce que l'écran sait en tirer une
/// phrase : « demandez des droits sur l'ensemble de la plateforme » n'est pas
/// « vous n'avez pas les droits ».
pub async fn portee_globale_exigee(pool: &PgPool, personne: uuid::Uuid) -> Result<()> {
    let autorise = kernel::auth::has_permission(
        pool,
        personne,
        crate::domain::permissions::EVENT_MANAGE,
        Scope::Global,
    )
    .await?;

    if autorise {
        Ok(())
    } else {
        Err(ApiError::new(ErrorCode::EventGlobalScopeRequired))
    }
}
