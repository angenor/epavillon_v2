//! Tenter une transition — **sans jamais rejouer le graphe** (R7, R8).
//!
//! # Ce que le service fait, et ce qu'il laisse faire
//!
//! Il **tente**. Le graphe vit dans `programme.proposal_transitions_allowed`,
//! quatorze lignes, et `tg_guard_proposal_status()` en est l'arbitre : il
//! refuse ce qui n'est pas déclaré, exige le motif quand la règle le dit, date
//! le dépôt et la décision, écrit la ligne de journal — **et émet l'événement
//! de domaine**.
//!
//! Le service ne rejoue le graphe que pour **offrir** les transitions
//! (`repo/transitions::offertes`) et, dans une action groupée, pour **écarter**
//! un dossier avant d'y toucher — parce qu'une sélection hétérogène doit rendre
//! un écart nommé par dossier, pas une erreur globale.
//!
//! # 🔴 CE SERVICE N'ÉMET AUCUN ÉVÉNEMENT, ET C'EST ICI QUE LA TENTATION EXISTE
//!
//! `tg_guard_proposal_status()` appelle déjà `platform.emit_event()` **dans la
//! transaction**, avec le numéro de dossier, l'édition, l'organisation, les
//! deux états et le motif. C'est **l'inverse de B3**, où aucun déclencheur du
//! module n'émettait rien.
//!
//! Émettre à son tour produirait **deux** événements par transition — donc deux
//! avis de dépôt, deux avis de décision, deux notifications —, et **le doublon
//! ne se verrait qu'en production**. `tests/transitions.rs` compte les lignes
//! d'outbox après une transition et exige **une** ligne, pas deux.
//!
//! # Les deux refus du garde se distinguent PAR LE MOMENT, jamais par le texte
//!
//! Le garde lève `restrict_violation` (23001) pour une transition non déclarée
//! et `not_null_violation` (23502) pour un motif manquant. La traduction de la
//! seconde est **sûre** : la transaction n'écrit que `status` et
//! `decision_reason`, deux colonnes nullables — aucune autre violation de
//! non-nullité n'y est possible.
//!
//! Le 23001, lui, sert **aussi** au contrôle de recevabilité. Ce service n'est
//! pas appelé pour un dépôt — `service/submit.rs` s'en charge et classe avant
//! d'écrire (R9) —, donc un 23001 qui remonte ici est nécessairement une
//! transition non déclarée. Son message français est repris **mot pour mot** :
//! le reformuler produirait deux libellés pour un même refus, et le second se
//! périmerait à la première évolution du SQL.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::pg_error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::domain::transitions::{self, ProposalStatus};
use crate::repo::transitions as repo;
use crate::repo::transitions::LigneDeJournal;
use crate::service::perimeter;
use crate::state::ProgrammeState;

/// L'issue d'une tentative. **Les deux refus sortent en 200** : le contrat les
/// exprime comme membres d'union de `DecisionResult`.
#[derive(Debug, Clone)]
pub enum Issue {
    Appliquee(LigneDeJournal),
    /// Le message français du **déclencheur**, repris mot pour mot.
    TransitionInterdite(String),
    MotifExige,
}

/// Tenter une transition sur un dossier.
///
/// **Le motif est écrit tel qu'il arrive, y compris nul.** La colonne
/// `decision_reason` porte le motif de la **dernière** transition et rien de
/// plus : une transition sans motif l'efface, et c'est le comportement du
/// modèle (écart n° 97). Le journal, lui, garde chacun avec son auteur — c'est
/// lui qu'un écran doit lire.
pub async fn tenter(
    state: &ProgrammeState,
    ctx: &RequestContext,
    dossier: ProposalId,
    vers: ProposalStatus,
    motif: Option<&str>,
) -> Result<Issue> {
    let motif = motif.map(str::trim).filter(|m| !m.is_empty());
    let mut tx = state.db().write(ctx).await?;

    let issue = sqlx::query!(
        r#"UPDATE programme.proposals
              SET status = $2::text::programme.proposal_status,
                  decision_reason = $3
            WHERE id = $1 AND deleted_at IS NULL
        RETURNING id"#,
        dossier.as_uuid(),
        vers.as_str(),
        motif
    )
    .fetch_one(&mut *tx)
    .await;

    match issue {
        Ok(_) => {
            // La ligne de journal est écrite par le déclencheur, DANS cette
            // transaction : elle se lit donc avant de valider, sur la même
            // connexion. La lire après validation rendrait la ligne d'une
            // transition concurrente si deux décisions se croisaient.
            let derniere = repo::derniere(&mut *tx, dossier).await?;
            tx.commit().await?;

            derniere.map(Issue::Appliquee).ok_or_else(|| {
                ApiError::internal("le déclencheur n'a pas journalisé la transition")
            })
        }
        Err(erreur) => {
            tx.rollback().await?;
            Ok(traduire(&erreur)?)
        }
    }
}

/// Traduire le refus du garde, **par le code d'erreur et jamais par le texte**.
fn traduire(erreur: &sqlx::Error) -> Result<Issue> {
    match pg_error::sqlstate(erreur).as_deref() {
        Some("23001") => Ok(Issue::TransitionInterdite(
            pg_error::restrict_violation_message(erreur)
                .unwrap_or("Cette transition n'est pas autorisée.")
                .to_owned(),
        )),
        // Sûr parce que la transaction n'écrit que deux colonnes nullables :
        // aucune autre violation de non-nullité n'y est possible.
        Some("23502") => Ok(Issue::MotifExige),
        _ => Err(pg_error::translate(erreur)),
    }
}

// -----------------------------------------------------------------------------
// L'action groupée
// -----------------------------------------------------------------------------

/// Ce que le contrat attend d'une action groupée — `ChangeStatusPayload`.
#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct ChangeStatusPayload {
    pub proposal_ids: Vec<Uuid>,
    pub to_status: ProposalStatus,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Les trois formes d'une action groupée vivent dans le domaine : **elles
/// sont partagées** avec l'affectation groupée, qui rend la même chose.
pub use crate::domain::bulk::{Ecart, RaisonDEcart, ResultatGroupe};

/// Changer l'état d'une sélection.
///
/// # L'autorisation est évaluée DOSSIER PAR DOSSIER
///
/// Une sélection de douze peut **traverser deux éditions**, et le périmètre
/// s'applique à chacune. Vérifier une fois pour le lot accorderait sur l'une ce
/// qui n'est permis que sur l'autre.
///
/// # Et l'écart est nommé pour chacun
///
/// Un dossier qui n'est pas dans le bon état, un motif manquant, un identifiant
/// hors périmètre : chacun ressort avec son numéro de dossier et sa raison.
pub async fn changer_en_groupe(
    state: &ProgrammeState,
    ctx: &RequestContext,
    perimetre: &kernel::auth::Perimeter,
    payload: ChangeStatusPayload,
) -> Result<ResultatGroupe> {
    let mut resultat = ResultatGroupe::default();
    let motif = payload.reason.as_deref();

    for id in payload.proposal_ids {
        let dossier = ProposalId(id);

        let Some(etat) = crate::repo::proposals::etat(state.pool(), dossier).await? else {
            // Sans numéro de dossier à rendre — il n'existe pas —, on rend
            // l'identifiant demandé : c'est ce que l'écran a en main.
            resultat.skipped.push(Ecart {
                proposal_id: id,
                reference_code: String::new(),
                reason: RaisonDEcart::NotFound,
            });
            continue;
        };

        if !perimetre.allows(etat.event_id) {
            // **Même écart qu'un dossier inexistant** : le refus ne dit pas à
            // qui forge une sélection que le dossier existe ailleurs.
            resultat.skipped.push(Ecart {
                proposal_id: id,
                reference_code: String::new(),
                reason: RaisonDEcart::NotFound,
            });
            continue;
        }

        // **Écarter avant de toucher.** Le garde reste l'arbitre, mais une
        // sélection hétérogène doit rendre un écart nommé par dossier : tenter
        // les douze et traduire douze exceptions coûterait douze transactions
        // avortées pour la même réponse.
        let offertes = repo::offertes(state.pool(), dossier, perimetre.person_id).await?;
        match transitions::motif_exige(&offertes, payload.to_status) {
            None => {
                resultat.skipped.push(Ecart {
                    proposal_id: id,
                    reference_code: etat.reference_code,
                    reason: RaisonDEcart::TransitionNotAllowed,
                });
                continue;
            }
            Some(true) if !transitions::motif_fourni(motif) => {
                resultat.skipped.push(Ecart {
                    proposal_id: id,
                    reference_code: etat.reference_code,
                    reason: RaisonDEcart::ReasonRequired,
                });
                continue;
            }
            Some(_) => {}
        }

        match tenter(state, ctx, dossier, payload.to_status, motif).await? {
            Issue::Appliquee(_) => resultat.applied.push(id),
            Issue::TransitionInterdite(_) => resultat.skipped.push(Ecart {
                proposal_id: id,
                reference_code: etat.reference_code,
                reason: RaisonDEcart::TransitionNotAllowed,
            }),
            Issue::MotifExige => resultat.skipped.push(Ecart {
                proposal_id: id,
                reference_code: etat.reference_code,
                reason: RaisonDEcart::ReasonRequired,
            }),
        }
    }

    Ok(resultat)
}

/// Les transitions offertes à ce lecteur, **après contrôle d'accès**.
pub async fn offertes_pour(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
) -> Result<Vec<crate::domain::transitions::AvailableTransition>> {
    perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;
    repo::offertes(state.pool(), dossier, lecteur).await
}

/// Le journal d'un dossier, **après contrôle d'accès**.
pub async fn journal_de(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
) -> Result<Vec<LigneDeJournal>> {
    perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;
    repo::journal(state.pool(), dossier).await
}
