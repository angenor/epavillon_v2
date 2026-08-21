//! Le renvoi après correction — **et pourquoi ce n'est pas un dépôt** (écart
//! n° 38).
//!
//! # La fenêtre de l'appel ne s'applique pas ici
//!
//! `tg_check_proposal_eligibility()` ne vérifie la fenêtre qu'au **premier**
//! dépôt : le modèle le sait déjà, et c'est ce qui rend ce geste possible. La
//! raison est de terrain — le comité demande une correction à huit jours de la
//! clôture, l'organisation répond trois jours après l'échéance, et lui opposer
//! la clôture serait lui reprocher un délai qu'elle n'a pas choisi.
//!
//! **Le plafond, lui, s'applique** : il compte les dossiers en course, et un
//! renvoi en remet un.
//!
//! # Pourquoi deux routes plutôt qu'une déduction sur l'état
//!
//! Déduire le geste de l'état ferait accepter un renvoi par la route de dépôt,
//! et un dossier corrigé franchirait la clôture **sans que personne l'ait
//! décidé**. Le geste est donc porté par le chemin, et chaque route refuse
//! l'état qui n'est pas le sien.
//!
//! # Ce service n'émet rien
//!
//! Le déclencheur d'état émet `programme.proposal.submitted`, comme au premier
//! dépôt. Un second événement enverrait deux avis à l'organisation.

use kernel::context::RequestContext;
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::draft::SaveDraftPayload;
use crate::domain::ids::ProposalId;
use crate::service::submit::{self, Geste, ResultatDeDepot};
use crate::state::ProgrammeState;

/// Renvoyer un dossier corrigé — **depuis `changes_requested`, et depuis lui
/// seul**.
pub async fn renvoyer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    payload: SaveDraftPayload,
) -> Result<ResultatDeDepot> {
    submit::envoyer(state, ctx, acteur, dossier, payload, Geste::Renvoi).await
}
