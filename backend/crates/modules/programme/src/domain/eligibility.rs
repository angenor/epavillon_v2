//! Le classement des trois refus de recevabilité en réponses nommées (R9).
//!
//! # L'entorse, et pourquoi elle est assumée
//!
//! Le principe VIII interdit de réimplémenter un invariant de la base, et
//! `tg_check_submission_eligibility()` tient déjà les trois conditions. **Ce
//! fichier les évalue quand même**, pour une raison qui n'a pas d'autre issue :
//! le contrat du front n'attend pas une erreur mais **deux réponses portant des
//! valeurs** — l'échéance pour un appel clos, le plafond pour un quota atteint.
//! Le déclencheur ne les rend que dans une phrase française interpolée, et les
//! extraire d'un message est la dépendance la plus fragile qu'on puisse écrire.
//!
//! S'y ajoute que le **même** code d'erreur PostgreSQL — `restrict_violation`,
//! 23001 — sert aux transitions interdites *et* aux trois refus de
//! recevabilité : sans classement préalable, on ne saurait même pas laquelle
//! des quatre causes s'applique (R8).
//!
//! # Ce qui borne l'entorse
//!
//! Trois conditions, **lues et non recalculées**, dans la même transaction que
//! l'écriture qu'elles précèdent. Le déclencheur n'est ni désactivé ni
//! contourné : une course entre la lecture et l'écriture **retombe sur lui**,
//! et son refus sort tel quel.

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;

/// Ce que la base porte sur l'appel au moment du dépôt, lu tel quel.
#[derive(Debug, Clone)]
pub struct EtatDeLAppel {
    /// `event.calls_for_proposals.status`.
    pub statut: String,
    pub ouvre_le: OffsetDateTime,
    /// `event.effective_deadline()` : prolongation comprise, jamais `closes_at`
    /// seule.
    pub echeance: OffsetDateTime,
    pub plafond_par_organisation: Option<i16>,
    pub exige_organisation_verifiee: bool,
}

/// Ce que la base porte sur l'organisation porteuse.
#[derive(Debug, Clone, Copy)]
pub struct EtatDeLOrganisation {
    /// Dossiers déjà comptés dans le plafond, **celui-ci exclu** — le
    /// déclencheur s'exclut lui-même par `p.id <> NEW.id`, et le décompte doit
    /// dire la même chose.
    pub dossiers_comptes: i64,
    pub verifiee: bool,
}

/// L'issue du classement.
///
/// **Les trois refus ne sont pas des erreurs** : deux sont des membres d'union
/// du contrat et sortent en 200 avec leur valeur. Le troisième — organisation
/// non vérifiée — n'a pas de membre d'union, et c'est voulu : l'écran ne le
/// rencontre pas, la campagne de la COP31 n'exigeant pas la vérification. S'il
/// survient, il remonte tel que le déclencheur le formule.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Recevabilite {
    /// Rien ne s'oppose au dépôt. Le déclencheur reste le dernier mot.
    Recevable,
    /// L'appel n'accepte plus de dépôt — statut, ou fenêtre.
    CallClosed {
        #[serde(with = "time::serde::rfc3339")]
        deadline: OffsetDateTime,
    },
    /// Le plafond de l'organisation est atteint sur cet appel.
    QuotaReached { max: i16 },
    /// L'appel est réservé aux organisations vérifiées par l'IFDD.
    OrganizationNotVerified,
}

/// Classe les trois refus, **dans l'ordre du déclencheur**.
///
/// L'ordre compte : un dossier hors fenêtre *et* au-delà du plafond doit rendre
/// le même refus que celui que la base aurait rendu, sans quoi le classement
/// préalable et le dernier mot diraient deux choses différentes du même dépôt.
///
/// # La fenêtre ne vaut que pour un **premier** dépôt
///
/// C'est écrit dans le modèle et ce n'est pas un confort : le comité demande
/// ses corrections **après** la clôture — c'est même le cas normal, l'évaluation
/// commençant quand l'appel se ferme. Un contrôle indifférencié refuserait le
/// renvoi d'un dossier que le comité vient lui-même de réclamer, et
/// l'organisation se retrouverait bloquée devant un écran affichant
/// « 1 point à corriger » et aucune issue (écart n° 38).
///
/// **Le plafond, lui, vaut dans les deux cas** : il compte des dossiers, pas
/// des envois.
pub fn classer(
    appel: &EtatDeLAppel,
    organisation: EtatDeLOrganisation,
    premier_depot: bool,
    maintenant: OffsetDateTime,
) -> Recevabilite {
    if premier_depot
        && (appel.statut != "open" || maintenant < appel.ouvre_le || maintenant > appel.echeance)
    {
        return Recevabilite::CallClosed {
            deadline: appel.echeance,
        };
    }

    if let Some(max) = appel.plafond_par_organisation {
        if organisation.dossiers_comptes >= i64::from(max) {
            return Recevabilite::QuotaReached { max };
        }
    }

    if appel.exige_organisation_verifiee && !organisation.verifiee {
        return Recevabilite::OrganizationNotVerified;
    }

    Recevabilite::Recevable
}
