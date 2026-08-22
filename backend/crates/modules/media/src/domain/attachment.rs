//! Ce qu'un rattachement porte, et ce que les écrans en lisent.
//!
//! # `AttachedMedia` est `AttachedImage`, plus ce que l'écran de gestion exige
//!
//! Le contrat du front porte `AttachedImage` : l'objet résolu, prêt à
//! l'affichage. Il lui manque trois choses sans lesquelles l'écran qui **gère**
//! les médias d'une entité ne peut rien faire — l'identifiant du rattachement,
//! sans lequel on ne sait pas quoi détacher ; le rôle, sans lequel on ne sait
//! pas où ranger la ligne ; et l'ordre de tri, sans lequel une galerie ne se
//! réordonne pas.
//!
//! S'y ajoute **l'état de l'objet**. Entre le dépôt et le passage du worker, un
//! fichier est parfaitement valide et pas encore servable : l'écran doit
//! pouvoir dire « en traitement » plutôt que de laisser un trou.
//!
//! **Trois champs ajoutés, aucun renégocié** — le patron déjà employé par
//! `Asset` et par `AttachableRoleRule` dans ce module. L'ajout côté front est
//! inscrit aux obligations de B7.
//!
//! # `alt_text` peut être nul ici, et il ne l'est pas dans `AttachedImage`
//!
//! La différence n'est pas un relâchement : `AttachedImage` décrit ce que
//! `media.attached_image()` rend, et cette fonction ne rend que des objets
//! **servables**, pour lesquels `ck_assets_alt_text_required` garantit le texte.
//! Cette lecture-ci sert aussi les documents — un PDF n'a pas de texte
//! alternatif — et les objets encore en traitement.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Un média rattaché à une entité — `AttachedImage` **+** `attachment_id`,
/// `role`, `sort_order` et `status`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AttachedMedia {
    pub attachment_id: Uuid,
    pub role: String,
    pub sort_order: i16,
    pub asset_id: Uuid,
    /// L'adresse de l'**original**, composée par la base. Elle est là dès le
    /// dépôt, quand `sources` est encore vide.
    pub url: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// La surcharge du rattachement d'abord, le texte de l'objet ensuite —
    /// résolu **en base**, comme `media.attached_image()` le fait.
    pub alt_text: Option<serde_json::Value>,
    pub caption: Option<serde_json::Value>,
    pub credit: Option<String>,
    /// Les déclinaisons prêtes. Objet **vide mais présent** tant que le worker
    /// n'a rien produit.
    pub sources: serde_json::Value,
    /// L'état de l'objet. `ready` est le seul que les lectures publiques
    /// rendent ; les autres se disent, ici, plutôt que de disparaître.
    pub status: String,
}

/// Ce qu'un ajout déclare — `AttachmentPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AttachmentPayload {
    pub owner_schema: String,
    pub owner_table: String,
    pub owner_id: Uuid,
    pub role: String,
    pub asset_id: Uuid,
    /// L'ordre voulu dans un rôle multiple. Absent : à la suite.
    pub sort_order: Option<i16>,
    /// Le texte alternatif **propre à cet usage**. Il prime sur celui de
    /// l'objet et **ne le modifie pas** : un objet dédupliqué sert plusieurs
    /// fiches, et le texte pertinent n'y est pas le même (FR-040).
    pub alt_text_override: Option<serde_json::Value>,
}

/// L'écriture de remplacement, en lot — `AttachmentAssignment[]`.
///
/// # Ce que « remplacement » veut dire, exactement
///
/// **Chaque rôle nommé dans la liste est vidé puis regarni**, dans l'ordre où
/// ses affectations apparaissent. Un rôle **absent** de la liste n'est pas
/// touché.
///
/// C'est ce qui permet aux trois déclinaisons d'une édition de partir en un
/// geste, à une valeur nulle d'en retirer une **sans toucher aux deux autres**,
/// et à une galerie de se réordonner par un simple renvoi de la même liste dans
/// un autre ordre — sans qu'aucune route de réordonnancement existe.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AttachmentBatch {
    pub owner_schema: String,
    pub owner_table: String,
    pub owner_id: Uuid,
    pub assignments: Vec<AttachmentAssignment>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AttachmentAssignment {
    pub role: String,
    /// **Nul = retirer.** Le rôle est vidé, et l'objet stocké demeure.
    pub asset_id: Option<Uuid>,
    pub alt_text_override: Option<serde_json::Value>,
}

/// Ce que le détachement rend — et le champ répond à la question qu'on se pose
/// en lisant la réponse.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct DetachmentResult {
    /// **Toujours vrai.** Détacher ne détruit pas l'objet : il peut servir
    /// ailleurs, et la déduplication fait qu'il sert souvent ailleurs.
    pub asset_kept: bool,
}
