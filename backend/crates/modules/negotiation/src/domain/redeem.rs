//! Les neuf issues d'un code, et le message que l'API compose pour chacune.
//!
//! # POURQUOI LE MESSAGE VIENT D'ICI ET NON DE L'ÉCRAN
//!
//! Sept de ces neuf issues sont des refus, et chacune dit quelque chose que le
//! client ne sait pas : la date de révocation, le temps d'attente restant, le
//! nom du réseau rejoint. Un catalogue côté client donnerait deux textes pour un
//! même refus, et le second se périmerait au premier changement (FR-020). Les
//! titres, les aides et les boutons autour restent de l'i18n.
//!
//! # AUCUNE ISSUE NE LAISSE L'ÉCRAN SANS SUITE
//!
//! FR-015. Chaque message dit ce qui se passe **et** ce qu'on peut faire
//! ensuite — ressaisir, demander le code en cours au groupe, ou demander
//! l'accès à l'IFDD. C'est la raison pour laquelle ces refus sortent en 200 :
//! en 4xx, le transport du client les traiterait comme des pannes et l'écran
//! perdrait ses sorties (recherche R3).

use serde::Serialize;
use time::{Month, OffsetDateTime};

use crate::domain::access::{AccessRequestView, GrantedAccess, NetworkView};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RedeemIssue {
    Accepted,
    /// Mode « les deux » : le code est juste, il ne suffit pas.
    PendingApproval,
    AlreadyGranted,
    Unknown,
    Revoked,
    Exhausted,
    Expired,
    NotYetValid,
    Throttled,
}

impl RedeemIssue {
    /// Ce que la table des essais enregistre. **`already_granted` et
    /// `pending_approval` comptent pour `accepted`** : le code était juste, et
    /// un code juste ne consomme aucune tentative.
    pub fn outcome_db(self) -> &'static str {
        match self {
            Self::Accepted | Self::AlreadyGranted | Self::PendingApproval => "accepted",
            Self::Unknown => "unknown",
            Self::Revoked => "revoked",
            Self::Exhausted => "exhausted",
            Self::Expired => "expired",
            Self::NotYetValid => "not_yet_valid",
            Self::Throttled => "throttled",
        }
    }
}

/// `RedeemResult` — ce que rend `POST /invitation-codes/redeem`, en 200 pour
/// les neuf issues.
#[derive(Debug, Clone, Serialize)]
pub struct RedeemResult {
    pub issue: RedeemIssue,
    pub message: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
    pub retry_after_seconds: Option<i64>,
    pub granted: Option<GrantedAccess>,
    pub networks: Vec<NetworkView>,
    /// Renseignée pour la seule issue `pending_approval`.
    pub request: Option<AccessRequestView>,
}

impl RedeemResult {
    fn nu(issue: RedeemIssue, message: impl Into<String>) -> Self {
        Self {
            issue,
            message: message.into(),
            revoked_at: None,
            retry_after_seconds: None,
            granted: None,
            networks: Vec::new(),
            request: None,
        }
    }

    /// Le code est juste et l'accès est ouvert. **Le message nomme le réseau**
    /// quand le code en porte un (FR-011), et son libellé vient de la
    /// taxonomie — jamais d'un fichier de traduction.
    pub fn accepte(
        accorde: GrantedAccess,
        networks: Vec<NetworkView>,
        reseau_rejoint: Option<&str>,
    ) -> Self {
        let message = match reseau_rejoint {
            Some(label) => format!("Code reconnu. Bienvenue dans le {label}."),
            None => "Code reconnu. Les modules réservés sont ouverts.".to_owned(),
        };
        Self {
            granted: Some(accorde),
            networks,
            ..Self::nu(RedeemIssue::Accepted, message)
        }
    }

    /// L'accès était déjà là. **Ni second accès, ni perte de celui qu'on a**
    /// (FR-018) — seule l'appartenance au réseau a pu s'ajouter.
    pub fn deja_admise(
        accorde: GrantedAccess,
        networks: Vec<NetworkView>,
        reseau_rejoint: Option<&str>,
    ) -> Self {
        let message = match reseau_rejoint {
            Some(label) => {
                format!("Vous avez déjà l'accès. L'appartenance au {label} vient de s'y ajouter.")
            }
            None => "Vous avez déjà l'accès aux modules réservés.".to_owned(),
        };
        Self {
            granted: Some(accorde),
            networks,
            ..Self::nu(RedeemIssue::AlreadyGranted, message)
        }
    }

    /// Mode « approbation » ou « les deux » : le code reconnu ouvre une demande
    /// qui le porte, et l'administrateur tranche (FR-023).
    pub fn demande_ouverte(demande: AccessRequestView) -> Self {
        Self {
            request: Some(demande),
            ..Self::nu(
                RedeemIssue::PendingApproval,
                "Code reconnu. L'entrée demande l'accord d'un administrateur : \
                 votre demande est enregistrée, et la réponse vous arrivera par courriel.",
            )
        }
    }

    pub fn inconnu() -> Self {
        Self::nu(
            RedeemIssue::Unknown,
            "Ce code n'est pas reconnu. Vérifiez les huit caractères, tirets compris.",
        )
    }

    /// **La date est ce qui distingue ce refus d'un code inconnu.** Sans elle,
    /// la personne chercherait une faute de frappe dans un code juste — c'est
    /// la raison pour laquelle l'unicité du code couvre aussi les révoqués.
    pub fn revoque(revoque_le: OffsetDateTime) -> Self {
        Self {
            revoked_at: Some(revoque_le),
            ..Self::nu(
                RedeemIssue::Revoked,
                format!(
                    "Ce code a été révoqué le {}. Le réseau en a reçu un nouveau dans son \
                     groupe WhatsApp.",
                    jour_et_mois(revoque_le)
                ),
            )
        }
    }

    pub fn epuise() -> Self {
        Self::nu(
            RedeemIssue::Exhausted,
            "Ce code a atteint son nombre d'entrées. Demandez le code en cours à votre \
             groupe, ou faites une demande à l'IFDD.",
        )
    }

    pub fn termine() -> Self {
        Self::nu(
            RedeemIssue::Expired,
            "Ce code a terminé sa validité. Demandez le code en cours à votre groupe, ou \
             faites une demande à l'IFDD.",
        )
    }

    pub fn pas_encore_ouvert(ouvre_le: OffsetDateTime) -> Self {
        Self::nu(
            RedeemIssue::NotYetValid,
            format!(
                "Ce code n'est pas encore ouvert : il le sera le {}.",
                jour_et_mois(ouvre_le)
            ),
        )
    }

    /// **Le refus ne dit rien du code essayé** (FR-017) : ni qu'il existe, ni
    /// qu'il est révoqué, ni qu'il est épuisé. Il dit seulement d'attendre, et
    /// que la demande d'accès reste ouverte.
    pub fn trop_dessais(attente_secondes: i64) -> Self {
        Self {
            retry_after_seconds: Some(attente_secondes),
            ..Self::nu(
                RedeemIssue::Throttled,
                format!(
                    "Trop d'essais. Réessayez {}. Vous pouvez aussi faire une demande à l'IFDD.",
                    dans_combien_de_temps(attente_secondes)
                ),
            )
        }
    }
}

/// « 8 novembre », et « 8 novembre 2025 » si l'année n'est pas la courante.
///
/// **Formaté en temps universel**, et c'est un choix : la réponse porte aussi
/// `revoked_at` en entier, donc un écran qui voudrait le jour local l'a. Poser
/// le fuseau de la personne demanderait une lecture de plus pour un jour de
/// décalage possible autour de minuit.
fn jour_et_mois(instant: OffsetDateTime) -> String {
    let jour = instant.day();
    let mois = nom_du_mois(instant.month());
    if instant.year() == OffsetDateTime::now_utc().year() {
        format!("{jour} {mois}")
    } else {
        format!("{jour} {mois} {}", instant.year())
    }
}

fn nom_du_mois(mois: Month) -> &'static str {
    match mois {
        Month::January => "janvier",
        Month::February => "février",
        Month::March => "mars",
        Month::April => "avril",
        Month::May => "mai",
        Month::June => "juin",
        Month::July => "juillet",
        Month::August => "août",
        Month::September => "septembre",
        Month::October => "octobre",
        Month::November => "novembre",
        Month::December => "décembre",
    }
}

fn dans_combien_de_temps(secondes: i64) -> String {
    let minutes = (secondes + 59) / 60;
    match minutes {
        ..=0 => "dans un instant".to_owned(),
        1 => "dans une minute".to_owned(),
        m => format!("dans {m} minutes"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_refus_par_limite_annonce_son_attente() {
        let resultat = RedeemResult::trop_dessais(900);
        assert_eq!(resultat.issue, RedeemIssue::Throttled);
        assert!(
            resultat.message.contains("dans 15 minutes"),
            "{}",
            resultat.message
        );
        assert_eq!(resultat.retry_after_seconds, Some(900));
    }

    #[test]
    fn un_code_juste_ne_consomme_pas_de_tentative() {
        for issue in [
            RedeemIssue::Accepted,
            RedeemIssue::AlreadyGranted,
            RedeemIssue::PendingApproval,
        ] {
            assert_eq!(issue.outcome_db(), "accepted");
        }
    }
}
