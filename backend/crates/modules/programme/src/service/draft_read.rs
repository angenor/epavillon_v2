//! **Rouvrir un dossier tel qu'il a été saisi** — la recomposition (R6, écart
//! n° 39).
//!
//! # Une lecture, pas un `SELECT`
//!
//! Le formulaire travaille sur une structure d'**écran** : des textes en
//! français, une heure murale, des listes à clés. La base range la même chose
//! dans cinq tables — le dossier, ses organisations, ses intervenants, ses
//! thématiques polymorphes, ses pièces. Recomposer, c'est refaire ce chemin en
//! sens inverse, et trois conversions y sont délicates.
//!
//! **Le créneau redevient une heure MURALE dans le fuseau de l'ÉDITION.** Un
//! créneau saisi à 14:30 à Belém se rouvrirait à 11:30 pour qui corrige depuis
//! Dakar — **sans qu'aucune erreur ne soit levée**. La conversion se fait en
//! base, comme l'aller : aucune arithmétique de fuseau n'est écrite en Rust.
//!
//! **Les textes multilingues sont ramenés à leur français**, la seule langue
//! que le formulaire sait rendre — et **les textes provisoires sont effacés**.
//! Un dossier né avant l'étape 2 porte « Dossier sans titre » et « À
//! compléter » : les rendre au formulaire ferait relire au déposant un texte
//! que personne n'a écrit, et qu'il faudrait effacer à la main avant de saisir
//! le sien (écart n° 102).
//!
//! **Un intervenant retrouve son verrouillage d'identité.** Une personne qui
//! possède un compte détient sa propre fiche : ce n'est pas au déposant de la
//! réécrire, et le formulaire doit le savoir **avant** d'afficher un champ
//! modifiable (écart n° 31).
//!
//! # 🔴 UNE SEULE IMPLÉMENTATION, pour deux écrans
//!
//! Le formulaire de dépôt et le formulaire de correction rouvrent le même
//! dossier. Deux recompositions divergeraient au premier champ ajouté — l'une
//! rendrait le nouveau champ, l'autre non —, et la divergence se verrait comme
//! une perte de données au premier enregistrement suivant.

use kernel::error::{ApiError, Result};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::draft::{self, ProposalDraft};
use crate::domain::ids::{EventId, ProposalId};
use crate::repo::{cross, organizations, proposals, speakers};
use crate::service::perimeter;
use crate::state::ProgrammeState;

/// Une co-organisation recomposée — **avec de quoi la nommer**.
///
/// Le formulaire affiche une puce par organisation ; sans le nom, il devrait
/// recharger la fiche de chacune, et l'écran retomberait dans le N+1 que la
/// recomposition existe pour éviter.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CoorganisationRecomposee {
    pub organization_id: Uuid,
    pub role: String,
    pub legal_name: String,
    pub acronym: Option<String>,
    pub country_id: Option<Uuid>,
}

/// Un intervenant recomposé — **et son identité verrouillée ou non**.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IntervenantRecompose {
    pub person_id: Option<Uuid>,
    /// **Le verrouillage d'identité.** Vrai : la personne détient sa fiche, et
    /// le formulaire ferme les trois champs d'identité. Les deux instantanés —
    /// fonction et organisation au moment de l'activité — restent modifiables
    /// dans tous les cas : ils appartiennent au dossier, pas à la personne.
    pub has_account: bool,
    pub civility: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub job_title: String,
    pub organization_name: String,
    pub organization_id: Option<Uuid>,
    pub role: String,
    pub bio: String,
}

/// Le brouillon recomposé — la forme d'écran, enrichie de ce que la lecture
/// seule peut donner.
///
/// Les deux listes portent le MÊME NOM que celles de `ProposalDraft`, et c'est
/// voulu : l'écran ne connaît qu'une clé `speakers`. Ce sont celles de
/// `ProposalDraft` qui ne sortent pas — `skip_serializing`, posé là-bas.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BrouillonRecompose {
    #[serde(flatten)]
    pub draft: ProposalDraft,
    pub co_organizations: Vec<CoorganisationRecomposee>,
    pub speakers: Vec<IntervenantRecompose>,
}

/// `EditableProposal` — ce que l'écran reçoit pour rouvrir un dossier.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DossierRouvert {
    pub proposal_id: Uuid,
    pub reference_code: String,
    pub call_id: Option<Uuid>,
    pub event_id: Uuid,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub saved_at: OffsetDateTime,
    pub draft: BrouillonRecompose,
}

/// Rouvrir un dossier, **après contrôle de l'accès**.
///
/// L'accès passe par les deux voies ordinaires — adhésion active, ou lecture
/// générale dans le périmètre. L'IFDD corrige parfois un intitulé, et lui
/// fermer la porte obligerait à passer par la base.
pub async fn rouvrir(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
) -> Result<DossierRouvert> {
    perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;

    let fiche = proposals::fiche(state.pool(), dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let edition = cross::contexte_edition(state.pool(), EventId(fiche.event_id))
        .await?
        .ok_or_else(ApiError::not_found)?;

    let creneau = creneau_mural(state, dossier, &edition.timezone).await?;
    let themes = cross::themes_du_dossier(state.pool(), dossier).await?;

    Ok(DossierRouvert {
        proposal_id: fiche.id,
        reference_code: fiche.reference_code.clone(),
        call_id: fiche.call_id,
        event_id: fiche.event_id,
        status: fiche.status.clone(),
        saved_at: fiche.updated_at,
        draft: BrouillonRecompose {
            co_organizations: coorganisations(state, dossier).await?,
            speakers: intervenants(state, dossier).await?,
            draft: ProposalDraft {
                organization_id: Some(fiche.organization_id),
                co_organizations: Vec::new(),
                // **Les replis sont effacés** : le formulaire ne doit jamais
                // afficher « Dossier sans titre » (écart n° 102).
                title: draft::fr_sans_repli(&fiche.title),
                summary: fiche.summary.as_ref().map(draft::fr).unwrap_or_default(),
                objectives: draft::fr_sans_repli(&fiche.objectives),
                detailed_presentation: draft::fr_sans_repli(&fiche.detailed_presentation),
                expected_outcomes: fiche
                    .expected_outcomes
                    .as_ref()
                    .map(draft::fr)
                    .unwrap_or_default(),
                target_audiences: fiche.target_audiences.iter().map(draft::fr).collect(),
                theme_codes: themes,
                activity_type_code: fiche.activity_type_code.clone(),
                format: Some(fiche.format.clone()),
                language_codes: fiche.language_codes.clone(),
                country_id: fiche.country_id,
                speakers: Vec::new(),
                preferred_start_at: creneau,
                duration_minutes: fiche.duration_minutes,
                requested_sessions: fiche.requested_sessions,
                scheduling_constraints: fiche.scheduling_constraints.clone().unwrap_or_default(),
            },
        },
    })
}

/// **L'heure murale, convertie EN BASE dans le fuseau de l'édition.**
///
/// `AT TIME ZONE` fait le travail ; l'écrire en Rust demanderait une base de
/// fuseaux dans le processus, et les deux divergeraient au prochain changement
/// d'heure d'été décidé par un pays.
async fn creneau_mural(
    state: &ProgrammeState,
    dossier: ProposalId,
    fuseau: &str,
) -> Result<Option<String>> {
    let mural = sqlx::query_scalar!(
        r#"SELECT to_char(preferred_start_at AT TIME ZONE $2, 'YYYY-MM-DD"T"HH24:MI')
             FROM programme.proposals WHERE id = $1"#,
        dossier.as_uuid(),
        fuseau
    )
    .fetch_one(state.pool())
    .await?;

    Ok(mural)
}

/// Les co-organisations, **porteur exclu** : il est déjà `organization_id`, et
/// le formulaire ne l'affiche pas dans sa liste de partenaires.
async fn coorganisations(
    state: &ProgrammeState,
    dossier: ProposalId,
) -> Result<Vec<CoorganisationRecomposee>> {
    let liens: Vec<_> = organizations::du_dossier(state.pool(), dossier)
        .await?
        .into_iter()
        .filter(|l| l.role != "lead")
        .collect();

    let ids: Vec<Uuid> = liens.iter().map(|l| l.organization_id).collect();
    let fiches = cross::organisations_affichees(state.pool(), &ids).await?;

    Ok(liens
        .into_iter()
        .map(|lien| {
            let fiche = fiches.iter().find(|o| o.id == lien.organization_id);
            CoorganisationRecomposee {
                organization_id: lien.organization_id,
                role: lien.role,
                legal_name: fiche.map(|o| o.legal_name.clone()).unwrap_or_default(),
                acronym: fiche.and_then(|o| o.acronym.clone()),
                country_id: fiche.and_then(|o| o.country_id),
            }
        })
        .collect())
}

/// Les intervenants, **avec leur verrouillage d'identité**.
async fn intervenants(
    state: &ProgrammeState,
    dossier: ProposalId,
) -> Result<Vec<IntervenantRecompose>> {
    let lignes = speakers::du_dossier(state.pool(), dossier).await?;
    let ids: Vec<Uuid> = lignes.iter().map(|l| l.person_id).collect();
    let personnes = cross::fiches_personnes(state.pool(), &ids).await?;

    Ok(lignes
        .into_iter()
        .map(|ligne| {
            let personne = personnes.iter().find(|p| p.id == ligne.person_id);
            IntervenantRecompose {
                person_id: Some(ligne.person_id),
                has_account: personne.is_some_and(|p| p.has_account),
                civility: personne.and_then(|p| p.civility.clone()),
                first_name: personne.map(|p| p.first_name.clone()).unwrap_or_default(),
                last_name: personne.map(|p| p.last_name.clone()).unwrap_or_default(),
                email: personne.map(|p| p.email.clone()).unwrap_or_default(),
                job_title: ligne.job_title_snapshot.unwrap_or_default(),
                organization_name: ligne.organization_snapshot.unwrap_or_default(),
                organization_id: ligne.organization_id,
                role: ligne.role,
                bio: ligne.bio.as_ref().map(draft::fr).unwrap_or_default(),
            }
        })
        .collect())
}
