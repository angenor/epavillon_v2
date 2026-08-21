//! L'espace organisation — **une composition qui lui est propre, et où rien du
//! comité n'entre**.
//!
//! # Pourquoi ce n'est pas la vue de pilotage
//!
//! `programme.v_proposal_dashboard` répond au **comité** : notes, rang, revues
//! manquantes, membres du comité nommés. La servir au déposant en masquant
//! quelques champs serait le patron qui a produit, en v1, des données internes
//! visibles dans une réponse JSON que l'écran n'affichait pas.
//!
//! **FR-076 et FR-077 l'exigent** : composition propre au soumissionnaire, sans
//! note, sans note pondérée, sans rang, sans nom de membre du comité, sans
//! inscrit nommé. Le test balaie la charge utile **entière** — pas les champs
//! qu'on soupçonne.
//!
//! # L'accès est borné par l'ADHÉSION ACTIVE, jamais par le périmètre
//!
//! Une organisation n'administre rien. Le périmètre d'administration n'a aucun
//! sens ici, et un administrateur de la COP31 n'entre pas dans l'espace d'une
//! organisation dont il n'est pas membre — il a la fiche du comité pour cela.
//!
//! # Deux blocs que ce jalon ne peut pas remplir, et qui ne mentent pas
//!
//! **Les séances programmées** (`sessions`) appartiennent à `075_programme_
//! sessions.sql`, livré par B5 : la liste part **vide**, jamais absente — le
//! champ existe au contrat, et le supprimer ferait échouer l'écran. **Les
//! rappels** appartiennent à B6, pour la même raison. Ce sont des faits de
//! calendrier, pas des oublis (écart n° 108).

use kernel::error::{ApiError, Result};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::{EventId, ProposalId};
use crate::repo::comments::{Cote, Message};
use crate::repo::cross::{
    FicheAppel, FicheEdition, FicheOrganisationComplete, LigneDAdhesion, PersonneAffichee,
};
use crate::repo::proposals::Fiche;
use crate::repo::transitions::LigneDeJournal;
use crate::repo::{comments, cross, proposals, transitions};
use crate::state::ProgrammeState;

/// Un dossier et son avancement — `ProposalTracking`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SuiviDeDossier {
    /// **Sans note ni élimination** : voir `sans_le_comite`.
    pub proposal: Fiche,
    pub edition: FicheEdition,
    /// Le journal **réel**, celui dont la frise se compose. Le graphe des états
    /// vit en base : le réimplémenter dans un composant, c'est se garantir de
    /// diverger au premier changement de règle.
    pub transitions: Vec<LigneDeJournal>,
    /// Demandes de correction **encore ouvertes** — c'est LE nombre que l'écran
    /// crie.
    pub open_change_requests: i64,
    /// **Vide jusqu'à B5.** Voir l'en-tête du fichier.
    pub sessions: Vec<serde_json::Value>,
}

/// Le détail d'un dossier côté organisation — `ProposalFile`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DossierDuDeposant {
    pub tracking: SuiviDeDossier,
    /// **Le fil partagé, filtré à la source.** Les délibérations du comité n'y
    /// sont jamais, et les notes personnelles encore moins.
    pub comments: Vec<Message>,
    pub participants: Vec<PersonneAffichee>,
    pub history: Vec<cross::EntreeDHistorique>,
}

/// Une ligne du bloc « ce qui attend une action de ma part » —
/// `WorkspaceAction`.
///
/// **Le critère d'entrée est strict** : chacune est une chose que
/// l'organisation **seule** peut débloquer. Ce que le comité doit faire n'y
/// figure pas — une liste où figure ce qu'on ne peut pas traiter cesse d'être
/// lue.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ActionEnAttente {
    /// Un **code**, pas une phrase : le libellé est un texte d'interface.
    pub kind: String,
    pub proposal_id: Option<Uuid>,
    pub reference_code: Option<String>,
    pub subject: String,
    pub count: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_at: Option<OffsetDateTime>,
    pub target: String,
}

/// Un membre, son adhésion et la personne derrière — `MemberEntry`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Membre {
    pub membership: LigneDAdhesion,
    pub person: Option<PersonneAffichee>,
    /// **L'organisation a invité cette personne et attend sa réponse.**
    /// Une adhésion en attente a deux origines opposées, et les confondre
    /// ferait approuver une adhésion que l'intéressé n'a jamais acceptée.
    pub is_invitation: bool,
}

/// `WorkspaceOverview` — tout ce que la page d'accueil de l'espace affiche.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EspaceOrganisation {
    pub organization: FicheOrganisationComplete,
    /// L'adhésion de la personne connectée : c'est elle qui ouvre les actions
    /// de référent.
    pub membership: LigneDAdhesion,
    pub proposals: Vec<SuiviDeDossier>,
    pub members: Vec<Membre>,
    pub actions: Vec<ActionEnAttente>,
    /// **Ce n'est pas un ornement** : l'état vide — aucune proposition — met
    /// l'appel en cours en avant, et c'est la seule chose utile à montrer à qui
    /// n'a rien déposé.
    pub open_call: Option<FicheAppel>,
    pub call_edition: Option<FicheEdition>,
}

/// L'espace d'une organisation, **après contrôle de l'adhésion active**.
pub async fn espace(
    state: &ProgrammeState,
    lecteur: Uuid,
    organisation: Uuid,
) -> Result<EspaceOrganisation> {
    let adhesion = exiger_ladhesion_active(state, lecteur, organisation).await?;

    let organization = cross::fiche_organisation_complete(state.pool(), organisation)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let dossiers = proposals::de_lorganisation(state.pool(), organisation, None).await?;
    let mut proposals_suivis = Vec::with_capacity(dossiers.len());
    for dossier in dossiers {
        proposals_suivis.push(suivre(state, dossier).await?);
    }

    let membres = cross::adhesions_de_lorganisation(state.pool(), organisation).await?;
    let ids: Vec<Uuid> = membres.iter().map(|m| m.person_id).collect();
    let personnes = cross::personnes_affichees(state.pool(), &ids).await?;

    let (open_call, call_edition) = match cross::appel_ouvert_de_la_plateforme(state.pool()).await?
    {
        Some((call_id, event_id)) => (
            cross::fiche_appel(state.pool(), call_id).await?,
            cross::fiche_edition(state.pool(), EventId(event_id)).await?,
        ),
        None => (None, None),
    };

    let actions = actions_en_attente(&proposals_suivis, &membres, open_call.as_ref());

    Ok(EspaceOrganisation {
        organization,
        membership: adhesion,
        proposals: proposals_suivis,
        members: membres
            .into_iter()
            .map(|membership| Membre {
                person: personnes
                    .iter()
                    .find(|p| p.id == membership.person_id)
                    .cloned(),
                is_invitation: membership.invited_at.is_some(),
                membership,
            })
            .collect(),
        actions,
        open_call,
        call_edition,
    })
}

/// Le dossier d'un déposant — `ProposalFile`.
pub async fn dossier(
    state: &ProgrammeState,
    lecteur: Uuid,
    id: ProposalId,
) -> Result<DossierDuDeposant> {
    let etat = proposals::etat(state.pool(), id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    exiger_ladhesion_active(state, lecteur, etat.organization_id).await?;

    let fiche = proposals::fiche(state.pool(), id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let tracking = suivre(state, fiche).await?;

    // **Filtré à la source, côté organisation** : ce qui n'est pas envoyé ne
    // peut pas fuiter, et c'est le MÊME filtre que celui du comité — l'écrire
    // deux fois serait écrire deux filtres, et le second finirait par diverger.
    let fil = comments::fil(state.pool(), id, lecteur, Cote::Organisation).await?;

    Ok(DossierDuDeposant {
        tracking,
        participants: auteurs_nommables(state, &fil, etat.organization_id).await?,
        history: cross::historique_du_dossier(state.pool(), id).await?,
        comments: fil,
    })
}

/// **🔴 Les auteurs que le déposant a le droit de VOIR NOMMÉS** (écart n° 109).
///
/// Le contrat du front porte `participants` « pour ne pas résoudre les noms un
/// par un », et FR-077 interdit qu'un **nom de membre du comité** atteigne le
/// déposant. Les deux ne peuvent pas être vrais ensemble dès qu'un membre du
/// comité écrit au déposant, ce qui est le cas ordinaire d'une demande de
/// correction.
///
/// **FR-077 l'emporte, et le filtrage est à la source** : seuls les auteurs
/// **membres de l'organisation porteuse** sont nommés. Un message du comité
/// garde son identifiant d'auteur — l'écran l'affiche sous un libellé neutre,
/// « l'équipe de l'IFDD », et aucune route de cet espace ne permet de résoudre
/// cet identifiant en nom.
///
/// Relâcher la règle est une ligne à retirer ; l'inverse ne l'est pas, et une
/// fuite de nom ne se reprend pas.
async fn auteurs_nommables(
    state: &ProgrammeState,
    fil: &[Message],
    organisation: Uuid,
) -> Result<Vec<PersonneAffichee>> {
    let membres: std::collections::HashSet<Uuid> =
        cross::adhesions_de_lorganisation(state.pool(), organisation)
            .await?
            .into_iter()
            .filter(|a| a.status == "active")
            .map(|a| a.person_id)
            .collect();

    let mut vus = std::collections::HashSet::new();
    let auteurs: Vec<Uuid> = fil
        .iter()
        .map(|m| m.author_id)
        .filter(|id| membres.contains(id) && vus.insert(*id))
        .collect();

    cross::personnes_affichees(state.pool(), &auteurs).await
}

/// Les éditions sur lesquelles cette organisation a déposé.
pub async fn editions(
    state: &ProgrammeState,
    lecteur: Uuid,
    organisation: Uuid,
) -> Result<Vec<FicheEdition>> {
    exiger_ladhesion_active(state, lecteur, organisation).await?;

    let ids = cross::editions_de_lorganisation(state.pool(), organisation).await?;
    let mut fiches = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(fiche) = cross::fiche_edition(state.pool(), EventId(id)).await? {
            fiches.push(fiche);
        }
    }

    // De la plus récente à la plus ancienne : c'est le dossier en cours qu'on
    // cherche en arrivant, pas celui de la COP28.
    fiches.sort_by_key(|e| std::cmp::Reverse(e.starts_at));
    Ok(fiches)
}

// -----------------------------------------------------------------------------
// Les gardes et la composition
// -----------------------------------------------------------------------------

/// **L'adhésion active, et rien d'autre.**
///
/// Le refus est celui d'une ressource inexistante : une organisation dont on
/// n'est pas membre ne doit pas se distinguer d'une organisation qui n'existe
/// pas (principe IX).
async fn exiger_ladhesion_active(
    state: &ProgrammeState,
    lecteur: Uuid,
    organisation: Uuid,
) -> Result<LigneDAdhesion> {
    let adhesion = cross::adhesions_de_lorganisation(state.pool(), organisation)
        .await?
        .into_iter()
        .find(|a| a.person_id == lecteur);

    match adhesion {
        Some(a) if a.status == "active" => Ok(a),
        _ => Err(ApiError::not_found()),
    }
}

/// Composer le suivi d'un dossier — **et lui retirer ce qui appartient au
/// comité**.
async fn suivre(state: &ProgrammeState, dossier: Fiche) -> Result<SuiviDeDossier> {
    let id = ProposalId(dossier.id);
    let edition = cross::fiche_edition(state.pool(), EventId(dossier.event_id))
        .await?
        .ok_or_else(ApiError::not_found)?;

    let ouvertes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.proposal_comments
            WHERE proposal_id = $1 AND is_change_request
              AND resolved_at IS NULL AND deleted_at IS NULL"#,
        dossier.id
    )
    .fetch_one(state.pool())
    .await?;

    Ok(SuiviDeDossier {
        proposal: sans_le_comite(dossier),
        edition,
        transitions: transitions::journal(state.pool(), id).await?,
        open_change_requests: ouvertes,
        sessions: Vec::new(),
    })
}

/// **Ce que le déposant ne voit pas** (FR-077, écart n° 104).
///
/// Les deux notes partent nulles et l'élimination part fausse — « rien à
/// dire », l'état exact d'un dossier que personne n'a noté. `review_count`
/// reste : un nombre de revues déposées n'est ni une note ni un rang, et
/// l'organisation suit l'avancement de l'instruction. Le rang n'est pas
/// concerné : il vit sur la vue de pilotage, que cette composition n'ouvre
/// jamais.
fn sans_le_comite(dossier: Fiche) -> Fiche {
    Fiche {
        average_score: None,
        weighted_score: None,
        is_knocked_out: false,
        ..dossier
    }
}

/// Ce qui attend une action **de l'organisation**.
///
/// Quatre natures sur cinq sont calculables aujourd'hui. La cinquième —
/// « compte rendu de séance manquant » — attend B5 : sans séances, il n'y a
/// aucun compte rendu à réclamer.
fn actions_en_attente(
    dossiers: &[SuiviDeDossier],
    membres: &[LigneDAdhesion],
    appel_ouvert: Option<&FicheAppel>,
) -> Vec<ActionEnAttente> {
    let mut actions = Vec::new();

    for suivi in dossiers {
        let titre = suivi
            .proposal
            .title
            .get("fr")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_owned();

        if suivi.open_change_requests > 0 {
            actions.push(ActionEnAttente {
                kind: "changes_requested".to_owned(),
                proposal_id: Some(suivi.proposal.id),
                reference_code: Some(suivi.proposal.reference_code.clone()),
                subject: titre.clone(),
                count: suivi.open_change_requests,
                due_at: None,
                target: format!("/espace/dossiers/{}", suivi.proposal.id),
            });
        }

        // Un brouillon n'attend une action que **tant qu'il peut encore
        // partir** : le rappeler après l'échéance, c'est demander une chose
        // impossible.
        if suivi.proposal.status == "draft" {
            let echeance = appel_ouvert
                .filter(|c| Some(c.id) == suivi.proposal.call_id)
                .map(|c| c.extended_until.unwrap_or(c.closes_at));

            if echeance.is_some() {
                actions.push(ActionEnAttente {
                    kind: "draft_before_deadline".to_owned(),
                    proposal_id: Some(suivi.proposal.id),
                    reference_code: Some(suivi.proposal.reference_code.clone()),
                    subject: titre,
                    count: 1,
                    due_at: echeance,
                    target: format!("/proposer?dossier={}", suivi.proposal.id),
                });
            }
        }
    }

    // **Une DEMANDE, pas une invitation** : une invitation émise par
    // l'organisation attend la personne, et l'approuver ferait entrer
    // quelqu'un qui n'a rien répondu.
    let demandes = membres
        .iter()
        .filter(|m| m.status == "pending" && m.invited_at.is_none())
        .count() as i64;

    if demandes > 0 {
        actions.push(ActionEnAttente {
            kind: "membership_request".to_owned(),
            proposal_id: None,
            reference_code: None,
            subject: String::new(),
            count: demandes,
            due_at: None,
            target: "/espace/membres".to_owned(),
        });
    }

    actions
}
