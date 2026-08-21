//! Le dépôt — `draft → submitted`.
//!
//! # Ce qui se joue ici, dans cet ordre
//!
//! 1. **L'enregistrement d'abord.** Le contrat envoie le brouillon complet avec
//!    la demande de dépôt : il est écrit **avant** toute décision de
//!    recevabilité, et sa transaction est distincte. Si l'appel a fermé entre
//!    le chargement de la page et le clic, l'organisation ne doit pas perdre en
//!    plus ce qu'elle venait de saisir.
//! 2. **La complétude**, que la base ne vérifie pas : un dossier ne part pas au
//!    comité en s'appelant « Dossier sans titre ».
//! 3. **Les bornes d'intervenants de l'appel**, qu'**aucun déclencheur ne
//!    vérifie** (écart n° 27).
//! 4. **Le classement des trois refus de recevabilité, AVANT l'écriture**
//!    (R9) — c'est l'entorse assumée au principe VIII, et elle existe parce que
//!    le contrat attend deux réponses **portant des valeurs** : l'échéance,
//!    le plafond. Le déclencheur ne les rend que dans une phrase française.
//! 5. **La tentative**, enfin. Le déclencheur reste le dernier mot : une course
//!    entre la lecture et l'écriture retombe sur lui, et son refus est reclassé
//!    plutôt que traduit au texte (R8).
//!
//! # Ce que le service n'émet pas
//!
//! Rien. `tg_guard_proposal_status()` émet `programme.proposal.submitted` dans
//! la même transaction. Émettre à son tour enverrait deux avis de dépôt à
//! l'organisation, et le doublon ne se verrait qu'en production.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::pg_error;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::draft::{self, SaveDraftPayload};
use crate::domain::eligibility::{self, Recevabilite};
use crate::domain::ids::ProposalId;
use crate::repo::{cross, proposals, speakers};
use crate::service::draft_write;
use crate::state::ProgrammeState;

/// L'issue d'un dépôt — exactement `SubmitProposalResult`.
///
/// **Les trois refus sortent en 200**, avec leur discriminant et leur valeur :
/// ce sont des réponses que le contrat exprime, pas des erreurs.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResultatDeDepot {
    Submitted {
        proposal_id: Uuid,
        reference_code: String,
        #[serde(with = "time::serde::rfc3339")]
        submitted_at: OffsetDateTime,
        /// Revues indépendantes attendues — **lu sur l'appel**.
        required_reviews: i16,
        /// Annonce des résultats — **lue sur l'appel**.
        results_expected_at: Option<String>,
    },
    CallClosed {
        #[serde(with = "time::serde::rfc3339")]
        deadline: OffsetDateTime,
    },
    QuotaReached {
        max: i16,
    },
    /// Le troisième refus de recevabilité. Il n'a pas de membre d'union dans le
    /// contrat du front — la campagne de la COP31 n'exige pas la vérification —
    /// et il est rendu quand même : un refus muet serait pire qu'un
    /// discriminant que l'écran ne connaît pas encore.
    OrganizationNotVerified,
}

impl From<Recevabilite> for Option<ResultatDeDepot> {
    fn from(refus: Recevabilite) -> Self {
        match refus {
            Recevabilite::Recevable => None,
            Recevabilite::CallClosed { deadline } => Some(ResultatDeDepot::CallClosed { deadline }),
            Recevabilite::QuotaReached { max } => Some(ResultatDeDepot::QuotaReached { max }),
            Recevabilite::OrganizationNotVerified => Some(ResultatDeDepot::OrganizationNotVerified),
        }
    }
}

/// **Deux gestes, deux routes, et la différence n'est pas cosmétique** (écart
/// n° 38).
///
/// La **fenêtre de l'appel** ne s'applique qu'au premier dépôt : un dossier
/// renvoyé après une demande de correction part souvent après l'échéance —
/// c'est le comité qui a demandé la correction, et lui opposer la clôture
/// serait lui reprocher son propre délai. **Le plafond, lui, s'applique aux
/// deux** : il compte les dossiers en course, et un renvoi en remet un.
///
/// Les deux gestes sont donc **distingués par la route**, et non déduits de
/// l'état : déduire ferait accepter un renvoi par la route de dépôt, et un
/// dossier corrigé franchirait la clôture sans que personne l'ait décidé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Geste {
    /// `draft → submitted`. La fenêtre s'applique.
    Depot,
    /// `changes_requested → submitted`. La fenêtre ne s'applique pas.
    Renvoi,
}

impl Geste {
    /// L'état de départ que ce geste attend.
    fn etat_attendu(self) -> &'static str {
        match self {
            Self::Depot => "draft",
            Self::Renvoi => "changes_requested",
        }
    }

    fn refus(self) -> ApiError {
        let message = match self {
            Self::Depot => {
                "Ce dossier n'est plus un brouillon : un dossier corrigé se renvoie, il ne se \
                 dépose pas une seconde fois."
            }
            Self::Renvoi => "Ce dossier n'attend aucune correction : il n'y a rien à renvoyer.",
        };
        ApiError::with_message(ErrorCode::ValidationFailed, message).field("status")
    }
}

/// Déposer un dossier — **depuis un brouillon, et depuis lui seul**.
pub async fn deposer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    payload: SaveDraftPayload,
) -> Result<ResultatDeDepot> {
    envoyer(state, ctx, acteur, dossier, payload, Geste::Depot).await
}

/// Le chemin commun aux deux gestes.
pub(crate) async fn envoyer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    payload: SaveDraftPayload,
    geste: Geste,
) -> Result<ResultatDeDepot> {
    // L'enregistrement porte ses propres gardes — adhésion active, dossier
    // modifiable, bornes de l'appel — et sa propre transaction.
    let payload = SaveDraftPayload {
        proposal_id: Some(dossier.as_uuid()),
        ..payload
    };
    draft_write::enregistrer(state, ctx, acteur, payload).await?;

    let etat = proposals::etat(state.pool(), dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let call_id = etat
        .call_id
        .ok_or_else(|| ApiError::new(ErrorCode::ProposalUnknownReference).field("call_id"))?;
    let regles = cross::regles_de_lappel(state.pool(), call_id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ProposalUnknownReference).field("call_id"))?;

    verifier_la_completude(state, dossier).await?;
    verifier_les_intervenants(state, dossier, &regles).await?;

    // **L'état de départ est vérifié APRÈS l'enregistrement** : si le dossier
    // n'est pas dans l'état qu'attend ce geste, la correction saisie est déjà
    // sauvegardée, et c'est ce qui compte pour qui vient de taper.
    if etat.status != geste.etat_attendu() {
        return Err(geste.refus());
    }

    let premier_depot = geste == Geste::Depot;

    if let Some(refus) =
        classer(state, call_id, etat.organization_id, dossier, premier_depot).await?
    {
        return Ok(refus);
    }

    tenter(
        state,
        ctx,
        dossier,
        etat.organization_id,
        call_id,
        premier_depot,
        &regles,
    )
    .await
}

/// La tentative, et le reclassement d'une course.
async fn tenter(
    state: &ProgrammeState,
    ctx: &RequestContext,
    dossier: ProposalId,
    organisation: Uuid,
    call_id: Uuid,
    premier_depot: bool,
    regles: &cross::ReglesDeLAppel,
) -> Result<ResultatDeDepot> {
    let mut tx = state.db().write(ctx).await?;

    let issue = sqlx::query!(
        r#"UPDATE programme.proposals
              SET status = 'submitted'
            WHERE id = $1 AND deleted_at IS NULL
        RETURNING reference_code, submitted_at"#,
        dossier.as_uuid()
    )
    .fetch_one(&mut *tx)
    .await;

    match issue {
        Ok(ligne) => {
            tx.commit().await?;
            Ok(ResultatDeDepot::Submitted {
                proposal_id: dossier.as_uuid(),
                reference_code: ligne.reference_code,
                submitted_at: ligne
                    .submitted_at
                    .ok_or_else(|| ApiError::internal("le déclencheur n'a pas daté le dépôt"))?,
                required_reviews: regles.required_reviews,
                results_expected_at: regles.results_expected_at.map(|d| d.to_string()),
            })
        }
        Err(erreur) => {
            tx.rollback().await?;

            // Une course : le déclencheur de recevabilité a refusé entre notre
            // lecture et notre écriture. On RECLASSE plutôt que de lire son
            // message — trois phrases françaises, dont deux interpolent des
            // valeurs, changeraient à la première reformulation du SQL (R8).
            if pg_error::sqlstate(&erreur).as_deref() == Some("23001") {
                if let Some(refus) =
                    classer(state, call_id, organisation, dossier, premier_depot).await?
                {
                    return Ok(refus);
                }
                return Err(ApiError::with_message(
                    ErrorCode::Conflict,
                    pg_error::restrict_violation_message(&erreur)
                        .unwrap_or("Le dépôt a été refusé.")
                        .to_owned(),
                ));
            }

            Err(pg_error::translate(&erreur))
        }
    }
}

async fn classer(
    state: &ProgrammeState,
    call_id: Uuid,
    organisation: Uuid,
    dossier: ProposalId,
    premier_depot: bool,
) -> Result<Option<ResultatDeDepot>> {
    let appel = cross::etat_de_lappel(state.pool(), call_id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ProposalUnknownReference).field("call_id"))?;
    let porteuse =
        cross::dossiers_comptes(state.pool(), call_id, organisation, Some(dossier)).await?;

    Ok(eligibility::classer(&appel, porteuse, premier_depot, OffsetDateTime::now_utc()).into())
}

/// **Les trois textes obligatoires doivent être réellement remplis.**
///
/// Un brouillon naît avec des textes provisoires — `platform.i18n_text` refuse
/// un français vide et les colonnes sont `NOT NULL`. Les laisser passer au
/// dépôt enverrait au comité un dossier intitulé « Dossier sans titre ».
///
/// Le refus est un **422 nommant le champ** : le contrat du front n'exprime pas
/// ce cas — son écran valide avant d'envoyer —, il sort donc en erreur HTTP.
async fn verifier_la_completude(state: &ProgrammeState, dossier: ProposalId) -> Result<()> {
    let ligne = sqlx::query!(
        r#"SELECT title, objectives, detailed_presentation
             FROM programme.proposals WHERE id = $1"#,
        dossier.as_uuid()
    )
    .fetch_one(state.pool())
    .await?;

    for (champ, document) in [
        ("title", &ligne.title),
        ("objectives", &ligne.objectives),
        ("detailed_presentation", &ligne.detailed_presentation),
    ] {
        if draft::fr_sans_repli(document).trim().is_empty() {
            return Err(ApiError::with_message(
                ErrorCode::ValidationFailed,
                "Ce champ doit être renseigné avant le dépôt du dossier.",
            )
            .field(champ));
        }
    }

    Ok(())
}

/// **Les bornes d'intervenants de l'appel, qu'aucun déclencheur ne vérifie**
/// (écart n° 27).
///
/// Elles ne s'appliquent qu'au dépôt, jamais à l'enregistrement : un brouillon
/// se construit intervenant par intervenant, et refuser le premier
/// enregistrement parce qu'il n'y en a qu'un rendrait la saisie impossible.
async fn verifier_les_intervenants(
    state: &ProgrammeState,
    dossier: ProposalId,
    regles: &cross::ReglesDeLAppel,
) -> Result<()> {
    let mut conn = state.pool().acquire().await?;
    let compte = speakers::compter(&mut conn, dossier).await?;

    if compte < i64::from(regles.min_speakers) {
        return Err(ApiError::with_message(
            ErrorCode::ValidationFailed,
            format!(
                "Cet appel demande au moins {} intervenant(s).",
                regles.min_speakers
            ),
        )
        .field("speakers"));
    }
    if compte > i64::from(regles.max_speakers) {
        return Err(ApiError::with_message(
            ErrorCode::ValidationFailed,
            format!(
                "Cet appel accepte au plus {} intervenant(s).",
                regles.max_speakers
            ),
        )
        .field("speakers"));
    }

    Ok(())
}
