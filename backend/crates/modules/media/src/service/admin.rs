//! **Le back-office du média** : les orphelins, la suppression d'un objet, et le
//! tableau des quotas.
//!
//! # Ce que ce fichier ne fait pas, et c'est le piège de cette histoire
//!
//! `media.schedule_asset_purge()` **émet déjà** `media.asset.purge_scheduled`.
//! La suppression ne redouble donc rien : elle appelle la fonction, un point.
//! Le seul événement que ce module émet est `media.asset.purged`, quand l'objet
//! a réellement quitté le stockage — et il vit dans [`crate::jobs::purge`].
//! Annoncer l'intention deux fois ferait réagir deux fois qui l'écoute.
//!
//! # Les orphelins ne se recalculent pas
//!
//! `media.find_orphan_assets()` range déjà du plus lourd au plus léger et exclut
//! ce qui est rattaché. La réécrire en SQL d'ici ferait une seconde définition
//! de l'orphelin, et la première évolution du modèle les ferait diverger.

use kernel::auth::{has_permission, require_perimeter, require_permission, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::Deserialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::asset::{OrphanAsset, QuotaRow};
use crate::repo::{assets, attachments, cross, quotas};
use crate::state::MediaState;

/// La permission qui garde les trois lectures d'administration du média. Elle
/// vient du module Organisations : aucune permission `media.*` n'existe
/// (écart n° 127).
const PERMISSION_ADMIN: &str = "org.organization.manage";

/// Ce que rend la suppression d'un objet. **La consommation a déjà baissé** :
/// c'est la suppression logique qui la rend, pas la purge (FR-106).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PurgeProgrammee {
    #[serde(with = "time::serde::rfc3339::option")]
    pub scheduled_purge_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PlafondPayload {
    pub max_bytes: i64,
    pub max_files: i32,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrphelinsQuery {
    pub min_age_days: Option<i32>,
}

// -----------------------------------------------------------------------------
// Les orphelins et les quotas — le back-office
// -----------------------------------------------------------------------------

/// Les objets que plus rien n'utilise, du plus lourd au plus léger.
///
/// L'ancienneté par défaut vient des réglages, jamais d'une constante : le
/// modèle la déclare comme réglage précisément pour qu'elle puisse changer sans
/// redéploiement.
pub async fn orphelins(
    state: &MediaState,
    acteur: Uuid,
    anciennete: Option<i32>,
) -> Result<Vec<OrphanAsset>> {
    exiger_le_back_office(state, acteur).await?;

    let jours = match anciennete {
        Some(j) if j >= 0 => j,
        Some(_) => {
            return Err(ApiError::validation(
                "L'ancienneté ne peut pas être négative.",
                "min_age_days",
            ))
        }
        None => assets::anciennete_dorphelin(state.pool()).await?,
    };

    assets::orphelins(state.pool(), jours).await
}

/// Le tableau des quotas, trié par proximité du plafond.
pub async fn quotas(state: &MediaState, acteur: Uuid) -> Result<Vec<QuotaRow>> {
    exiger_le_back_office(state, acteur).await?;

    let lignes = quotas::tableau(state.pool()).await?;
    nommer(state, lignes).await
}

/// Relever — ou abaisser — le plafond d'une organisation. Prend effet
/// immédiatement : `media.has_storage_capacity()` lit la ligne, sans cache.
pub async fn relever_le_plafond(
    state: &MediaState,
    ctx: &RequestContext,
    acteur: Uuid,
    organization_id: Uuid,
    payload: &PlafondPayload,
) -> Result<QuotaRow> {
    exiger_le_back_office(state, acteur).await?;

    if payload.max_bytes <= 0 {
        return Err(ApiError::validation(
            "Un plafond de stockage est un nombre d'octets strictement positif.",
            "max_bytes",
        ));
    }
    if payload.max_files <= 0 {
        return Err(ApiError::validation(
            "Un plafond de fichiers est strictement positif.",
            "max_files",
        ));
    }
    if !cross::organisation_existe(state.pool(), organization_id).await? {
        return Err(ApiError::not_found());
    }

    let mut tx = state.db().write(ctx).await?;
    let ligne = quotas::relever(
        &mut tx,
        organization_id,
        payload.max_bytes,
        payload.max_files,
        payload.note.as_deref(),
    )
    .await?;
    tx.commit().await?;

    nommer(state, vec![ligne])
        .await?
        .pop()
        .ok_or_else(|| ApiError::internal("quota écrit mais introuvable à la relecture"))
}

// -----------------------------------------------------------------------------
// La suppression d'un objet
// -----------------------------------------------------------------------------

/// **Refusée si l'objet est encore rattaché**, en disant combien d'entités
/// l'utilisent (FR-105, écart n° 128).
///
/// Le même fichier déposé par deux organisations ne donne qu'une ligne : sans ce
/// refus, la première ferait disparaître l'image de la seconde, sans que rien ne
/// l'annonce.
pub async fn supprimer(
    state: &MediaState,
    ctx: &RequestContext,
    acteur: Uuid,
    asset_id: Uuid,
) -> Result<PurgeProgrammee> {
    let Some(objet) = assets::pour_suppression(state.pool(), asset_id).await? else {
        return Err(ApiError::not_found());
    };

    exiger_la_propriete(state, acteur, objet).await?;

    let usages = attachments::compter_pour_objet(state.pool(), asset_id).await?;
    if usages > 0 {
        return Err(ApiError::with_message(
            ErrorCode::MediaAssetInUse,
            format!(
                "Ce fichier est encore utilisé par {usages} fiche(s) ; il ne peut pas être supprimé."
            ),
        )
        .detail(format!("{usages} rattachement(s) visent l'objet {asset_id}")));
    }

    let mut tx = state.db().write(ctx).await?;
    let instant = assets::programmer_la_purge(&mut tx, asset_id).await?;
    tx.commit().await?;

    Ok(PurgeProgrammee {
        scheduled_purge_at: instant,
    })
}

// -----------------------------------------------------------------------------
// Les gardes
// -----------------------------------------------------------------------------

/// La garde des trois routes de back-office.
///
/// **Le périmètre vide se refuse explicitement**, avant la permission : un
/// compte sans aucun périmètre d'administration doit lire un refus, jamais une
/// liste vide (principe V, règle métier n° 8).
async fn exiger_le_back_office(state: &MediaState, acteur: Uuid) -> Result<()> {
    require_perimeter(state.pool(), acteur)
        .await
        .map_err(|e| e.detail("périmètre d'administration vide"))?;
    require_permission(state.pool(), acteur, PERMISSION_ADMIN, Scope::Global).await
}

/// Qui peut supprimer un objet : celui à qui il appartient.
///
/// C'est le décalque des deux gardes de `domain/guards.rs` — référent de
/// l'organisation propriétaire, ou la personne elle-même —, appliqué à l'objet
/// plutôt qu'à ce qu'il illustre. Le back-office global passe aussi : c'est lui
/// qui vide le disque à partir de la liste des orphelins.
async fn exiger_la_propriete(
    state: &MediaState,
    acteur: Uuid,
    objet: assets::ObjetASupprimer,
) -> Result<()> {
    if objet.owner_person_id == Some(acteur) {
        return Ok(());
    }
    if has_permission(state.pool(), acteur, PERMISSION_ADMIN, Scope::Global).await? {
        return Ok(());
    }
    if let Some(organisation) = objet.owner_organization_id {
        let adhesion = cross::adhesion(state.pool(), acteur, organisation).await?;
        let referent = adhesion.is_some_and(|a| a.active && a.referent);
        let admin = has_permission(
            state.pool(),
            acteur,
            PERMISSION_ADMIN,
            Scope::Organization(organisation),
        )
        .await?;
        if referent || admin {
            return Ok(());
        }
    }

    Err(ApiError::forbidden().detail(format!("l'objet {} n'appartient pas à l'acteur", acteur)))
}

/// Rattache à chaque ligne de quota la dénomination de son organisation, et la
/// part consommée par laquelle le tableau se trie.
async fn nommer(state: &MediaState, lignes: Vec<quotas::LigneDeQuota>) -> Result<Vec<QuotaRow>> {
    if lignes.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<Uuid> = lignes.iter().map(|l| l.organization_id).collect();
    let noms = cross::noms_dorganisations(state.pool(), &ids).await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            let nom = noms
                .iter()
                .find(|(id, _)| *id == l.organization_id)
                .map(|(_, nom)| nom.clone())
                .unwrap_or_default();
            QuotaRow {
                organization_id: l.organization_id,
                organization_name: nom,
                max_bytes: l.max_bytes,
                used_bytes: l.used_bytes,
                max_files: l.max_files,
                used_files: l.used_files,
                used_ratio: if l.max_bytes > 0 {
                    l.used_bytes as f64 / l.max_bytes as f64
                } else {
                    0.0
                },
                note: l.note,
            }
        })
        .collect())
}
