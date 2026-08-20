//! La règle du sigle — **écart n° 9**, et la seule règle que ce service ajoute
//! au modèle.
//!
//! **Ce n'est pas une entorse au principe VIII.** Le modèle ne porte pas cette
//! règle et ne doit pas la porter : il *pourrait* l'exprimer par
//! `CHECK (NOT has_pavilion OR acronym IS NOT NULL)`, et on s'en abstient
//! **exprès**, parce que l'arbitrage retenu veut qu'une édition sans pavillon
//! reste enregistrable sans sigle — les webinaires du cycle PACO n'en ont pas,
//! et la reprise des données de la v1 en dépend (research.md § R1).
//!
//! **Pourquoi elle compte.** `programme.tg_assign_reference_code()` préfixe le
//! numéro de dossier par le sigle de l'édition, et à défaut par les **huit
//! premiers caractères de l'adresse d'URL** : une édition `cop31-test` sans
//! sigle produit « COP31-TE-00001 », un numéro qu'aucune organisation ne peut
//! épeler au téléphone.

/// Bornes de FR-028. Un sigle est fait pour être dit et écrit à la main.
pub const MIN: usize = 2;
pub const MAX: usize = 12;

/// **Le sigle manque-t-il ?** Vrai quand l'état **résultant** de l'écriture
/// tient un pavillon et ne porte aucun sigle utilisable.
///
/// L'état résultant, et non l'état antérieur : c'est ce qui fait que basculer
/// une édition en « pavillon tenu » sans fournir de sigle est refusé, et que
/// **retirer** le sigle d'une édition à pavillon l'est aussi — le cas qu'on
/// oublie le plus souvent.
pub fn exiger(has_pavilion: bool, acronym: Option<&str>) -> bool {
    has_pavilion && acronym.is_none_or(|s| s.trim().is_empty())
}

/// Le format est-il acceptable ? De 2 à 12 caractères, lettres **ASCII**,
/// chiffres et tiret.
///
/// L'ASCII n'est pas une négligence : le sigle sert de préfixe à un numéro de
/// dossier qu'on épelle au téléphone et qu'on recopie dans un courriel.
/// « COP31é » ne s'épelle pas.
pub fn format_valide(sigle: &str) -> bool {
    let longueur = sigle.chars().count();
    (MIN..=MAX).contains(&longueur) && sigle.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// La valeur **proposée** avec le refus : accents dépliés, non-alphanumériques
/// retirés, majuscules, tronquée à douze (FR-029).
///
/// `None` quand il ne reste pas deux caractères — un titre entièrement composé
/// de signes ou d'une écriture que le dépliage ne couvre pas. Mieux vaut ne
/// rien proposer qu'une valeur que la vérification de format refuserait
/// ensuite.
pub fn proposer(titre_fr: &str) -> Option<String> {
    let mut propose = String::with_capacity(MAX);

    for c in titre_fr.chars().flat_map(char::to_uppercase) {
        match deplier(c) {
            Some(ascii) => propose.push_str(ascii),
            None if c.is_ascii_alphanumeric() => propose.push(c),
            None => continue,
        }
        if propose.len() >= MAX {
            break;
        }
    }

    propose.truncate(MAX);
    (propose.chars().count() >= MIN).then_some(propose)
}

/// Dépliage des majuscules accentuées du latin usuel. Une table plutôt qu'une
/// dépendance de normalisation Unicode : ce qui est traité ici est ce qu'un
/// titre d'édition francophone porte réellement, et une table de trente entrées
/// se relit.
///
/// `to_uppercase` est appliqué **avant** : il rend déjà « É » de « é » et « SS »
/// de « ß ».
fn deplier(c: char) -> Option<&'static str> {
    Some(match c {
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "A",
        'Æ' => "AE",
        'Ç' => "C",
        'È' | 'É' | 'Ê' | 'Ë' => "E",
        'Ì' | 'Í' | 'Î' | 'Ï' => "I",
        'Ñ' => "N",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "O",
        'Œ' => "OE",
        'Ù' | 'Ú' | 'Û' | 'Ü' => "U",
        'Ý' | 'Ÿ' => "Y",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- La règle -----------------------------------------------------------

    #[test]
    fn sans_pavillon_aucun_sigle_nest_exige() {
        assert!(!exiger(false, None));
        assert!(!exiger(false, Some("")));
    }

    #[test]
    fn avec_pavillon_le_sigle_manquant_est_refuse() {
        assert!(exiger(true, None));
        assert!(exiger(true, Some("")));
        assert!(
            exiger(true, Some("   ")),
            "un sigle d'espaces n'en est pas un"
        );
        assert!(!exiger(true, Some("COP31")));
    }

    // --- Le format ----------------------------------------------------------

    #[test]
    fn les_deux_bornes_sont_tenues() {
        assert!(!format_valide("A"), "un caractère : trop court");
        assert!(format_valide("AB"));
        assert!(format_valide("DOUZECARACT"));
        assert!(
            format_valide("DOUZECARACTE"),
            "douze : la borne haute passe"
        );
        assert!(!format_valide("TREIZECARACTE"), "treize : trop long");
    }

    #[test]
    fn le_jeu_de_caracteres_est_ferme() {
        assert!(format_valide("COP31"));
        assert!(format_valide("COP-31"));
        assert!(!format_valide("COP 31"), "l'espace ne s'épelle pas");
        assert!(!format_valide("COP31é"), "l'accent ne se recopie pas");
        assert!(!format_valide("COP_31"));
    }

    // --- La proposition -----------------------------------------------------

    #[test]
    fn la_proposition_deplie_les_accents_et_retire_le_reste() {
        assert_eq!(
            proposer("COP31 — Conférence des Parties").as_deref(),
            Some("COP31CONFERE")
        );
        assert_eq!(proposer("Été à Genève").as_deref(), Some("ETEAGENEVE"));
    }

    #[test]
    fn la_proposition_est_tronquee_a_douze() {
        let propose = proposer("Rendez-vous de la Francophonie").expect("une proposition");
        assert_eq!(propose.chars().count(), MAX);
        assert_eq!(propose, "RENDEZVOUSDE");
    }

    /// Ce qui est proposé doit passer la vérification de format : proposer une
    /// valeur que l'écriture suivante refuserait ferait tourner l'équipe en rond.
    #[test]
    fn ce_qui_est_propose_est_toujours_acceptable() {
        for titre in [
            "COP31 — Conférence des Parties",
            "Été à Genève",
            "Rendez-vous de la Francophonie",
            "PACO 2027",
            "Forum Œcuménique",
        ] {
            let propose = proposer(titre).expect("une proposition");
            assert!(format_valide(&propose), "{titre} → {propose}");
        }
    }

    #[test]
    fn aucune_proposition_possible() {
        assert_eq!(proposer(""), None);
        assert_eq!(proposer("— … !"), None);
        assert_eq!(proposer("A"), None, "un seul caractère ne suffit pas");
        assert_eq!(proposer("日本語"), None, "rien à déplier, rien à proposer");
    }
}
