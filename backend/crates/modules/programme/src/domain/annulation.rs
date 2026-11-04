//! L'organisation annule son activité retenue (04/11/2026) : ce qui l'autorise.
//!
//! Le garde du modèle exige un motif quand la règle le dit, et la règle
//! `accepted -> cancelled` ne l'exige pas : l'équipe annule sans motif depuis le
//! 28/10. Le motif de l'organisation est donc exigé ici, et l'état des séances
//! aussi, que le garde ne lit pas.

use kernel::error::{ApiError, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refus {
    PasRetenue,
    SeanceCommencee,
    MotifManquant,
}

impl Refus {
    pub fn message(self) -> &'static str {
        match self {
            Self::PasRetenue => {
                "Seule une activité retenue peut être annulée par l'organisation. \
                 Un dossier encore en évaluation se retire, il ne s'annule pas."
            }
            Self::SeanceCommencee => {
                "Une séance de cette activité a déjà commencé : elle ne peut plus être \
                 annulée. Écrivez à l'équipe de l'ePavillon."
            }
            Self::MotifManquant => "Indiquez le motif de l'annulation : l'équipe en a besoin.",
        }
    }
}

impl From<Refus> for ApiError {
    fn from(refus: Refus) -> Self {
        match refus {
            Refus::PasRetenue | Refus::SeanceCommencee => {
                ApiError::with_message(ErrorCode::Conflict, refus.message())
            }
            Refus::MotifManquant => {
                ApiError::with_message(ErrorCode::ValidationFailed, refus.message()).field("reason")
            }
        }
    }
}

/// Dans cet ordre : un dossier qui ne s'annule pas n'a pas à réclamer de motif.
pub fn verifier(statut: &str, seance_commencee: bool, motif: &str) -> Result<(), Refus> {
    if statut != "accepted" {
        return Err(Refus::PasRetenue);
    }
    if seance_commencee {
        return Err(Refus::SeanceCommencee);
    }
    if motif.trim().is_empty() {
        return Err(Refus::MotifManquant);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_activite_retenue_et_pas_commencee_s_annule_avec_motif() {
        assert_eq!(
            verifier("accepted", false, "Intervenant indisponible"),
            Ok(())
        );
    }

    #[test]
    fn le_motif_est_exige_et_des_espaces_n_en_sont_pas_un() {
        assert_eq!(verifier("accepted", false, ""), Err(Refus::MotifManquant));
        assert_eq!(
            verifier("accepted", false, "   \n"),
            Err(Refus::MotifManquant)
        );
    }

    #[test]
    fn seul_un_dossier_retenu_s_annule() {
        for statut in [
            "draft",
            "submitted",
            "under_review",
            "cancelled",
            "rejected",
        ] {
            assert_eq!(verifier(statut, false, "motif"), Err(Refus::PasRetenue));
        }
    }

    #[test]
    fn une_seance_commencee_ferme_l_annulation() {
        assert_eq!(
            verifier("accepted", true, "motif"),
            Err(Refus::SeanceCommencee)
        );
    }

    #[test]
    fn les_refus_parlent_francais_sans_tiret_cadratin() {
        for refus in [
            Refus::PasRetenue,
            Refus::SeanceCommencee,
            Refus::MotifManquant,
        ] {
            assert!(!refus.message().contains('\u{2014}'));
        }
        let erreur = ApiError::from(Refus::MotifManquant);
        assert_eq!(erreur.code, ErrorCode::ValidationFailed);
        assert_eq!(
            ApiError::from(Refus::SeanceCommencee).code,
            ErrorCode::Conflict
        );
    }
}
