//! Le créneau d'une séance naissante — souhait, repli, durée, rangs (R4, R5).
//!
//! # Pourquoi un repli existe, et pourquoi il n'est pas un pis-aller
//!
//! `sessions.starts_at` et `ends_at` sont `NOT NULL`, et `ck_sessions_period`
//! exige une fin strictement postérieure : **une séance sans créneau n'est pas
//! écrivable**. Refuser l'acceptation d'un dossier parce que l'organisation n'a
//! pas proposé d'horaire serait absurde — c'est justement l'arbitrage que
//! l'équipe s'apprête à faire. Le repli est visible dans le panneau « à placer »,
//! et sera déplacé de toute façon.
//!
//! # La conversion se fait EN BASE, jamais ici
//!
//! Ce fichier décide **quel** créneau, pas comment le composer : une heure
//! murale posée sur un jour civil dans le fuseau d'une édition demande la base
//! de fuseaux de PostgreSQL. C'est le patron de B4, et c'est ce qui a fait
//! tomber le formulaire du front sur `Europe/Geneva`.

use time::{Duration, OffsetDateTime, Time};

/// La durée par défaut quand **rien** ne la donne — ni le dossier, ni l'appel.
///
/// Soixante minutes est la valeur que le modèle lui-même retient pour
/// `default_duration_minutes` : reprendre ce nombre plutôt qu'en inventer un
/// autre garde une seule source à la convention.
pub const DUREE_PAR_DEFAUT: i32 = 60;

/// D'où vient le début d'une séance naissante.
///
/// Le service en tire la requête : le souhait s'écrit tel quel, les deux replis
/// se composent en base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Debut {
    /// `proposals.preferred_start_at`, écrit tel quel.
    Souhaite(OffsetDateTime),
    /// Le **premier jour de l'édition** à l'heure d'ouverture quotidienne de
    /// l'appel — composé par la base, dans le fuseau de l'édition.
    PremierJourALHeureDeLAppel(Time),
    /// Sans appel, il n'y a ni heure d'ouverture ni durée par défaut à lire :
    /// c'est le début de l'édition qui sert (R5).
    DebutDeLEdition,
}

/// Décider le début, dans l'ordre du modèle.
pub fn debut(souhaite: Option<OffsetDateTime>, heure_de_lappel: Option<Time>) -> Debut {
    match (souhaite, heure_de_lappel) {
        (Some(instant), _) => Debut::Souhaite(instant),
        (None, Some(heure)) => Debut::PremierJourALHeureDeLAppel(heure),
        (None, None) => Debut::DebutDeLEdition,
    }
}

/// La durée d'une séance naissante, en minutes : celle du dossier, à défaut
/// celle de l'appel, à défaut [`DUREE_PAR_DEFAUT`].
pub fn duree_minutes(du_dossier: Option<i16>, de_lappel: Option<i16>) -> i32 {
    du_dossier
        .or(de_lappel)
        .map(i32::from)
        .filter(|m| *m > 0)
        .unwrap_or(DUREE_PAR_DEFAUT)
}

/// La fin, quand le début est connu.
pub fn fin(debut: OffsetDateTime, duree_minutes: i32) -> OffsetDateTime {
    debut + Duration::minutes(i64::from(duree_minutes))
}

/// Les rangs d'occurrence à créer — `1..=occurrences`.
///
/// `requested_sessions` est `NOT NULL DEFAULT 1` et borné entre 1 et 50 par le
/// modèle ; la borne basse est reprise ici pour qu'une donnée aberrante ne
/// produise **aucune** séance plutôt qu'une exception.
pub fn rangs(occurrences: i16) -> Vec<i16> {
    (1..=occurrences.max(1)).collect()
}

/// L'adresse d'URL de la n-ième occurrence.
///
/// **Le rang n'est suffixé que lorsqu'il y en a plusieurs** : un dossier à une
/// seule séance garde l'adresse de son titre. La collision dans l'édition est
/// traitée ailleurs, par le suffixe numérique de `domain/slug.rs` — la même
/// fonction que pour un dossier (R7).
pub fn adresse(base: &str, rang: i16, occurrences: i16) -> String {
    if occurrences <= 1 {
        base.to_owned()
    } else {
        format!("{base}-{rang}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{datetime, time};

    #[test]
    fn le_creneau_souhaite_prime_sur_tout() {
        let souhaite = datetime!(2027-11-12 14:00 -3);
        assert_eq!(
            debut(Some(souhaite), Some(time!(9:00))),
            Debut::Souhaite(souhaite)
        );
    }

    #[test]
    fn sans_souhait_le_repli_est_lheure_douverture_de_lappel() {
        assert_eq!(
            debut(None, Some(time!(9:30))),
            Debut::PremierJourALHeureDeLAppel(time!(9:30))
        );
    }

    /// Sans appel, il n'y a pas d'heure d'ouverture à lire : c'est le début de
    /// l'édition qui sert.
    #[test]
    fn sans_appel_le_repli_est_le_debut_de_ledition() {
        assert_eq!(debut(None, None), Debut::DebutDeLEdition);
    }

    #[test]
    fn la_duree_suit_le_dossier_puis_lappel_puis_la_convention() {
        assert_eq!(duree_minutes(Some(45), Some(90)), 45);
        assert_eq!(duree_minutes(None, Some(90)), 90);
        assert_eq!(duree_minutes(None, None), DUREE_PAR_DEFAUT);
    }

    /// Une durée nulle ou négative en base ferait échouer `ck_sessions_period`.
    #[test]
    fn une_duree_aberrante_retombe_sur_la_convention() {
        assert_eq!(duree_minutes(Some(0), None), DUREE_PAR_DEFAUT);
    }

    #[test]
    fn la_fin_est_le_debut_plus_la_duree() {
        assert_eq!(
            fin(datetime!(2027-11-12 14:00 -3), 90),
            datetime!(2027-11-12 15:30 -3)
        );
    }

    #[test]
    fn les_rangs_vont_de_un_au_nombre_doccurrences() {
        assert_eq!(rangs(1), vec![1]);
        assert_eq!(rangs(3), vec![1, 2, 3]);
    }

    #[test]
    fn un_nombre_doccurrences_aberrant_produit_quand_meme_une_seance() {
        assert_eq!(rangs(0), vec![1]);
    }

    #[test]
    fn le_rang_ne_se_suffixe_que_sil_y_en_a_plusieurs() {
        assert_eq!(adresse("atelier-mangroves", 1, 1), "atelier-mangroves");
        assert_eq!(adresse("atelier-mangroves", 2, 3), "atelier-mangroves-2");
    }
}
