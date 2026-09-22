//! Le mode d'admission : le lire, le basculer.
//!
//! **Il n'y a rien à redéployer.** La valeur vit dans `platform.settings` et se
//! relit à chaque tentative de saisie, sans cache — c'est exactement ce que
//! SC-002 mesure : un administrateur bascule sur « approbation », et la
//! personne suivante se voit proposer la demande.
//!
//! Une demande déjà en attente **survit à la bascule** (FR-029) : rien ici ne
//! touche aux demandes, et la file continue de les servir.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::admission::{AdmissionMode, AdmissionSettings};
use crate::repo::settings;
use crate::state::NegotiationState;

pub async fn lire(state: &NegotiationState) -> Result<AdmissionSettings> {
    let mut conn = state.pool().acquire().await?;
    let courant = settings::mode_dadmission(&mut conn).await?;
    Ok(AdmissionSettings::composer(courant))
}

/// Bascule le mode. Une valeur hors des trois sort en
/// `NEGOTIATION_ADMISSION_MODE_INVALID`, qui **désigne le champ** : l'écran la
/// pose sous le sélecteur, pas en bandeau de panne.
pub async fn ecrire(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    demande: &str,
) -> Result<AdmissionSettings> {
    let mode = AdmissionMode::parse(demande.trim())
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationAdmissionModeInvalid).field("mode"))?;

    let mut tx = state.db().write(ctx).await?;
    settings::ecrire_le_mode(&mut tx, mode, acteur).await?;
    tx.commit().await?;

    // Relu depuis la base plutôt que rendu de mémoire : ce que l'écran affiche
    // après la bascule est ce que la prochaine tentative lira.
    lire(state).await
}
