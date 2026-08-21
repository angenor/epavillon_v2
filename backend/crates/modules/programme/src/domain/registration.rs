//! Les six issues d'une tentative d'inscription, et ce qui les décide.
//!
//! # Le partage passe entre le hasard et la faute
//!
//! Une personne peut arriver une minute après la clôture, ou trouver la salle
//! pleine : ce sont des **issues normales** d'une tentative bien formée, et
//! elles sortent en 200 avec leur valeur. Un formulaire mal rempli, lui, est une
//! erreur — code stable, message français, champ nommé.
//!
//! # La valeur qui accompagne un refus est RELUE, jamais extraite d'une phrase
//!
//! Le déclencheur écrit « Capacité atteinte (30 places). » ; le service rend
//! `capacity` en le relisant sur la séance. Extraire un nombre d'un message
//! français est un piège que B3 a déjà nommé.

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;

/// L'issue d'une tentative bien formée — `RegistrationResult`.
///
/// **La bascule en liste d'attente n'est pas un refus, c'est une place** : elle
/// porte la position obtenue, que la base a posée.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IssueDInscription {
    Registered {
        registration: serde_json::Value,
    },
    Waitlisted {
        registration: serde_json::Value,
        position: i32,
    },
    /// Une inscription vivante existe déjà : la ligne est **relue et rendue**,
    /// et non refusée (`ux_registrations_person_session`).
    AlreadyRegistered {
        registration: serde_json::Value,
    },
    /// Jauge atteinte **sans liste d'attente**. Le nombre de places est relu sur
    /// la séance.
    Full {
        capacity: i32,
    },
    Closed {
        #[serde(with = "time::serde::rfc3339")]
        closed_at: OffsetDateTime,
    },
    NotOpenYet {
        #[serde(with = "time::serde::rfc3339")]
        opens_at: OffsetDateTime,
    },
}

/// Ce qu'une annulation rend — `CancelRegistrationResult`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AnnulationRendue {
    pub registration: serde_json::Value,
    /// Personnes promues depuis la liste d'attente : **0 ou 1** (R20). Une
    /// annulation libère une place, et une seule.
    pub promoted: i64,
}

/// Une ligne de la liste **nominative** du back-office — `RegistrationRow`.
///
/// Elle exige `programme.registration.manage` sur l'édition. Le rôle de
/// programmation ne la détient pas (écart n° 119) : une chargée de
/// programmation compose la grille sans pouvoir ouvrir la liste des inscrits.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LigneDInscrit {
    pub registration: serde_json::Value,
    pub person: serde_json::Value,
    pub organization_name: Option<String>,
}

/// Les quatre fenêtres qui décident si une séance prend une inscription —
/// **chacune avec son propre motif** (écart n° 115).
///
/// La base n'en vérifie qu'une seule et demie : elle refuse une séance annulée
/// et une clôture dépassée, mais ignore `registration_required` et la date
/// d'ouverture. Les quatre sont donc décidées ici, sur des valeurs relues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fenetre {
    Ouverte,
    /// La séance est annulée : `REGISTRATION_NOT_ACCEPTED`.
    SeanceAnnulee,
    /// `registration_required` est faux : la séance ne prend pas d'inscription.
    SansInscription,
    /// Avant `registration_opens_at`.
    PasEncoreOuverte(OffsetDateTime),
    /// Après `registration_closes_at`.
    Close(OffsetDateTime),
}

/// L'état d'une séance, du seul point de vue de l'inscription.
#[derive(Debug, Clone, Copy)]
pub struct EtatDInscription {
    pub annulee: bool,
    pub inscription_requise: bool,
    pub ouvre_le: Option<OffsetDateTime>,
    pub ferme_le: Option<OffsetDateTime>,
}

/// Décider la fenêtre, **dans l'ordre où les motifs comptent**.
///
/// L'annulation d'abord : une séance qui n'a pas lieu ne se discute pas. Puis
/// l'absence d'inscription, qui est un fait de la séance. Puis les deux bornes
/// de temps, qui sont des faits de l'instant.
pub fn fenetre(etat: EtatDInscription, maintenant: OffsetDateTime) -> Fenetre {
    if etat.annulee {
        return Fenetre::SeanceAnnulee;
    }
    if !etat.inscription_requise {
        return Fenetre::SansInscription;
    }
    if let Some(ouvre) = etat.ouvre_le {
        if maintenant < ouvre {
            return Fenetre::PasEncoreOuverte(ouvre);
        }
    }
    if let Some(ferme) = etat.ferme_le {
        if maintenant > ferme {
            return Fenetre::Close(ferme);
        }
    }
    Fenetre::Ouverte
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn etat() -> EtatDInscription {
        EtatDInscription {
            annulee: false,
            inscription_requise: true,
            ouvre_le: None,
            ferme_le: None,
        }
    }

    const MAINTENANT: OffsetDateTime = datetime!(2027-11-01 12:00 UTC);

    #[test]
    fn une_seance_ordinaire_est_ouverte() {
        assert_eq!(fenetre(etat(), MAINTENANT), Fenetre::Ouverte);
    }

    /// Les quatre motifs sont **distincts** : l'écran n'affiche pas la même
    /// chose selon qu'une séance est annulée ou que les inscriptions ne sont pas
    /// encore ouvertes.
    #[test]
    fn les_quatre_fenetres_rendent_quatre_motifs_distincts() {
        let annulee = EtatDInscription {
            annulee: true,
            ..etat()
        };
        assert_eq!(fenetre(annulee, MAINTENANT), Fenetre::SeanceAnnulee);

        let sans = EtatDInscription {
            inscription_requise: false,
            ..etat()
        };
        assert_eq!(fenetre(sans, MAINTENANT), Fenetre::SansInscription);

        let ouvre = datetime!(2027-11-05 09:00 UTC);
        let plus_tard = EtatDInscription {
            ouvre_le: Some(ouvre),
            ..etat()
        };
        assert_eq!(
            fenetre(plus_tard, MAINTENANT),
            Fenetre::PasEncoreOuverte(ouvre)
        );

        let ferme = datetime!(2027-10-30 18:00 UTC);
        let close = EtatDInscription {
            ferme_le: Some(ferme),
            ..etat()
        };
        assert_eq!(fenetre(close, MAINTENANT), Fenetre::Close(ferme));
    }

    /// L'annulation prime : une séance annulée dont les inscriptions seraient
    /// aussi closes doit dire qu'elle est annulée.
    #[test]
    fn lannulation_prime_sur_la_cloture() {
        let etat = EtatDInscription {
            annulee: true,
            ferme_le: Some(datetime!(2027-10-30 18:00 UTC)),
            ..etat()
        };
        assert_eq!(fenetre(etat, MAINTENANT), Fenetre::SeanceAnnulee);
    }
}
