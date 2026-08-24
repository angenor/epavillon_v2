//! Les formes du back-office : la liste, ses facettes, la fiche complète.
//!
//! Elles suivent `frontend/app/types/admin-organizations.ts`, qui en est la
//! source unique. Rien n'est inventé ici : chaque valeur est une colonne
//! d'`org.*`, de `analytics.mv_organization_scorecard`, ou un décompte joint.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::{OrganizationDomainId, OrganizationId, OrganizationNameId, PersonId};

/// Une ligne de la liste — la fiche de performance, plus ce qu'il faut pour la
/// **lire**.
///
/// **Quatre colonnes sont relues sur la table vivante** (FR-048) : statut,
/// sceau, score de confiance et pointeur de fusion. Ce sont celles qui bougent
/// au geste de l'opérateur, et la projection n'est rafraîchie que par un travail
/// différé — sans cette relecture, poser un sceau ne changerait rien à l'écran
/// jusqu'au prochain rafraîchissement.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationListRow {
    pub organization_id: OrganizationId,
    pub legal_name: String,
    pub acronym: Option<String>,
    pub slug: String,
    pub statut: String,
    pub organization_type_code: String,
    pub organization_type_label: Option<Value>,
    /// Couleur du terme, quand elle est posée — repère de teinte, jamais un fond
    /// de texte.
    pub organization_type_color: Option<String>,
    pub country_id: Option<Uuid>,
    pub pays_iso3: Option<String>,
    pub pays_nom: Option<Value>,
    pub statut_oif: String,
    pub est_verifiee: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    pub score_confiance: i16,
    pub merged_into_id: Option<OrganizationId>,

    pub membres_actifs: i64,
    pub membres_en_attente: i64,
    pub referents: i64,

    pub propositions_deposees: i64,
    pub propositions_acceptees: i64,
    pub propositions_rejetees: i64,
    pub ratio_acceptation: Option<f64>,

    pub sessions_programmees: i64,
    pub sessions_realisees: i64,

    /// Cette fiche figure-t-elle dans une paire **non arbitrée** ? C'est le
    /// signal qui relie la liste à la file des doublons.
    pub pending_duplicate_count: i64,
    /// Fiches absorbées par celle-ci. Zéro pour la quasi-totalité.
    pub absorbed_count: i64,

    #[serde(with = "time::serde::rfc3339::option")]
    pub derniere_activite: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub inscrite_le: OffsetDateTime,
}

/// Une facette comptée **sur le même jeu de lignes** que la liste.
///
/// Les demander à part ferait diverger « Sénégal (3) » de ce qui s'affiche, au
/// premier filtre ajouté (FR-046).
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationFacet {
    pub value: String,
    pub label: Option<Value>,
    pub count: i64,
}

/// Tout l'écran de la liste, en une réponse.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationListScreen {
    pub rows: Vec<OrganizationListRow>,
    pub countries: Vec<OrganizationFacet>,
    pub types: Vec<OrganizationFacet>,
    /// Paires non arbitrées, toutes fiches confondues : la pastille de la file.
    pub pending_duplicates: i64,
    /// **Le périmètre a-t-il restreint la liste ?** Vrai pour une personne dont
    /// les droits ne valent que sur une ou deux éditions : l'écran le dit,
    /// plutôt que de laisser croire que la plateforme ne compte que ces fiches.
    pub scoped_to_events: bool,
}

// -----------------------------------------------------------------------------
// La fiche
// -----------------------------------------------------------------------------

/// Une dénomination, avec l'auteur qui l'a ajoutée.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationNameRow {
    pub id: OrganizationNameId,
    pub name: String,
    pub kind: String,
    pub locale: Option<String>,
    pub is_confirmed: bool,
    pub created_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// **Dénomination posée par la base** — le nom légal et le sigle, recopiés
    /// par `tg_organizations_sync_names`. Elles ne se retirent pas à la main :
    /// elles suivent la fiche.
    ///
    /// La comparaison porte sur le nom normalisé **et le genre** : une faute
    /// d'orthographe connue peut avoir le même nom normalisé que le nom légal —
    /// « Developpement » sans accent — sans être pour autant posée par la base.
    pub is_derived: bool,
}

/// Un domaine et son état de vérification.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationDomainRow {
    pub id: OrganizationDomainId,
    pub domain: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    pub verification_method: Option<String>,
    pub auto_join: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// **Ce domaine est-il porté par une AUTRE fiche ?** C'est le signal de
    /// doublon le plus fiable du modèle, et il se voit d'abord ici.
    pub shared_with: Vec<OrganizationRef>,
}

/// Une organisation désignée par son identifiant et son nom — ce qu'un renvoi
/// affiche.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationRef {
    pub organization_id: OrganizationId,
    pub legal_name: String,
}

/// Un membre, avec la direction de son attente quand il y en a une.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationMemberRow {
    pub id: Uuid,
    pub person_id: PersonId,
    pub display_name: String,
    pub primary_email: String,
    pub role: String,
    pub status: String,
    pub is_primary: bool,
    pub job_title: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub invited_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub approved_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Une activité portée par l'organisation, dossier ou séance.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationActivityRow {
    pub kind: String,
    pub id: Uuid,
    pub reference_code: Option<String>,
    pub title: Value,
    pub event_id: Uuid,
    pub event_name: Value,
    pub edition_year: i16,
    pub role: String,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub occurred_at: Option<OffsetDateTime>,
}

/// Une ligne d'historique — `platform.entity_history()`.
///
/// Ce n'est **pas une table** : l'historique est un sous-produit du journal
/// d'audit. `actor_label` y est dénormalisé volontairement — il reste lisible
/// après anonymisation, quand `actor_id` ne pointe plus vers personne.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationHistoryEntry {
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    pub actor_id: Option<PersonId>,
    pub actor_label: Option<String>,
    pub action: String,
    pub field: Option<String>,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
}

/// Une fusion inscrite au journal.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationMergeEntry {
    pub id: Uuid,
    pub source_id: OrganizationId,
    pub source_name: String,
    pub target_id: OrganizationId,
    pub target_name: String,
    pub performed_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub performed_at: OffsetDateTime,
    pub rows_reassigned: Value,
    pub reason: Option<String>,
}

/// Un renvoi de fusion, dans un sens ou dans l'autre.
#[derive(Debug, Clone, Serialize)]
pub struct MergedRef {
    pub organization_id: OrganizationId,
    pub legal_name: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub merged_at: Option<OffsetDateTime>,
}

/// **Toute la fiche en une réponse** — huit lectures assemblées.
///
/// `merged_into` n'est pas un détail d'affichage : une fiche absorbée reste
/// consultable, c'est la promesse de `org.resolve_organization()`. Son écran
/// s'ouvre normalement, coiffé du renvoi vers la fiche vivante.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationDetail {
    pub organization_id: OrganizationId,
    pub legal_name: String,
    pub acronym: Option<String>,
    pub slug: String,
    pub status: String,
    pub organization_type_code: String,
    pub organization_type_label: Option<Value>,
    pub country_id: Option<Uuid>,
    pub country_name: Option<Value>,
    pub city: Option<String>,
    pub description: Option<Value>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    pub verified_by_name: Option<String>,
    pub trust_score: i16,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub created_by_name: Option<String>,

    pub merged_into: Option<MergedRef>,
    pub absorbed: Vec<MergedRef>,

    /// **Toujours pleine sur la route.** `Option` n'est ici que la valeur de
    /// départ de la composition : le service la remplace aussitôt, et la lecture
    /// COALESCE ses compteurs à zéro pour une organisation que la projection
    /// matérialisée ne connaît pas encore. Le site la déclare non nulle.
    pub scorecard: Option<Value>,
    pub names: Vec<OrganizationNameRow>,
    pub domains: Vec<OrganizationDomainRow>,
    pub members: Vec<OrganizationMemberRow>,
    pub activities: Vec<OrganizationActivityRow>,
    pub history: Vec<OrganizationHistoryEntry>,
    pub merges: Vec<OrganizationMergeEntry>,
    /// Paires **non arbitrées** où cette fiche apparaît : le lien vers la fusion.
    pub duplicates: Vec<super::duplicates::DuplicatePair>,
}

// -----------------------------------------------------------------------------
// Les trois écritures de la fiche
// -----------------------------------------------------------------------------

/// **Le sceau n'est pas le statut.** `verified_at` dit que l'IFDD a reconnu
/// l'organisation ; `status` dit où elle en est de son cycle de vie. Les
/// mélanger ferait disparaître d'un écran une organisation qu'on voulait
/// seulement ne pas mettre en avant.
#[derive(Debug, Clone, Deserialize)]
pub struct OrganizationVerification {
    #[serde(default)]
    pub organization_id: Option<Uuid>,
    pub verified: bool,
}

/// Vérification manuelle d'un domaine, ou bascule de son rattachement
/// automatique. **`manual` seule est livrée** : les deux autres méthodes
/// appartiennent à un autre jalon.
#[derive(Debug, Clone, Deserialize)]
pub struct DomainVerification {
    #[serde(default)]
    pub organization_id: Option<Uuid>,
    #[serde(default)]
    pub domain_id: Option<Uuid>,
    pub verified: bool,
    #[serde(default)]
    pub auto_join: bool,
}

/// Confirmation d'une dénomination saisie à l'import ou par un utilisateur.
#[derive(Debug, Clone, Deserialize)]
pub struct NameConfirmation {
    #[serde(default)]
    pub organization_id: Option<Uuid>,
    #[serde(default)]
    pub name_id: Option<Uuid>,
    pub is_confirmed: bool,
}

/// Ce que rend une écriture de la fiche : **la fiche entière, recomposée**.
///
/// Vérifier un domaine change le score de confiance, qui change le rang de la
/// fiche dans la liste ; poser le sceau change ce que la file des doublons
/// affiche. Rendre le seul objet modifié laisserait trois panneaux afficher des
/// valeurs fausses jusqu'au prochain rechargement.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OrganizationWriteOutcome {
    Saved {
        detail: Box<OrganizationDetail>,
    },
    NotFound,
    /// Le domaine est déjà détenu, **et le refus nomme la fiche** : sans ce nom,
    /// il est incompréhensible.
    DomainTaken {
        conflict_with: OrganizationRef,
    },
}
