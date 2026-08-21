//! Les sept facettes, comptées sur les lignes **déjà lues** (R16).
//!
//! # Pourquoi elles ne font pas l'objet d'une requête
//!
//! Le contrat du front l'exige et en donne la raison : « demandées à part,
//! elles seraient mesurées à un autre instant, et le "Retenu (17)" du filtre
//! finirait par ne plus correspondre aux lignes affichées ». C'est aussi ce que
//! B3 a fait pour les facettes de sa liste d'éditions.
//!
//! **Une exception, et une seule** : le décompte des dossiers non lus vient de
//! `programme.unread_proposals_for()`, qui prend le lecteur en paramètre. Ce
//! n'est pas une facette mais une **relation** entre un dossier et une
//! personne — la même ligne est lue par l'un et pas par l'autre.
//!
//! # Ce que porte un libellé, et ce qu'il ne porte pas
//!
//! `label` vient de la **base** quand la valeur y est nommée — thématique,
//! pays, organisation, personne. Il est **nul** pour un code d'énumération —
//! statut, format —, que l'écran traduit lui-même : un statut est un libellé
//! d'interface, pas une donnée qu'un administrateur modifie.

use serde::Serialize;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Une valeur de filtre et son décompte **sur le périmètre**, filtres non
/// appliqués.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProposalFacet {
    pub value: String,
    /// Multilingue quand la base le nomme, déjà résolu quand c'est un nom
    /// propre, nul quand l'écran le traduit.
    pub label: Option<serde_json::Value>,
    pub count: i64,
    /// Couleur de `reference.taxonomy_terms` — **donnée, jamais jeton de
    /// design** : les figer dans la feuille de style est le défaut n° 1 de la
    /// v1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Les sept, dans l'ordre où l'écran les propose.
#[derive(Debug, Clone, Serialize, ToSchema, Default)]
pub struct ProposalFacets {
    pub statuses: Vec<ProposalFacet>,
    pub themes: Vec<ProposalFacet>,
    pub formats: Vec<ProposalFacet>,
    pub countries: Vec<ProposalFacet>,
    pub organizations: Vec<ProposalFacet>,
    pub reviewers: Vec<ProposalFacet>,
    /// Les trois signaux transverses — `unreviewed`, `late`, `unread`. Ce ne
    /// sont **pas des statuts** : un dossier « non évalué » peut être déposé ou
    /// en évaluation, un dossier « en retard » peut porter deux revues sur
    /// trois. Les confondre ferait manquer exactement les dossiers qu'on
    /// cherche.
    pub flags: Vec<ProposalFacet>,
}

/// Les trois signaux transverses, par leur code.
pub const FLAG_UNREVIEWED: &str = "unreviewed";
pub const FLAG_LATE: &str = "late";
pub const FLAG_UNREAD: &str = "unread";

/// L'ordre du **cycle de vie**, celui de `programme.proposal_status`.
///
/// Un statut ne se range pas par popularité : « déposé » précède « en
/// évaluation », qui précède « retenu ». Trier ces sept-là par décompte
/// rendrait la barre de filtres illisible d'un rechargement à l'autre.
pub const ORDRE_DES_STATUTS: [&str; 8] = [
    "draft",
    "submitted",
    "under_review",
    "changes_requested",
    "accepted",
    "rejected",
    "withdrawn",
    "cancelled",
];

/// Même raison, pour `event.participation_mode`.
pub const ORDRE_DES_FORMATS: [&str; 3] = ["in_person", "online", "hybrid"];

/// Ranger des facettes selon une séquence déclarée, **les absentes écartées**.
///
/// Une valeur que le décompte n'a pas rencontrée ne devient pas une facette à
/// zéro ici : la liste des statuts possibles est connue de l'écran, et
/// afficher les huit quand l'édition n'en porte que trois lui donnerait cinq
/// filtres qui ne filtrent rien. `Compteur::declarer` sert au cas inverse —
/// une valeur qu'on veut voir à zéro parce que son absence surprendrait.
pub fn selon(facettes: Vec<ProposalFacet>, ordre: &[&str]) -> Vec<ProposalFacet> {
    let mut restantes = facettes;
    let mut rangees = Vec::with_capacity(restantes.len());

    for valeur in ordre {
        if let Some(position) = restantes.iter().position(|f| f.value == *valeur) {
            rangees.push(restantes.remove(position));
        }
    }

    // Ce que la séquence ne nomme pas suit, dans l'ordre de rencontre : une
    // valeur nouvelle en base ne doit pas disparaître de l'écran parce que ce
    // fichier ne la connaît pas encore.
    rangees.extend(restantes);
    rangees
}

/// Ranger par décompte décroissant. **Le tri est stable** : deux valeurs à
/// égalité gardent leur ordre de première rencontre, sans quoi la barre de
/// filtres se rebattrait d'un appel à l'autre sans que rien n'ait changé.
pub fn par_compte_decroissant(mut facettes: Vec<ProposalFacet>) -> Vec<ProposalFacet> {
    facettes.sort_by_key(|f| std::cmp::Reverse(f.count));
    facettes
}

/// Un compteur qui **retient l'ordre de première apparition** et le libellé vu
/// avec la valeur.
///
/// L'ordre importe : une `HashMap` rendrait les facettes dans un ordre
/// différent à chaque appel, et l'écran afficherait ses filtres rebattus d'un
/// rechargement à l'autre sans que rien n'ait changé.
#[derive(Default)]
pub struct Compteur {
    ordre: Vec<String>,
    comptes: HashMap<String, i64>,
    libelles: HashMap<String, (Option<serde_json::Value>, Option<String>)>,
}

impl Compteur {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compte une occurrence. Le libellé et la couleur sont retenus **à la
    /// première rencontre** : les redire à chaque ligne coûterait une
    /// allocation par dossier pour la même valeur.
    pub fn ajouter(
        &mut self,
        valeur: &str,
        label: Option<serde_json::Value>,
        color: Option<String>,
    ) {
        match self.comptes.get_mut(valeur) {
            Some(compte) => *compte += 1,
            None => {
                self.ordre.push(valeur.to_owned());
                self.comptes.insert(valeur.to_owned(), 1);
                self.libelles.insert(valeur.to_owned(), (label, color));
            }
        }
    }

    /// Compte une occurrence sans libellé — le cas d'un code d'énumération.
    pub fn ajouter_code(&mut self, valeur: &str) {
        self.ajouter(valeur, None, None);
    }

    /// Pose une valeur à zéro. **Une facette absente et une facette à zéro ne
    /// disent pas la même chose** : « Retenu (0) » apprend qu'aucun dossier
    /// n'est retenu, une ligne manquante laisse croire que le filtre n'existe
    /// pas.
    pub fn declarer(&mut self, valeur: &str) {
        if !self.comptes.contains_key(valeur) {
            self.ordre.push(valeur.to_owned());
            self.comptes.insert(valeur.to_owned(), 0);
            self.libelles.insert(valeur.to_owned(), (None, None));
        }
    }

    pub fn rendre(self) -> Vec<ProposalFacet> {
        let Self {
            ordre,
            mut comptes,
            mut libelles,
        } = self;
        ordre
            .into_iter()
            .map(|valeur| {
                let count = comptes.remove(&valeur).unwrap_or(0);
                let (label, color) = libelles.remove(&valeur).unwrap_or((None, None));
                ProposalFacet {
                    value: valeur,
                    label,
                    count,
                    color,
                }
            })
            .collect()
    }
}
