//! La composition de l'écran des messages d'incident — **en une transaction de
//! lecture, donc en un instant**.
//!
//! Sept lectures : l'édition, les lignes, le poste de direct, ses compteurs
//! d'incidents actifs, les natures, les journées, les activités et les
//! organisations visables. Toutes dans la même transaction : `now()` y vaut
//! `transaction_timestamp()` et ne bouge pas, et `REPEATABLE READ` y ajoute un
//! instantané unique. Un message qui expire entre la deuxième et la sixième
//! lecture ne peut donc pas apparaître actif dans la liste et absent du
//! compteur.

use kernel::auth::Perimeter;
use kernel::error::{ApiError, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::desk::{
    IncidentListScreen, IncidentStateCounts, IncidentTargetOption, IncidentTargets,
    OverrunTemplate, ETATS,
};
use crate::domain::incident::ManagedIncident;
use crate::repo;
use crate::repo::cross::{event, org, programme};
use crate::service::write;
use crate::state::LiveState;

/// L'écran, **gardé par le périmètre** : vide → 403, hors périmètre → 404.
///
/// **Aucune permission n'est exigée** (R11) : lire les messages d'une édition
/// qu'on administre n'est pas un privilège — un bandeau publié est de toute
/// façon public, et en exiger une protégerait un texte déjà lisible de tous.
pub async fn ecran(
    state: &LiveState,
    perimetre: &Perimeter,
    event_id: Uuid,
    locale: &str,
) -> Result<IncidentListScreen> {
    write::assurer_le_perimetre(perimetre, event_id)?;
    composer(state.pool(), event_id, locale).await
}

/// Le gabarit du raccourci « Signaler un débordement ».
///
/// **Le périmètre se vérifie sur l'édition de l'ACTIVITÉ**, une fois celle-ci
/// retrouvée : une activité inexistante et une activité hors périmètre rendent
/// le même 404.
pub async fn gabarit_de_debordement(
    state: &LiveState,
    perimetre: &Perimeter,
    session_id: Uuid,
    locale: &str,
) -> Result<OverrunTemplate> {
    write::refuser_un_perimetre_vide(perimetre)?;

    let mut tx = repo::lecture(state.pool()).await?;
    let gabarit = programme::gabarit(&mut tx, session_id, locale).await?;
    tx.commit().await?;

    let gabarit = gabarit.ok_or_else(ApiError::not_found)?;
    perimetre.ensure(gabarit.event_id)?;

    Ok(OverrunTemplate {
        session_id: gabarit.session_id,
        title: gabarit.title,
        starts_at: gabarit.starts_at,
        ends_at: gabarit.ends_at,
        event_id: gabarit.event_id,
    })
}

/// Un message, relu dans le périmètre. **404 quand il n'y est pas** — inexistant
/// et hors périmètre sont indiscernables.
pub async fn relire(state: &LiveState, perimetre: &Perimeter, id: Uuid) -> Result<ManagedIncident> {
    write::refuser_un_perimetre_vide(perimetre)?;
    write::charger_dans_le_perimetre(state, perimetre, id)
        .await?
        .ok_or_else(ApiError::not_found)
}

pub async fn composer(pool: &PgPool, event_id: Uuid, locale: &str) -> Result<IncidentListScreen> {
    let mut tx = repo::lecture(pool).await?;

    let entete = event::entete(&mut tx, event_id, locale)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let rows = repo::incidents::de_ledition(&mut tx, event_id).await?;

    // **Les compteurs sont établis AVANT tout filtrage**, comme partout dans le
    // projet, et les cinq états sont posés à zéro d'abord : un état absent de la
    // réponse ferait afficher un tiret là où l'écran attend un décompte.
    let mut counts: IncidentStateCounts = ETATS
        .iter()
        .map(|etat| ((*etat).to_owned(), 0_i64))
        .collect();
    for ligne in &rows {
        *counts.entry(ligne.state.clone()).or_insert(0) += 1;
    }

    let desk = crate::service::desk::composer(&mut tx, event_id, entete.aujourdhui).await?;
    let kinds = repo::kinds::natures(&mut tx).await?;

    let targets = IncidentTargets {
        event: IncidentTargetOption {
            id: entete.event_id,
            label: entete.title_label.clone(),
            hint: entete.acronym.clone(),
            starts_at: None,
        },
        days: event::journees(&mut tx, event_id, locale)
            .await?
            .into_iter()
            .map(|j| IncidentTargetOption {
                id: j.id,
                label: j.label,
                hint: Some(j.day_date.to_string()),
                starts_at: None,
            })
            .collect(),
        sessions: programme::cibles(&mut tx, event_id, locale)
            .await?
            .into_iter()
            .map(|s| IncidentTargetOption {
                id: s.id,
                label: s.label,
                hint: None,
                // **Un INSTANT, jamais une précision textuelle** : l'interface
                // seule sait l'afficher dans le fuseau de l'édition.
                starts_at: Some(s.starts_at),
            })
            .collect(),
        organizations: org::animant_ledition(&mut tx, event_id)
            .await?
            .into_iter()
            .map(|o| IncidentTargetOption {
                id: o.id,
                label: o.legal_name,
                hint: o.acronym,
                starts_at: None,
            })
            .collect(),
    };

    tx.commit().await?;

    Ok(IncidentListScreen {
        event_id: entete.event_id,
        event_title: entete.title,
        timezone: entete.timezone,
        zone_label: entete.zone_label,
        rows,
        desk,
        counts,
        kinds,
        targets,
    })
}
