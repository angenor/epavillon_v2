//! Les adhésions : deux files qui ne se confondent jamais.
//!
//! **Le même mot recouvre deux attentes inverses.** Une personne qui DEMANDE à
//! rejoindre attend qu'un référent l'accepte ; une personne qu'un référent
//! INVITE attend, elle, d'accepter elle-même. Le statut `pending` ne dit pas
//! laquelle des deux : c'est `invited_at` qui porte la direction.
//!
//! Les confondre, c'est faire entrer quelqu'un qui n'a jamais rien accepté.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ids::{MembershipId, OrganizationId, PersonId};
use super::organization::Organization;

/// `org.membership_role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRole {
    Manager,
    Member,
    Contributor,
}

impl MembershipRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manager => "manager",
            Self::Member => "member",
            Self::Contributor => "contributor",
        }
    }

    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "manager" => Some(Self::Manager),
            "member" => Some(Self::Member),
            "contributor" => Some(Self::Contributor),
            _ => None,
        }
    }
}

/// `org.membership_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipStatus {
    Pending,
    Active,
    Revoked,
}

impl MembershipStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }

    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "pending" => Some(Self::Pending),
            "active" => Some(Self::Active),
            "revoked" => Some(Self::Revoked),
            _ => None,
        }
    }
}

/// `Membership` de `frontend/app/types/org.ts` — la ligne, telle quelle.
#[derive(Debug, Clone, Serialize)]
pub struct Membership {
    pub id: MembershipId,
    pub organization_id: OrganizationId,
    pub person_id: PersonId,
    pub role: MembershipRole,
    pub status: MembershipStatus,
    /// **Jamais calculé par le service** : `tg_default_primary_membership`
    /// attribue la primauté à la première adhésion active.
    pub is_primary: bool,
    pub job_title: Option<String>,
    pub invited_by: Option<PersonId>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub invited_at: Option<OffsetDateTime>,
    pub approved_by: Option<PersonId>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub approved_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl Membership {
    /// L'organisation a-t-elle invité ? C'est ce qui sépare les deux files, et
    /// la seule question qui décide de **qui** peut trancher.
    ///
    /// **La colonne reste renseignée après l'acceptation** : elle porte
    /// l'histoire de l'adhésion, pas son état. Une adhésion active née d'une
    /// invitation répond donc vrai ici — et n'attend pourtant plus personne.
    pub fn is_invitation(&self) -> bool {
        self.invited_at.is_some()
    }

    /// Une invitation **qui attend encore**. C'est la question que se posent
    /// l'invitation répétée et la décision d'un référent : les deux doivent
    /// distinguer « en vol » de « déjà honorée », ce que `is_invitation` seul ne
    /// fait pas.
    pub fn is_pending_invitation(&self) -> bool {
        self.status == MembershipStatus::Pending && self.is_invitation()
    }
}

/// Ce que le formulaire de rattachement envoie.
///
/// **`organization_id` est facultatif dans le corps** : c'est le chemin qui fait
/// foi, et la route l'y remplace toujours. Le contrat du front le porte encore —
/// il vient des données simulées, qui n'ont pas de chemin —, et l'exiger
/// obligerait à l'envoyer en double sous peine d'un refus de validation
/// incompréhensible.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct JoinOrganization {
    #[serde(default)]
    pub organization_id: Option<Uuid>,
    #[serde(default)]
    pub job_title: Option<String>,
}

/// `JoinOrganizationResult`. **`pending` n'est pas un échec** : c'est le
/// fonctionnement normal quand le domaine ne prouve rien.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JoinOutcome {
    Joined {
        membership_id: MembershipId,
        organization: Box<Organization>,
    },
    Pending {
        membership_id: MembershipId,
        organization: Box<Organization>,
    },
    AlreadyMember {
        organization: Box<Organization>,
        membership_status: MembershipStatus,
    },
}

/// Ce que révèle le domaine d'une adresse — `EmailDomainMatch`.
///
/// `null` sur messagerie grand public ou domaine inconnu : dans les deux cas
/// l'écran ne propose rien, il ne devine pas.
#[derive(Debug, Clone, Serialize)]
pub struct EmailDomainMatch {
    pub domain: String,
    pub organization: Box<Organization>,
    pub domain_record: OrganizationDomainRecord,
    /// Vrai seulement si le domaine est **vérifié** et marqué `auto_join`. La
    /// vérification `ck_domain_autojoin_requires_verification` garantit déjà que
    /// le second implique le premier ; on le teste quand même, une donnée
    /// importée pouvant précéder la contrainte.
    pub can_auto_join: bool,
    pub member_count: i64,
}

/// La ligne de `org.organization_domains` qui a produit la correspondance.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationDomainRecord {
    pub id: Uuid,
    pub organization_id: OrganizationId,
    pub domain: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    pub verification_method: Option<String>,
    pub auto_join: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Ce que l'invitation d'un membre porte. `organization_id` vient du chemin —
/// voir [`JoinOrganization`].
#[derive(Debug, Clone, Deserialize)]
pub struct InviteMember {
    #[serde(default)]
    pub organization_id: Option<Uuid>,
    pub email: String,
    #[serde(default = "role_par_defaut")]
    pub role: MembershipRole,
    #[serde(default)]
    pub job_title: Option<String>,
}

fn role_par_defaut() -> MembershipRole {
    MembershipRole::Member
}

/// Une personne, réduite à ce que la file affiche.
#[derive(Debug, Clone, Serialize)]
pub struct MemberPerson {
    pub id: PersonId,
    pub display_name: String,
    pub primary_email: String,
    pub first_name: String,
    pub last_name: String,
    pub preferred_locale: String,
}

/// `MemberEntry` — l'adhésion, la personne, et la direction de l'attente.
#[derive(Debug, Clone, Serialize)]
pub struct MemberEntry {
    pub membership: Membership,
    pub person: MemberPerson,
    /// L'organisation a invité cette personne et attend sa réponse.
    pub is_invitation: bool,
}

/// `InviteMemberResult`. Trois issues, et l'écran les rend différemment :
/// `already_invited` propose de **relancer**, jamais d'émettre une seconde
/// invitation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InviteOutcome {
    Invited { entry: Box<MemberEntry> },
    AlreadyMember { entry: Box<MemberEntry> },
    AlreadyInvited { entry: Box<MemberEntry> },
}

/// La décision d'un référent sur une **demande**. Elle ne vise jamais une
/// invitation : approuver une invitation ferait entrer quelqu'un qui n'a rien
/// répondu.
#[derive(Debug, Clone, Deserialize)]
pub struct DecideMembership {
    /// Facultatif : le chemin fait foi.
    #[serde(default)]
    pub membership_id: Option<Uuid>,
    pub approved: bool,
}

/// Le jeton d'une invitation, et ce qu'il porte.
#[derive(Debug, Clone, Deserialize)]
pub struct AcceptInvitation {
    pub token: String,
}

/// Ce que rend l'acceptation d'une invitation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AcceptInvitationOutcome {
    Accepted {
        membership: Box<Membership>,
        organization: Box<Organization>,
    },
    /// Les trois refus de jeton du noyau, rendus tels quels.
    Rejected {
        reason: kernel::tokens::TokenRejection,
    },
}

/// Ce que rend une révocation. `last_manager` n'est pas une erreur HTTP quand
/// c'est la personne elle-même qui part : c'est un refus que l'écran affiche.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RevokeOutcome {
    Revoked,
    LastManager,
}

/// Charge utile du jeton d'invitation. Elle ne porte **pas** le jeton
/// lui-même — celui-ci vit dans la charge du travail d'envoi, effacée dès la
/// remise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationPayload {
    pub organization_id: Uuid,
    pub membership_id: Uuid,
    pub email: String,
}

impl InvitationPayload {
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}
