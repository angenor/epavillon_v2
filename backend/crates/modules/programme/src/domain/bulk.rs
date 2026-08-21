//! Ce que rend une action groupée — **partagé par les deux**.
//!
//! Changer l'état d'une sélection et confier une sélection sont deux gestes
//! différents ; ils rendent la **même** forme, parce que l'écran les affiche
//! au même endroit et de la même façon. Déclarer deux types jumeaux ferait
//! diverger le premier ajout de raison.
//!
//! # Pourquoi une action groupée ne rend pas un nombre
//!
//! Une sélection est **hétérogène** : sur douze dossiers, trois sont déjà
//! confiés à cette personne, un s'en est déporté, deux ne sont pas dans le bon
//! état. Répondre « 6 dossiers traités » sans dire ce qu'il est advenu des six
//! autres laisse croire à un succès complet — le défaut classique des actions
//! de masse. **Appliqués + écartés = taille de la sélection**, toujours.
//!
//! # Le refus qui n'en dit pas trop
//!
//! `NotFound` couvre trois causes : le dossier n'existe pas, il est hors
//! périmètre, ou le lecteur n'a pas la permission sur son édition. Les
//! distinguer apprendrait à qui forge une sélection que le dossier existe
//! ailleurs (principe IX).
//!
//! **Un dossier introuvable hors action groupée reste un 404** : seul, c'est
//! une ressource qui n'existe pas ; parmi douze, c'est un écart qu'on montre à
//! côté des onze autres.

use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// `BulkSkipReason` — pourquoi un dossier n'a pas suivi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RaisonDEcart {
    /// Le membre du comité porte déjà ce dossier.
    AlreadyAssigned,
    /// Il s'en est **déporté**. Le lui réattribuer effacerait une déclaration
    /// d'impartialité : `recused_at` n'est pas une suppression.
    Recused,
    TransitionNotAllowed,
    ReasonRequired,
    /// **Introuvable, hors périmètre, ou hors permission — indiscernables.**
    NotFound,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Ecart {
    pub proposal_id: Uuid,
    /// Vide quand le dossier n'a pas été résolu : il n'y a alors aucun numéro
    /// à rendre, et l'identifiant demandé est ce que l'écran a en main.
    pub reference_code: String,
    pub reason: RaisonDEcart,
}

/// `BulkResult` — ce qui a été fait, et **dossier par dossier** ce qui ne l'a
/// pas été.
#[derive(Debug, Clone, Serialize, ToSchema, Default)]
pub struct ResultatGroupe {
    /// Dossiers réellement modifiés.
    pub applied: Vec<Uuid>,
    pub skipped: Vec<Ecart>,
}
