//! Les lectures d'éditions du back-office, et leur garde.
//!
//! **Les trois cas du périmètre restent distincts jusqu'au bout** (FR-008,
//! FR-014) : administrer la plateforme entière, administrer telles éditions,
//! n'administrer rien. Les confondre afficherait une liste vide là où il faut un
//! refus d'accès, et personne ne saurait s'il n'y a rien à voir ou s'il n'a pas
//! le droit de voir. Le troisième cas se refuse **ici aussi**, et pas seulement
//! dans l'extracteur : un service qui rendrait une liste vide sur un périmètre
//! vide serait faux le jour où on l'appelle d'ailleurs.
//!
//! **La seule exception du module est ailleurs** et elle est écrite : le
//! sélecteur `GET /events` est *filtré* et non refusé, parce que le contrat du
//! front le veut ainsi.

use kernel::auth::AdminScope;
use kernel::error::{ApiError, Result};
use sqlx::PgExecutor;
use sqlx::PgPool;
use std::collections::BTreeMap;

use crate::domain::edition::{EditionListRow, EditionListScreen, EditionSeriesOption};
use crate::domain::ids::EventId;
use crate::repo::cross::Decomptes;
use crate::repo::{cross, editions};

/// Tout l'écran de la liste, **en une réponse** (FR-018).
pub async fn ecran(pool: &PgPool, perimetre: &AdminScope) -> Result<EditionListScreen> {
    if perimetre.is_empty() {
        return Err(ApiError::forbidden());
    }

    let bases = editions::lignes_du_perimetre(pool, perimetre).await?;
    let identifiants: Vec<uuid::Uuid> = bases.iter().map(|b| b.id).collect();
    let decomptes = cross::decomptes_par_edition(pool, &identifiants).await?;

    let series = facette_des_series(&bases);
    let years = facette_des_millesimes(&bases);

    let rows = bases
        .into_iter()
        .map(|base| {
            let d = decomptes.get(&base.id).cloned().unwrap_or_default();
            composer(base, d)
        })
        .collect();

    Ok(EditionListScreen {
        rows,
        series,
        years,
        is_global_scope: perimetre.is_global,
    })
}

/// L'édition **telle qu'elle est devenue** (FR-026), lue après une écriture :
/// c'est ce que l'écran réaffiche.
pub async fn ligne<'e>(
    executor: impl PgExecutor<'e> + Copy,
    id: EventId,
) -> Result<Option<EditionListRow>> {
    let Some(base) = editions::ligne(executor, id).await? else {
        return Ok(None);
    };
    let d = cross::decomptes(executor, id).await?;

    Ok(Some(composer(base, d)))
}

/// La ligne du modèle et ses décomptes joints, réunis. Écrit **une fois** :
/// deux compositions de vingt-huit champs finiraient par diverger.
pub fn composer(base: editions::LigneBase, d: Decomptes) -> EditionListRow {
    EditionListRow {
        id: base.id,
        title: base.title,
        acronym: base.acronym,
        slug: base.slug,
        series_id: base.series_id,
        series_name: base.series_name,
        series_kind: base.series_kind,
        edition_label: base.edition_label,
        edition_year: base.edition_year,
        status: base.status,
        participation_mode: base.participation_mode,
        timezone: base.timezone,
        starts_at: base.starts_at,
        ends_at: base.ends_at,
        country_id: base.country_id,
        country_name: base.country_name,
        city: base.city,
        address: base.address,
        latitude: base.latitude,
        longitude: base.longitude,
        has_pavilion: base.has_pavilion,
        programme_published_at: base.programme_published_at,
        proposal_count: d.proposal_count,
        session_count: d.session_count,
        scheduled_session_count: d.scheduled_session_count,
        call_status: base.call_status,
        call_deadline: base.call_deadline,
        day_count: d.day_count,
    }
}

// -----------------------------------------------------------------------------
// Les facettes
// -----------------------------------------------------------------------------

/// Les séries **présentes dans les lignes**, comptées sur ce même jeu (FR-018).
///
/// Proposer au filtre le catalogue entier ferait offrir des séries dont aucune
/// édition n'est visible — et à un compte détaché, la liste des séries de toute
/// la plateforme, qu'il n'a pas à connaître. Le décompte suit la même règle : un
/// « COP climat (4) » qui n'en montrerait qu'une est le défaut que B2 a corrigé
/// sur ses propres facettes.
fn facette_des_series(bases: &[editions::LigneBase]) -> Vec<EditionSeriesOption> {
    let mut par_serie: BTreeMap<uuid::Uuid, EditionSeriesOption> = BTreeMap::new();

    for base in bases {
        let (Some(id), Some(name), Some(kind)) = (
            base.series_id,
            base.series_name.clone(),
            base.series_kind.clone(),
        ) else {
            // Une édition hors série ne fabrique aucune facette, et n'en
            // disparaît pas pour autant de la liste.
            continue;
        };

        par_serie
            .entry(id)
            .or_insert_with(|| EditionSeriesOption {
                id,
                name,
                kind,
                is_active: base.series_is_active.unwrap_or(false),
                edition_count: 0,
            })
            .edition_count += 1;
    }

    let mut series: Vec<EditionSeriesOption> = par_serie.into_values().collect();
    // Les plus fournies d'abord : c'est l'ordre dans lequel un filtre se lit.
    series.sort_by(|a, b| b.edition_count.cmp(&a.edition_count).then(a.id.cmp(&b.id)));
    series
}

/// Les millésimes présents dans les lignes, **décroissants** : la prochaine COP
/// est ce qu'on vient chercher.
fn facette_des_millesimes(bases: &[editions::LigneBase]) -> Vec<i16> {
    let mut annees: Vec<i16> = bases.iter().map(|b| b.edition_year).collect();
    annees.sort_unstable_by(|a, b| b.cmp(a));
    annees.dedup();
    annees
}
