//! Zone 1 du tableau de bord — **ce qui demande une action**.
//!
//! La seule zone qui coûte quelque chose si on ne la lit pas. Elle vient en
//! premier, et elle doit rester lisible **vide** : un back-office où tout va
//! bien ne doit pas ressembler à un écran cassé.

use serde::Serialize;
use time::OffsetDateTime;

/// Les cinq familles, et le critère qui les réunit : chacune se règle dans un
/// écran du back-office, par quelqu'un de l'équipe, aujourd'hui.
///
/// **Ce qui n'y figure pas est aussi délibéré que ce qui y figure.** Un dossier
/// déposé la veille n'est pas une alerte — c'est le fonctionnement normal ; il
/// n'en devient une qu'à l'approche de l'échéance et sans évaluation. Une liste
/// où l'on trouve ce qui n'appelle rien cesse d'être lue, et c'est alors la
/// ligne qui comptait qu'on rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminActionKind {
    ProposalsUnreviewed,
    ReviewsOverdue,
    OrganizationDuplicates,
    ScheduleConflicts,
    ActiveIncidents,
}

/// Gravité **d'affichage** — deux niveaux, pas trois.
///
/// `high` : ce qui a une échéance dépassée ou en cours de l'être, et ce qui est
/// visible du public (un incident actif l'est). `medium` : ce qui attend sans
/// date. Il n'y a délibérément **pas de `low`** — une ligne qui n'appelle rien
/// n'a pas sa place dans ce bloc, elle appartient aux chiffres.
///
/// À ne pas confondre avec la gravité de `v_operational_health`, qui vient de la
/// base avec ses seuils, ni avec celle d'un incident, qui est une donnée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminActionSeverity {
    High,
    Medium,
}

/// Un exemple nommé sous une ligne d'action. **Trois au plus.**
///
/// « 7 revues en retard » ne dit pas par où commencer ; « 7 revues en retard —
/// Lemoine (3), Ben Amor (2) » le dit. Au-delà de trois, la ligne cesse d'être
/// un résumé et il faut ouvrir l'écran concerné : c'est ce que le lien propose.
#[derive(Debug, Clone, Serialize)]
pub struct AdminActionExample {
    pub label: String,
    /// Précision courte : numéro de dossier, décompte, salle. Facultative.
    pub hint: Option<String>,
    /// Destination propre à l'exemple, **relative et non localisée**.
    pub target: Option<String>,
}

/// Une ligne du bloc d'actions — une famille, son décompte, ses exemples, son
/// écran.
///
/// **Une ligne par famille, jamais une par élément.** Quarante dossiers non
/// évalués produiraient quarante lignes, et le bloc censé se lire d'un coup
/// d'œil deviendrait la liste des propositions — qui existe déjà, avec ses
/// filtres. Le décompte et trois exemples suffisent à décider ; le reste est un
/// clic.
///
/// **Une famille sans élément n'émet aucune ligne.**
#[derive(Debug, Clone, Serialize)]
pub struct AdminAction {
    pub kind: AdminActionKind,
    pub severity: AdminActionSeverity,
    pub count: i64,
    /// Échéance qui rend la ligne urgente ; nulle quand il n'y en a pas.
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_at: Option<OffsetDateTime>,
    pub examples: Vec<AdminActionExample>,
    /// L'écran qui règle l'affaire. Chemin relatif, non localisé.
    pub target: String,
}
