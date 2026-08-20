//! Le comité de sélection — **qui siège, et rien d'autre**.
//!
//! ## Ce que ce service n'accorde pas
//!
//! `event.call_reviewers` dit la **composition**, pas le droit d'accès. Le
//! commentaire du modèle le dit en toutes lettres : l'autorisation reste portée
//! par `identity.role_assignments` sur la portée de l'édition. Ajouter quelqu'un
//! au comité **n'attribue aucun rôle** — le service se contente de **signaler**
//! que la personne ne détient pas la permission d'évaluer, par
//! `has_review_permission`.
//!
//! Laisser croire l'inverse coûterait cher dans les deux sens : un évaluateur
//! sans droits qui ne peut pas ouvrir les dossiers qu'on lui a confiés, ou un
//! droit accordé en silence par un geste que personne ne perçoit comme une
//! attribution.
//!
//! ## Un seul geste, une transaction
//!
//! Ajouts, retraits et plafonds ensemble. La charge utile est **dédoublonnée par
//! le service**, jamais remontée comme erreur de base : la clé primaire
//! `(call_id, person_id)` ne doit jamais se plaindre.
//!
//! Une personne inconnue est refusée en **nommant** ce qui cloche
//! (`EVENT_UNKNOWN_REFERENCE`) : la clé étrangère refuserait aussi, mais elle
//! refuserait l'enregistrement entier sans dire laquelle des huit lignes est en
//! cause.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use std::collections::HashSet;
use uuid::Uuid;

use crate::domain::ids::{CallId, EventId};
use crate::domain::tabs::{CommitteePayload, CommitteeSaveResult, RemovedWithAssignments};
use crate::repo::{committee, cross};
use crate::state::EventState;

/// Enregistrer la composition d'un comité.
pub async fn enregistrer(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    call_id: CallId,
    payload: CommitteePayload,
) -> Result<CommitteeSaveResult> {
    let sieges = dedoublonner(&payload);

    // **Refuser une personne inconnue en la nommant**, avant d'ouvrir la
    // transaction : rien n'a à être écrit pour découvrir cela.
    let demandees: Vec<Uuid> = sieges.iter().map(|(id, _, _)| *id).collect();
    let existantes: HashSet<Uuid> = cross::personnes_existantes(state.pool(), &demandees)
        .await?
        .into_iter()
        .collect();
    if let Some(inconnue) = demandees.iter().find(|id| !existantes.contains(id)) {
        return Err(ApiError::with_message(
            ErrorCode::EventUnknownReference,
            format!("La personne {inconnue} n'existe pas."),
        )
        .field("person_id"));
    }

    // **Le décompte des dossiers confiés se prend AVANT le retrait** : les
    // affectations survivent au retrait — elles ne sont pas en cascade — mais
    // la lecture, elle, ne rend que les membres présents. Après l'ordre, le nom
    // du retiré ne serait plus lisible dans la composition.
    let avant = cross::comite_resolu(state.pool(), call_id, event_id).await?;
    let retires: Vec<RemovedWithAssignments> = avant
        .iter()
        .filter(|(id, _)| !existantes.contains(id))
        .filter(|(_, p)| p.assigned_count > 0)
        .map(|(_, p)| RemovedWithAssignments {
            full_name: p.full_name.clone(),
            assigned_count: p.assigned_count,
        })
        .collect();

    let mut tx = state.db().write(ctx).await?;
    committee::remplacer(&mut tx, call_id, &sieges)
        .await
        .map_err(|e| kernel::pg_error::translate(&e))?;
    tx.commit().await?;

    // **Aucun événement n'est émis**, et c'est une soustraction délibérée : B4
    // lit `event.call_reviewers` directement, sur sa propre question. Un
    // événement ferait un second chemin pour la même information
    // (`contracts/events.md`).

    let members = super::detail::comite_resolu(state.pool(), call_id, event_id).await?;

    Ok(CommitteeSaveResult {
        ok: true,
        members,
        removed_with_assignments: retires,
    })
}

/// **Dédoublonner par personne**, la dernière ligne l'emportant.
///
/// L'écran peut envoyer deux fois la même personne — une liste recomposée après
/// un ajout, par exemple. La clé primaire refuserait, et ce refus serait un
/// message technique pour une situation que le service sait résoudre seul.
fn dedoublonner(payload: &CommitteePayload) -> Vec<(Uuid, bool, Option<i16>)> {
    let mut vus: Vec<(Uuid, bool, Option<i16>)> = Vec::with_capacity(payload.members.len());

    for siege in &payload.members {
        match vus.iter_mut().find(|(id, _, _)| *id == siege.person_id) {
            Some(existant) => {
                existant.1 = siege.is_lead;
                existant.2 = siege.workload_cap;
            }
            None => vus.push((siege.person_id, siege.is_lead, siege.workload_cap)),
        }
    }

    vus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tabs::CommitteeSeat;

    fn siege(person_id: Uuid, responsable: bool, plafond: Option<i16>) -> CommitteeSeat {
        CommitteeSeat {
            person_id,
            is_lead: responsable,
            workload_cap: plafond,
        }
    }

    /// Deux fois la même personne : **une seule ligne**, la dernière gagnant.
    /// C'est la seule résolution qui ne surprenne personne — l'écran envoie sa
    /// liste dans l'ordre où il l'affiche.
    #[test]
    fn une_personne_repetee_ne_fait_quune_ligne() {
        let personne = Uuid::now_v7();
        let payload = CommitteePayload {
            call_id: None,
            members: vec![
                siege(personne, false, Some(5)),
                siege(personne, true, Some(12)),
            ],
        };

        let sieges = dedoublonner(&payload);
        assert_eq!(sieges.len(), 1);
        assert!(sieges[0].1, "la dernière ligne l'emporte");
        assert_eq!(sieges[0].2, Some(12));
    }

    #[test]
    fn deux_personnes_distinctes_font_deux_lignes() {
        let payload = CommitteePayload {
            call_id: None,
            members: vec![
                siege(Uuid::now_v7(), true, None),
                siege(Uuid::now_v7(), false, Some(8)),
            ],
        };

        assert_eq!(dedoublonner(&payload).len(), 2);
    }

    #[test]
    fn un_comite_vide_est_une_composition_valide() {
        let payload = CommitteePayload {
            call_id: None,
            members: Vec::new(),
        };

        assert!(dedoublonner(&payload).is_empty());
    }
}
