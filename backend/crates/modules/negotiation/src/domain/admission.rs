//! Le mode d'admission, et ce que chaque valeur produit.
//!
//! **Il se relit à chaque tentative, sans cache** (recherche R4). C'est la
//! condition du critère de sortie SC-002 : l'administrateur bascule, et la
//! personne suivante le voit — sans mise en ligne ni redémarrage. Un cache de
//! quelques secondes ferait mentir ce critère pour économiser une lecture
//! indexée par clé primaire, sur un geste rare.

use kernel::error::{ApiError, Result};

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
