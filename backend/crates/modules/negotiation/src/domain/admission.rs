//! Le mode d'admission, et ce que chaque valeur produit.
//!
//! **Il se relit à chaque tentative, sans cache** (recherche R4). C'est la
//! condition du critère de sortie SC-002 : l'administrateur bascule, et la
//! personne suivante le voit — sans mise en ligne ni redémarrage. Un cache de
//! quelques secondes ferait mentir ce critère pour économiser une lecture
//! indexée par clé primaire, sur un geste rare.

use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};

pub const CLE_MODE: &str = "negotiation.admission_mode";
pub const CLE_ESSAIS: &str = "negotiation.invitation_attempts";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionMode {
    /// Un code juste ouvre aussitôt.
    Code,
    /// Le code ne suffit plus : il ouvre une demande, qu'un administrateur
    /// tranche.
    Approval,
    CodeAndApproval,
}

impl AdmissionMode {
    /// Ce que le semis écrit, et le repli d'une base dont il n'aurait pas été
    /// rejoué.
    pub const DEFAUT: Self = Self::Code;

    pub fn as_db(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Approval => "approval",
            Self::CodeAndApproval => "code_and_approval",
        }
    }

    pub fn parse(valeur: &str) -> Option<Self> {
        match valeur {
            "code" => Some(Self::Code),
            "approval" => Some(Self::Approval),
            "code_and_approval" => Some(Self::CodeAndApproval),
            _ => None,
        }
    }

    /// Un mode que l'API ne reconnaît pas **ne se replie pas silencieusement**.
    ///
    /// L'absence de la clé est un état connu — une base dont le semis n'a pas
    /// été rejoué — et vaut le défaut. Une valeur *présente* et illisible ne
    /// peut venir que d'une écriture à la main ou d'une divergence entre le
    /// code et le modèle : replier sur « code » ouvrirait alors l'entrée
    /// immédiate à un administrateur qui croyait avoir exigé son approbation.
    pub fn lire(valeur: Option<String>) -> Result<Self> {
        match valeur {
            None => Ok(Self::DEFAUT),
            Some(brut) => Self::parse(&brut).ok_or_else(|| {
                ApiError::internal(format!(
                    "mode d'admission « {brut} » inconnu dans platform.settings.{CLE_MODE}"
                ))
            }),
        }
    }

    /// Un code reconnu suffit-il à ouvrir ?
    ///
    /// **`Approval` répond non comme `CodeAndApproval`**, et le code reconnu
    /// ouvre alors une demande qui le porte. L'application ne propose pas la
    /// saisie dans ce mode (FR-022), mais la route reste montée : une personne
    /// qui l'atteint quand même — écran gardé en mémoire, bascule du mode entre
    /// deux gestes — doit repartir avec une suite, pas avec une impasse
    /// (FR-015).
    pub fn ouvre_aussitot(self) -> bool {
        self == Self::Code
    }
}

/// La limite d'essais, telle que `platform.settings` la porte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimiteDEssais {
    pub max: i32,
    pub fenetre_minutes: i32,
    pub verrou_minutes: i32,
}

impl Default for LimiteDEssais {
    /// Les valeurs du semis, mot pour mot.
    fn default() -> Self {
        Self {
            max: 5,
            fenetre_minutes: 15,
            verrou_minutes: 15,
        }
    }
}

// ---------------------------------------------------------------------------
// Le réglage, tel que le back-office le lit et l'écrit
// ---------------------------------------------------------------------------

/// Les trois valeurs, dans l'ordre où l'écran les propose.
pub const MODES: [AdmissionMode; 3] = [
    AdmissionMode::Code,
    AdmissionMode::Approval,
    AdmissionMode::CodeAndApproval,
];

/// Ce qu'un mode produit **pour la personne qui entre**, en faits et non en
/// phrases : l'écran du back-office compose son texte à partir de là, par ses
/// fichiers de traduction, comme tout écran du site (FR-045).
///
/// Rendre ici une phrase française donnerait deux catalogues pour un même
/// écran, et le second se périmerait au premier changement de vocabulaire.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct AdmissionModeOption {
    pub mode: &'static str,
    /// La saisie d'un code est-elle proposée dans ce mode ? Fausse en
    /// « approbation seule » : l'application fait alors disparaître le champ
    /// du parcours (FR-022).
    pub offers_code: bool,
    /// Un code juste ouvre-t-il aussitôt ?
    pub code_opens: bool,
    /// Un administrateur doit-il trancher ?
    pub needs_approval: bool,
}

/// `AdmissionSettings` — ce que rend `GET /admin/negotiation/admission`.
#[derive(Debug, Clone, Serialize)]
pub struct AdmissionSettings {
    pub mode: &'static str,
    pub options: Vec<AdmissionModeOption>,
}

/// `UpdateAdmissionModePayload` — la bascule.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAdmissionModePayload {
    pub mode: String,
}

impl AdmissionMode {
    /// Le champ de saisie d'un code est-il proposé ?
    pub fn offre_la_saisie(self) -> bool {
        self != Self::Approval
    }

    pub fn option(self) -> AdmissionModeOption {
        AdmissionModeOption {
            mode: self.as_db(),
            offers_code: self.offre_la_saisie(),
            code_opens: self.ouvre_aussitot(),
            needs_approval: !self.ouvre_aussitot(),
        }
    }
}

impl AdmissionSettings {
    pub fn composer(courant: AdmissionMode) -> Self {
        Self {
            mode: courant.as_db(),
            options: MODES.iter().map(|m| m.option()).collect(),
        }
    }
}
