//! Type d'erreur unique de l'API : code stable, message français, champ fautif.
//!
//! Le front branche sur le CODE, jamais sur le texte. Renommer un code est un
//! changement majeur — voir `specs/001-socle-identite/contracts/errors.md`.

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

use crate::context;

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

macro_rules! codes {
    ($( $variant:ident => $code:literal, $status:expr, $message:literal ;)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ErrorCode { $( $variant, )* }

        impl ErrorCode {
            /// Le catalogue entier, dans l'ordre où il est déclaré.
            ///
            /// C'est ce qui permet à la documentation OpenAPI de porter **chaque
            /// code stable** (FR-063) sans qu'on les y recopie : un code ajouté
            /// ici apparaît dans la documentation au prochain démarrage, et un
            /// code oublié n'existe pas.
            pub const ALL: &'static [ErrorCode] = &[ $( Self::$variant, )* ];

            pub fn as_str(self) -> &'static str {
                match self { $( Self::$variant => $code, )* }
            }

            pub fn status(self) -> StatusCode {
                match self { $( Self::$variant => $status, )* }
            }

            /// Message par défaut du catalogue. Une erreur de validation le
            /// remplace par un texte qui nomme le champ.
            pub fn message(self) -> &'static str {
                match self { $( Self::$variant => $message, )* }
            }
        }
    };
}

codes! {
    ValidationFailed => "VALIDATION_FAILED", StatusCode::UNPROCESSABLE_ENTITY,
        "La requête contient une valeur invalide.";
    Unauthenticated => "UNAUTHENTICATED", StatusCode::UNAUTHORIZED,
        "Votre session a expiré. Veuillez vous reconnecter.";
    Forbidden => "FORBIDDEN", StatusCode::FORBIDDEN,
        "Vous n'avez pas les droits nécessaires pour cette action.";
    NotFound => "NOT_FOUND", StatusCode::NOT_FOUND,
        "La ressource demandée est introuvable.";
    Conflict => "CONFLICT", StatusCode::CONFLICT,
        "Cette action entre en conflit avec l'état actuel de la donnée.";
    PayloadTooLarge => "PAYLOAD_TOO_LARGE", StatusCode::PAYLOAD_TOO_LARGE,
        "La requête dépasse la taille autorisée.";
    Internal => "INTERNAL", StatusCode::INTERNAL_SERVER_ERROR,
        "Une erreur interne est survenue. L'incident a été enregistré.";
    ServiceUnavailable => "SERVICE_UNAVAILABLE", StatusCode::SERVICE_UNAVAILABLE,
        "Le service est momentanément indisponible.";

    IdentitySessionExpired => "IDENTITY_SESSION_EXPIRED", StatusCode::UNAUTHORIZED,
        "Votre session a expiré. Veuillez vous reconnecter.";
    IdentitySessionRevoked => "IDENTITY_SESSION_REVOKED", StatusCode::UNAUTHORIZED,
        "Cette session a été fermée. Veuillez vous reconnecter.";
    IdentityRefreshReused => "IDENTITY_REFRESH_REUSED", StatusCode::UNAUTHORIZED,
        "Par sécurité, toutes vos sessions ont été fermées. Veuillez vous reconnecter.";
    IdentityOriginRejected => "IDENTITY_ORIGIN_REJECTED", StatusCode::FORBIDDEN,
        "Requête refusée : origine non autorisée.";
    IdentityPasswordTooWeak => "IDENTITY_PASSWORD_TOO_WEAK", StatusCode::UNPROCESSABLE_ENTITY,
        "Le mot de passe doit compter au moins 8 caractères, dont une majuscule et une minuscule.";

    IdentityEmailAlreadyUsed => "IDENTITY_EMAIL_ALREADY_USED", StatusCode::CONFLICT,
        "Cette adresse est déjà utilisée par une autre personne.";
    IdentityAccountAlreadyExists => "IDENTITY_ACCOUNT_ALREADY_EXISTS", StatusCode::CONFLICT,
        "Cette personne a déjà un compte avec mot de passe.";
    IdentityRoleWindowInvalid => "IDENTITY_ROLE_WINDOW_INVALID", StatusCode::UNPROCESSABLE_ENTITY,
        "La date de fin doit être postérieure à la prise d'effet.";
    IdentityRoleScopeMismatch => "IDENTITY_ROLE_SCOPE_MISMATCH", StatusCode::UNPROCESSABLE_ENTITY,
        "Une portée globale ne vise aucune cible ; une portée ciblée en exige une.";
    IdentityRoleRevocationInvalid => "IDENTITY_ROLE_REVOCATION_INVALID", StatusCode::UNPROCESSABLE_ENTITY,
        "Un motif de retrait ne peut pas être posé sur une attribution en cours.";
    IdentityUnknownReference => "IDENTITY_UNKNOWN_REFERENCE", StatusCode::UNPROCESSABLE_ENTITY,
        "La valeur choisie n'existe pas.";
    IdentityPrivacyWrongAction => "IDENTITY_PRIVACY_WRONG_ACTION", StatusCode::UNPROCESSABLE_ENTITY,
        "L'anonymisation ne répond qu'à une demande d'effacement.";

    // --- Organisations (B2) ---------------------------------------------------
    OrgNotManager => "ORG_NOT_MANAGER", StatusCode::FORBIDDEN,
        "Seul un référent de cette organisation peut effectuer cette action.";
    OrgMembershipIsInvitation => "ORG_MEMBERSHIP_IS_INVITATION", StatusCode::UNPROCESSABLE_ENTITY,
        "Cette adhésion est une invitation : elle attend la réponse de la personne, pas la vôtre.";
    OrgMembershipNotPending => "ORG_MEMBERSHIP_NOT_PENDING", StatusCode::UNPROCESSABLE_ENTITY,
        "Cette adhésion n'attend plus de décision.";
    OrgLastManager => "ORG_LAST_MANAGER", StatusCode::UNPROCESSABLE_ENTITY,
        "Cette organisation n'aurait plus aucun référent. Désignez un remplaçant d'abord.";
    OrgMergeFieldNotArbitrable => "ORG_MERGE_FIELD_NOT_ARBITRABLE", StatusCode::UNPROCESSABLE_ENTITY,
        "L'adresse de la fiche absorbée ne peut pas être reprise : elle reste la sienne, et c'est ce qui fait que ses anciens liens continuent de fonctionner.";
    OrgMergeGlobalScopeRequired => "ORG_MERGE_GLOBAL_SCOPE_REQUIRED", StatusCode::FORBIDDEN,
        "La fusion de deux organisations exige des droits sur l'ensemble de la plateforme.";
    OrgMergeSameOrganization => "ORG_MERGE_SAME_ORGANIZATION", StatusCode::UNPROCESSABLE_ENTITY,
        "Une organisation ne peut pas être fusionnée avec elle-même.";
    OrgDomainVerificationRequired => "ORG_DOMAIN_VERIFICATION_REQUIRED", StatusCode::UNPROCESSABLE_ENTITY,
        "Un rattachement automatique exige un domaine vérifié.";
    OrgNameIsDerived => "ORG_NAME_IS_DERIVED", StatusCode::UNPROCESSABLE_ENTITY,
        "Le nom légal et le sigle suivent la fiche : ils ne se retirent pas à la main.";
    OrgUnknownReference => "ORG_UNKNOWN_REFERENCE", StatusCode::UNPROCESSABLE_ENTITY,
        "La valeur choisie n'existe pas.";
    OrgInvitationNotYours => "ORG_INVITATION_NOT_YOURS", StatusCode::FORBIDDEN,
        "Cette invitation ne vous est pas adressée.";

    // --- Événements (B3) ------------------------------------------------------
    // Trois seulement : la quasi-totalité des refus de ce module sont exprimés
    // par le contrat du front et sortent donc en 200. Ces trois-là n'y ont
    // aucune place.
    EventGlobalScopeRequired => "EVENT_GLOBAL_SCOPE_REQUIRED", StatusCode::FORBIDDEN,
        "La création d'une édition exige des droits sur l'ensemble de la plateforme.";
    EventCriterionHasScores => "EVENT_CRITERION_HAS_SCORES", StatusCode::UNPROCESSABLE_ENTITY,
        "Ce critère porte déjà des notes : le retirer effacerait l'argumentaire des évaluations rendues.";
    EventUnknownReference => "EVENT_UNKNOWN_REFERENCE", StatusCode::UNPROCESSABLE_ENTITY,
        "La valeur choisie n'existe pas.";

    // --- Propositions (B4) ----------------------------------------------------
    // Six, et pas plus. Sept refus métier de ce module sont déjà des membres
    // d'union du contrat du front et sortent en 200 avec leur discriminant :
    // appel clos, plafond atteint, transition impossible, motif exigé, et les
    // trois écarts d'une action groupée. Aucun code n'est ajouté pour la
    // recevabilité — ses refus sont des RÉPONSES, pas des erreurs.
    ProposalNotEditable => "PROPOSAL_NOT_EDITABLE", StatusCode::UNPROCESSABLE_ENTITY,
        "Ce dossier n'est plus modifiable. Vous pouvez en déposer un nouveau.";
    ProposalSpeakerIdentityLocked => "PROPOSAL_SPEAKER_IDENTITY_LOCKED", StatusCode::UNPROCESSABLE_ENTITY,
        "Cette personne possède un compte : son identité lui appartient et ne se modifie pas depuis un dossier.";
    ProposalReviewNotAssigned => "PROPOSAL_REVIEW_NOT_ASSIGNED", StatusCode::FORBIDDEN,
        "Ce dossier ne vous est pas confié : vous pouvez le lire, pas le noter.";
    ProposalUnknownTerm => "PROPOSAL_UNKNOWN_TERM", StatusCode::UNPROCESSABLE_ENTITY,
        "Cette thématique n'existe pas.";
    ProposalTextTooLong => "PROPOSAL_TEXT_TOO_LONG", StatusCode::UNPROCESSABLE_ENTITY,
        "Ce texte dépasse la longueur autorisée.";
    ProposalUnknownReference => "PROPOSAL_UNKNOWN_REFERENCE", StatusCode::UNPROCESSABLE_ENTITY,
        "La valeur choisie n'existe pas.";

    // MAIL_RELAY_UNREACHABLE n'est PAS ici : il ne franchit aucune réponse
    // HTTP. Il vit dans `mail.rs`, d'où il part vers `platform.jobs.last_error`.
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub field: Option<String>,
    /// Détail technique : part dans la trace, jamais dans la réponse.
    pub detail: Option<String>,
}

impl ApiError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.message().to_owned(),
            field: None,
            detail: None,
        }
    }

    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Self::new(code)
        }
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn detail(mut self, detail: impl fmt::Display) -> Self {
        self.detail = Some(detail.to_string());
        self
    }

    pub fn internal(detail: impl fmt::Display) -> Self {
        Self::new(ErrorCode::Internal).detail(detail)
    }

    pub fn forbidden() -> Self {
        Self::new(ErrorCode::Forbidden)
    }

    pub fn unauthenticated() -> Self {
        Self::new(ErrorCode::Unauthenticated)
    }

    pub fn not_found() -> Self {
        Self::new(ErrorCode::NotFound)
    }

    pub fn validation(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ValidationFailed, message).field(field)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)?;
        if let Some(detail) = &self.detail {
            write!(f, " ({detail})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.code.status()
    }

    fn error_response(&self) -> HttpResponse {
        if let Some(detail) = &self.detail {
            tracing::error!(code = self.code.as_str(), detail, "erreur d'API");
        }
        HttpResponse::build(self.status_code()).json(ErrorBody {
            code: self.code.as_str(),
            message: &self.message,
            field: self.field.as_deref(),
            request_id: context::current_request_id(),
        })
    }
}

/// Une erreur de base non traduite ne sort jamais telle quelle : son texte
/// porte des noms de tables, parfois des valeurs.
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        crate::pg_error::translate(&err)
    }
}

/// Un corps mal formé est une erreur de validation comme une autre : elle doit
/// porter son code stable et son message français, pas le texte anglais de
/// serde — qui nomme au passage les champs de la charge utile.
///
/// Le seul emprunt fait à ce texte est le **nom du champ manquant**, extrait
/// entre les accents graves et filtré : il est utile au formulaire, et le front
/// désigne ses champs par ces noms-là.
impl From<&actix_web::error::JsonPayloadError> for ApiError {
    fn from(err: &actix_web::error::JsonPayloadError) -> Self {
        use actix_web::error::JsonPayloadError;

        match err {
            JsonPayloadError::Overflow { .. } | JsonPayloadError::OverflowKnownLength { .. } => {
                ApiError::new(ErrorCode::PayloadTooLarge).detail(err)
            }
            JsonPayloadError::Deserialize(source) => match champ_fautif(&source.to_string()) {
                Some(champ) => ApiError::validation(
                    format!("Le champ « {champ} » est absent ou mal formé."),
                    champ,
                )
                .detail(err),
                None => ApiError::new(ErrorCode::ValidationFailed).detail(err),
            },
            _ => ApiError::new(ErrorCode::ValidationFailed).detail(err),
        }
    }
}

/// `missing field \`password\` at line 1 column 20` → `password`. Le filtre est
/// volontairement étroit : ce qui sort d'ici part dans une réponse.
///
/// **Seuls deux messages de serde nomment un champ.** Les autres nomment une
/// valeur — `unknown variant \`anonymized\`, expected one of …` —, et prendre le
/// premier terme entre accents graves ferait désigner à l'écran un champ qui
/// n'existe pas. Sans préfixe reconnu, on rend le refus sans champ : générique,
/// mais jamais trompeur.
fn champ_fautif(texte: &str) -> Option<String> {
    if !texte.starts_with("missing field ") && !texte.starts_with("unknown field ") {
        return None;
    }

    let apres = texte.split_once('`')?.1;
    let champ = apres.split_once('`')?.0;

    let acceptable = !champ.is_empty()
        && champ.len() <= 40
        && champ
            .bytes()
            .all(|o| o.is_ascii_lowercase() || o.is_ascii_digit() || o == b'_');

    acceptable.then(|| champ.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_champ_manquant_est_extrait_du_texte_de_serde() {
        assert_eq!(
            champ_fautif("missing field `password` at line 1 column 20").as_deref(),
            Some("password")
        );
    }

    /// Le texte qui nomme une VALEUR, pas un champ : ce cas désignait
    /// `anonymized` comme champ fautif, et l'écran soulignait une case qui
    /// n'existe pas.
    #[test]
    fn une_valeur_hors_liste_ne_se_fait_pas_passer_pour_un_champ() {
        assert_eq!(
            champ_fautif("unknown variant `anonymized`, expected one of `active`, `suspended`"),
            None
        );
    }

    #[test]
    fn un_champ_inconnu_est_nomme() {
        assert_eq!(
            champ_fautif("unknown field `granted`, expected one of `role_code`").as_deref(),
            Some("granted")
        );
    }

    #[test]
    fn rien_dautre_ne_franchit_lextraction() {
        assert_eq!(champ_fautif("expected value at line 1 column 1"), None);
        assert_eq!(
            champ_fautif("unknown token `Ceci n'est pas un champ`"),
            None
        );
    }
}
