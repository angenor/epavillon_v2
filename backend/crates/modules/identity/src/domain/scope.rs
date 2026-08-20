//! Portées visées par une écriture de rôle, et ce que cette écriture rend.
//!
//! **La portée est le sujet de l'autorisation, pas un détail de la ligne.**
//! `identity.role.assign` sur la COP31 et la même permission globalement sont
//! deux droits différents : c'est la portée qui les distingue, jamais le nom du
//! rôle.
//!
//! **Le couple (type, cible) se valide ici, avant la base — et c'est une
//! entorse assumée au principe VIII.** `ck_role_assignment_scope` dit déjà
//! qu'une portée globale ne vise rien et qu'une portée ciblée exige une cible ;
//! normalement on laisse la base refuser et on traduit. Mais l'autorisation
//! passe *avant* l'écriture, et sans cible on ne sait pas sur quoi la tester —
//! il n'y a rien à interroger. Le refus rendu porte donc **le même code que la
//! base rendrait**, `IDENTITY_ROLE_SCOPE_MISMATCH` sur le champ `scope_id` :
//! l'appelant ne peut pas distinguer les deux chemins.

use kernel::auth::{Scope, ScopeType};
use kernel::error::{ApiError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::admin_users::UserDetail;
use super::login::PersonStatus;
use super::rbac::RoleAssignmentView;

/// La portée visée par une attribution, telle que le client l'a écrite.
pub fn portee_visee(scope_type: ScopeType, scope_id: Option<Uuid>) -> Result<Scope> {
    let incoherent = || ApiError::new(ErrorCode::IdentityRoleScopeMismatch).field("scope_id");

    match (scope_type, scope_id) {
        (ScopeType::Global, None) => Ok(Scope::Global),
        (ScopeType::Global, Some(_)) => Err(incoherent()),
        (_, None) => Err(incoherent()),
        (ScopeType::Organization, Some(id)) => Ok(Scope::Organization(id)),
        (ScopeType::Event, Some(id)) => Ok(Scope::Event(id)),
        (ScopeType::NegotiationSpace, Some(id)) => Ok(Scope::NegotiationSpace(id)),
    }
}

/// Une cible de portée offerte au choix — une édition, une organisation.
#[derive(Debug, Clone, Serialize)]
pub struct ScopeChoice {
    pub scope_type: ScopeType,
    pub scope_id: Uuid,
    pub label: String,
    pub hint: Option<String>,
    /// Hors de ce que l'acteur peut réellement accorder : offerte en lecture
    /// pour qu'il comprenne qu'elle existe, **jamais sélectionnable**. La taire
    /// ferait croire à un bogue à qui cherche une édition qu'il sait présente.
    pub disabled: bool,
}

/// Ce que rend une écriture de rôle. **Les six issues sortent en 200** : ce
/// sont des refus prévus par le contrat du site, pas des erreurs HTTP, et
/// chacune dit quoi corriger — « impossible » ne dit rien.
///
/// `message` ne vaut que pour `scope_not_allowed` : il porte **le texte du
/// trigger, mot pour mot**. Le reformuler ici produirait deux libellés pour un
/// même refus, et le second se périmerait à la première évolution du modèle.
#[derive(Debug, Clone, Serialize)]
pub struct RoleWriteOutcome {
    pub status: &'static str,
    pub assignment: Option<RoleAssignmentView>,
    /// Attributions **en cours** après l'écriture : l'écran s'y recale sans
    /// recharger la fiche.
    pub assignments: Vec<RoleAssignmentView>,
    pub conflict_with: Option<RoleAssignmentView>,
    pub message: Option<String>,
}

impl RoleWriteOutcome {
    fn nouvelle(status: &'static str, assignments: Vec<RoleAssignmentView>) -> Self {
        Self {
            status,
            assignment: None,
            assignments,
            conflict_with: None,
            message: None,
        }
    }

    pub fn granted(assignment: RoleAssignmentView, assignments: Vec<RoleAssignmentView>) -> Self {
        Self {
            assignment: Some(assignment),
            ..Self::nouvelle("granted", assignments)
        }
    }

    pub fn revoked(assignment: RoleAssignmentView, assignments: Vec<RoleAssignmentView>) -> Self {
        Self {
            assignment: Some(assignment),
            ..Self::nouvelle("revoked", assignments)
        }
    }

    /// `ux_role_assignments_active` ne filtre que sur `revoked_at IS NULL` :
    /// une attribution **expirée** compte encore. Le refus nomme donc la ligne
    /// en place, sans quoi l'écran dirait « déjà attribué » devant une liste où
    /// rien n'apparaît comme actif.
    pub fn duplicate(
        conflit: Option<RoleAssignmentView>,
        assignments: Vec<RoleAssignmentView>,
    ) -> Self {
        Self {
            conflict_with: conflit,
            ..Self::nouvelle("duplicate", assignments)
        }
    }

    pub fn scope_not_allowed(message: String, assignments: Vec<RoleAssignmentView>) -> Self {
        Self {
            message: Some(message),
            ..Self::nouvelle("scope_not_allowed", assignments)
        }
    }

    /// L'acteur n'a pas `identity.role.assign` **sur la portée visée**. Distinct
    /// de `scope_not_allowed`, qui dit que le rôle lui-même n'admet pas cette
    /// portée : l'un se corrige en changeant de portée, l'autre en demandant un
    /// droit.
    ///
    /// **Rien n'accompagne le refus.** Les autres issues rendent les
    /// attributions en cours pour que l'écran s'y recale ; celle-ci répond à qui
    /// n'avait pas le droit d'écrire, et lui renvoyer la liste des rôles de la
    /// personne visée ferait de cette route une lecture déguisée.
    pub fn forbidden_scope() -> Self {
        Self::nouvelle("forbidden_scope", Vec::new())
    }

    pub fn not_found() -> Self {
        Self::nouvelle("not_found", Vec::new())
    }
}

/// Les statuts qu'une écriture d'administration peut **poser**.
///
/// `anonymized` n'en fait pas partie, et c'est le type qui le dit : l'effacement
/// RGPD passe par `identity.anonymize_person()`, qui purge l'identité et émet
/// son propre événement. Le poser ici marquerait une fiche comme effacée sans
/// rien effacer — la pire des deux issues. Un client qui l'envoie quand même
/// reçoit un refus de validation, sur le champ `status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignableStatus {
    Active,
    Suspended,
    Blocked,
}

impl AssignableStatus {
    pub fn en_statut(self) -> PersonStatus {
        match self {
            Self::Active => PersonStatus::Active,
            Self::Suspended => PersonStatus::Suspended,
            Self::Blocked => PersonStatus::Blocked,
        }
    }
}

/// Ce que rend un changement de statut. `missing_deadline` traduit
/// `ck_people_suspension_window` : une suspension sans terme est refusée **par
/// la base**, et c'est une décision du modèle — sans date de fin, c'est un
/// blocage qui n'ose pas dire son nom.
#[derive(Debug, Clone, Serialize)]
pub struct PersonWriteOutcome {
    pub status: &'static str,
    pub detail: Option<UserDetail>,
}

impl PersonWriteOutcome {
    pub fn saved(detail: UserDetail) -> Self {
        Self {
            status: "saved",
            detail: Some(detail),
        }
    }

    pub fn missing_deadline(detail: Option<UserDetail>) -> Self {
        Self {
            status: "missing_deadline",
            detail,
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: "not_found",
            detail: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_portee_ciblee_sans_cible_porte_le_code_de_la_base() {
        let erreur = portee_visee(ScopeType::Event, None).expect_err("refus attendu");
        assert_eq!(erreur.code, ErrorCode::IdentityRoleScopeMismatch);
        assert_eq!(erreur.field.as_deref(), Some("scope_id"));
    }

    #[test]
    fn une_portee_globale_ne_vise_aucune_cible() {
        assert!(portee_visee(ScopeType::Global, Some(Uuid::now_v7())).is_err());
        assert_eq!(
            portee_visee(ScopeType::Global, None).expect("portée globale"),
            Scope::Global
        );
    }
}
