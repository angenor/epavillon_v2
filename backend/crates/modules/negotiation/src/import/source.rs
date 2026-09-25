//! Ce qu'un lecteur de la source officielle rend : la forme pivot, ou la raison
//! d'une lecture manquée. Le comparateur ne connaît que cette forme.

use async_trait::async_trait;
use std::fmt;
use time::PrimitiveDateTime;

#[async_trait]
pub trait SourceOfficielle: Send + Sync {
    async fn lire(&self) -> Result<Vec<SessionLue>, EchecLecture>;
}

/// Une réunion telle que la source la publie, avant tri et rattachement.
///
/// Les heures sont **murales** : l'heure du lieu, correction de la source
/// appliquée. Le fuseau de l'édition les situe au moment d'écrire.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionLue {
    pub cle: String,
    /// Préfixe « CANCELLED » ou « POSTPONED » retiré, espaces resserrés.
    pub titre: String,
    pub annulation: Option<AnnulationSource>,
    pub categories: Vec<String>,
    pub debut: PrimitiveDateTime,
    pub fin: Option<PrimitiveDateTime>,
    pub salle: Option<String>,
    pub acces_ouvert: Option<bool>,
    pub url: String,
    pub point: Option<PointLu>,
}

/// Le point de l'ordre du jour que le titre cite : « CMA 8 (a) », « Global goal
/// on adaptation ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointLu {
    pub code: String,
    pub intitule: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnulationSource {
    Annulee,
    Reportee,
}

/// Une lecture manquée : elle compte, elle n'écrit aucune session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EchecLecture {
    Injoignable(String),
    DelaiDepasse,
    Illisible(String),
    AucuneReunion,
}

impl fmt::Display for EchecLecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Injoignable(cause) => write!(f, "Source injoignable : {cause}."),
            Self::DelaiDepasse => write!(f, "La source n'a pas répondu dans les 15 secondes."),
            Self::Illisible(cause) => write!(f, "Contenu de la source illisible : {cause}."),
            Self::AucuneReunion => write!(
                f,
                "La source n'a rendu aucune réunion de négociation : lecture tenue pour manquée."
            ),
        }
    }
}
