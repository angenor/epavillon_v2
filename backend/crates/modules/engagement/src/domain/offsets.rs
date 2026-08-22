//! Les décalages traversent **en minutes**, dans les deux sens.
//!
//! # Pourquoi des minutes et non un intervalle
//!
//! C'est le contrat du front, et son commentaire dit pourquoi : *« en MINUTES et
//! non en texte : `'1 day'` et `'24 hours'` sont le même intervalle pour la base
//! et deux chaînes différentes pour un `Map` »* — ce qui suffirait à afficher
//! deux fois le même rappel.
//!
//! Cela évite aussi de traverser `interval[]`, dont la représentation binaire ne
//! se lit pas à l'œil dans un test rouge.
//!
//! # La conversion vit dans le SQL, pas ici
//!
//! En lecture : `extract(epoch FROM o) / 60`. En écriture :
//! `make_interval(mins => m)`. Ce fichier porte la **règle**, ses bornes et ses
//! tests ; les requêtes qui l'appliquent vivent dans `repo/`.

/// Les bornes de `engagement.are_offsets_valid()` : de un à huit décalages,
/// tous strictement positifs. Elles sont **traduites** ici, jamais revérifiées :
/// le refus reste celui de `ck_reminder_rules_offsets`, et ces constantes
/// servent à le nommer avant qu'il ne tombe.
pub const MIN_DECALAGES: usize = 1;
pub const MAX_DECALAGES: usize = 8;

/// Le défaut du modèle : 2 jours, 1 jour, 1 heure, 30 minutes. **Cumulés** — ce
/// n'est pas un choix parmi quatre, les quatre rappels partent.
pub const DEFAUT_MINUTES: &[i32] = &[2880, 1440, 60, 30];

/// Les décalages sont-ils recevables ?
///
/// Le doublon est refusé : deux fois « 1 jour » produirait deux rappels
/// identiques que `ux_scheduled_reminders_once` réduirait silencieusement à un —
/// l'administrateur croirait avoir programmé cinq envois et en verrait quatre.
pub fn sont_valides(minutes: &[i32]) -> bool {
    if !(MIN_DECALAGES..=MAX_DECALAGES).contains(&minutes.len()) {
        return false;
    }
    if minutes.iter().any(|m| *m <= 0) {
        return false;
    }
    let mut tries = minutes.to_vec();
    tries.sort_unstable();
    tries.dedup();
    tries.len() == minutes.len()
}

/// Rangés du plus lointain au plus proche — l'ordre dans lequel le modèle
/// écrit son défaut, et celui dans lequel l'écran les lit.
pub fn ranges(minutes: &[i32]) -> Vec<i32> {
    let mut tries = minutes.to_vec();
    tries.sort_unstable_by(|a, b| b.cmp(a));
    tries
}

/// Minutes → secondes, la forme que `make_interval` et `extract(epoch)`
/// manipulent. Un `i64` : huit décalages de plusieurs jours dépassent la borne
/// d'un `i32` en secondes bien avant celle du modèle.
pub fn en_secondes(minutes: i32) -> i64 {
    i64::from(minutes) * 60
}

/// Secondes → minutes, arrondi **au plus proche**.
///
/// Un intervalle posé à la main en base peut porter des secondes : les tronquer
/// ferait d'un rappel à 90 secondes un rappel à 1 minute, et l'écran afficherait
/// « 1 minute avant » pour un envoi qui part une minute et demie plus tôt.
pub fn depuis_secondes(secondes: f64) -> i32 {
    (secondes / 60.0).round() as i32
}

/// Le délai tel qu'un destinataire le lit — la variable `delai` du type
/// `programme.session.reminder`.
///
/// **Arrondi vers l'unité déclarée, jamais composé** : « dans 2 jours » et non
/// « dans 2 jours et 0 heure ». Un rappel est une phrase, pas un chronomètre.
/// Les deux langues sont écrites ici parce que ce texte **n'est pas une
/// traduction d'interface** : il ne s'affiche dans aucun écran, et il devient
/// une donnée le jour où un modèle publié le remplace.
pub fn libelle_delai(minutes: i32, anglais: bool) -> String {
    let (nombre, unite) = if minutes % 1440 == 0 {
        (minutes / 1440, if anglais { "day" } else { "jour" })
    } else if minutes % 60 == 0 {
        (minutes / 60, if anglais { "hour" } else { "heure" })
    } else {
        // « minute » s'écrit pareil dans les deux langues ; seul le pluriel
        // et le mot d'introduction changent, et ils sont traités plus bas.
        (minutes, "minute")
    };

    let pluriel = if nombre > 1 { "s" } else { "" };
    if anglais {
        format!("in {nombre} {unite}{pluriel}")
    } else {
        format!("dans {nombre} {unite}{pluriel}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_delai_se_lit_dans_lunite_declaree() {
        assert_eq!(libelle_delai(2880, false), "dans 2 jours");
        assert_eq!(libelle_delai(1440, false), "dans 1 jour");
        assert_eq!(libelle_delai(60, false), "dans 1 heure");
        assert_eq!(libelle_delai(30, false), "dans 30 minutes");
        assert_eq!(libelle_delai(90, false), "dans 90 minutes");
        assert_eq!(libelle_delai(2880, true), "in 2 days");
        assert_eq!(libelle_delai(60, true), "in 1 hour");
    }

    #[test]
    fn le_defaut_du_modele_est_valide() {
        assert!(sont_valides(DEFAUT_MINUTES));
    }

    #[test]
    fn les_bornes_du_modele_sont_traduites() {
        assert!(!sont_valides(&[]));
        assert!(sont_valides(&[30]));
        assert!(sont_valides(&[1, 2, 3, 4, 5, 6, 7, 8]));
        assert!(!sont_valides(&[1, 2, 3, 4, 5, 6, 7, 8, 9]));
    }

    #[test]
    fn un_decalage_nul_ou_negatif_est_refuse() {
        assert!(!sont_valides(&[0]));
        assert!(!sont_valides(&[60, -30]));
    }

    /// Le doublon serait absorbé par la clé d'unicité du modèle, sans rien dire.
    #[test]
    fn un_decalage_repete_est_refuse() {
        assert!(!sont_valides(&[1440, 60, 1440]));
    }

    #[test]
    fn les_decalages_sortent_du_plus_lointain_au_plus_proche() {
        assert_eq!(ranges(&[30, 2880, 60, 1440]), vec![2880, 1440, 60, 30]);
    }

    #[test]
    fn la_conversion_fait_laller_et_le_retour() {
        for m in DEFAUT_MINUTES {
            assert_eq!(depuis_secondes(en_secondes(*m) as f64), *m);
        }
    }

    #[test]
    fn les_secondes_orphelines_sont_arrondies_au_plus_proche() {
        assert_eq!(depuis_secondes(90.0), 2);
        assert_eq!(depuis_secondes(89.0), 1);
    }
}
