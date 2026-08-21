//! Les formes de la fiche d'évaluation, et **le seul calcul qu'elle porte**.
//!
//! Les onze lectures vivent dans `repo/`, leur assemblage dans
//! `service/desk.rs`. Ce fichier ne porte que ce qui se décide **sans base** :
//! la forme de la réponse, et l'état d'avancement d'un membre du comité.
//!
//! # Pourquoi l'état d'avancement est calculé une fois, ici
//!
//! Trois endroits du front l'affichaient — l'en-tête en avancement, le panneau
//! en liste, la barre de retard en alerte — et trois calculs séparés
//! divergeaient sur le cas limite qui compte : **une revue commencée n'est pas
//! une revue rendue**. Calculé une fois à la source, il ne peut plus diverger.

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::repo::assignments::Affectation;
use crate::repo::comments::Message;
use crate::repo::cross::{
    AnteriorAffiche, Critere, EntreeDHistorique, FicheAppel, FicheEdition, OrganisationAffichee,
    PersonneAffichee as Personne,
};
use crate::repo::documents::PieceDuDossier;
use crate::repo::organizations::LienDOrganisation;
use crate::repo::proposals::Fiche;
use crate::repo::reviews::Revue;
use crate::repo::scores::Note;
use crate::repo::speakers::IntervenantLu;
use crate::repo::transitions::LigneDeJournal;

/// Où en est un membre du comité sur ce dossier — `ReviewProgressState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EtatDAvancement {
    Submitted,
    /// Commencée, **pas rendue**. C'est le cas limite qui faisait diverger les
    /// trois calculs du front.
    Drafted,
    Pending,
    Overdue,
    /// Le déport prime sur tout le reste : quelqu'un qui s'est retiré n'est ni
    /// en retard ni en attente, il ne notera pas.
    Recused,
}

/// L'ordre des tests est la règle : déport, puis dépôt, puis brouillon, puis
/// retard. Le permuter donnerait « en retard » à un membre déporté depuis un
/// mois.
pub fn etat_davancement(
    affectation: &Affectation,
    revue_deposee: Option<OffsetDateTime>,
    revue_existe: bool,
    maintenant: OffsetDateTime,
) -> EtatDAvancement {
    if affectation.recused_at.is_some() {
        return EtatDAvancement::Recused;
    }
    if revue_deposee.is_some() {
        return EtatDAvancement::Submitted;
    }
    if revue_existe {
        return EtatDAvancement::Drafted;
    }
    match affectation.due_at {
        Some(echeance) if echeance < maintenant => EtatDAvancement::Overdue,
        _ => EtatDAvancement::Pending,
    }
}

// -----------------------------------------------------------------------------
// Les formes de la réponse — `ReviewDeskScreen` et ses entrées
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrganisationDuDossier {
    pub link: LienDOrganisation,
    pub organization: Option<OrganisationAffichee>,
    /// **Nul pour une fiche qui n'a jamais rien déposé** — et non un objet à
    /// zéro : « jamais rien déposé » et « zéro accepté sur douze » ne se lisent
    /// pas pareil.
    pub track_record: Option<AnteriorAffiche>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IntervenantDuDossier {
    pub speaker: IntervenantLu,
    pub person: Option<Personne>,
}

/// La revue d'un pair, ses notes et son auteur — `PeerReview`.
///
/// **Elle n'est composée que si le voile est levé** : quand il est baissé, la
/// requête qui la lit n'est pas exécutée, et ce champ reste vide.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RevueDUnPair {
    pub review: Revue,
    pub scores: Vec<Note>,
    pub reviewer: Option<Personne>,
    pub assignment: Option<Affectation>,
}

/// Ma revue, prête à être reprise — `MyReview`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MaRevue {
    pub review: Option<Revue>,
    /// Indexé par critère. **Une entrée absente est une note non posée**, pas
    /// un zéro : zéro sur un critère éliminatoire disqualifie le dossier.
    pub scores: std::collections::BTreeMap<String, f64>,
    pub comments: std::collections::BTreeMap<String, String>,
    /// Nulle si ce dossier ne m'a pas été confié.
    pub assignment: Option<Affectation>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvancementDuComite {
    pub assignment: Affectation,
    pub person: Option<Personne>,
    pub state: EtatDAvancement,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<OffsetDateTime>,
}

/// Ce que ce lecteur a le droit de faire — `ReviewDeskPermissions`.
///
/// **Ce n'est pas un contrôle de sécurité** : chaque écriture le refait, et une
/// action masquée reste refusée sur une URL forgée. Ce qu'il garantit, c'est
/// que la notation et la décision ne se ressemblent pas à l'écran.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct DroitsSurLaFiche {
    pub can_review: bool,
    pub can_decide: bool,
    pub can_assign: bool,
    /// **Décorrélé de la permission** : un membre du comité peut lire un
    /// dossier qu'on ne lui a pas confié, sans le noter (R21).
    pub is_assigned: bool,
    pub is_recused: bool,
}

/// `ReviewDeskScreen` — tout l'écran en une réponse.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FicheDEvaluation {
    pub proposal: Fiche,
    pub edition: FicheEdition,
    pub call: Option<FicheAppel>,

    pub organizations: Vec<OrganisationDuDossier>,
    pub speakers: Vec<IntervenantDuDossier>,
    pub documents: Vec<PieceDuDossier>,
    pub themes: serde_json::Value,
    pub transitions: Vec<LigneDeJournal>,
    pub history: Vec<EntreeDHistorique>,

    pub criteria: Vec<CritereAffiche>,
    pub max_weighted_score: f64,
    pub required_reviews: Option<i16>,
    pub blind_review: bool,
    /// **Vrai quand l'appel est en aveugle, que je suis affecté et que ma revue
    /// n'est pas déposée.** Un administrateur qui décide sans noter n'est pas
    /// concerné.
    pub blind_veiled: bool,
    /// Revues déposées que le voile me cache. **Compter n'ancre pas ; lire,
    /// si.**
    pub veiled_count: i64,
    pub my_review: MaRevue,
    /// **Vide quand le voile est baissé** — elles ne sont pas lues.
    pub peer_reviews: Vec<RevueDUnPair>,
    pub committee: Vec<AvancementDuComite>,

    pub comments: Vec<Message>,
    pub participants: Vec<Personne>,
    pub permissions: DroitsSurLaFiche,
    pub rank: i64,
    /// **L'état d'AVANT la visite** : l'ouverture pose l'accusé, la réponse dit
    /// ce qu'il en était.
    pub first_visit: bool,
    pub read_count: i64,
    /// **Ajout au contrat du front, assumé** : l'en-tête a besoin des actions
    /// offertes, et une requête de plus à l'affichage les lui donnerait au prix
    /// d'un aller-retour. Ignoré jusqu'au raccordement.
    pub available_transitions: Vec<crate::domain::transitions::AvailableTransition>,
}

/// Un critère de la grille — `ReviewCriterion`. La grille appartient à
/// **l'appel** : poids, note maximale et caractère éliminatoire varient d'un
/// appel à l'autre, et aucune constante d'interface ne les redit.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CritereAffiche {
    pub id: Uuid,
    pub call_id: Uuid,
    pub code: String,
    pub label: serde_json::Value,
    pub description: Option<serde_json::Value>,
    pub max_score: f64,
    pub weight: f64,
    pub is_knockout: bool,
    pub sort_order: i16,
}

impl CritereAffiche {
    pub fn depuis(critere: Critere, call_id: Uuid) -> Self {
        Self {
            id: critere.id,
            call_id,
            code: critere.code,
            label: critere.label,
            description: critere.description,
            max_score: critere.max_score,
            weight: critere.weight,
            is_knockout: critere.is_knockout,
            sort_order: critere.sort_order,
        }
    }
}
