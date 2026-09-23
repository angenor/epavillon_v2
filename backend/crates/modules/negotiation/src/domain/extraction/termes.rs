//! Un italique n'est un terme touchable que s'il est anglais : sur le guide de la
//! CdP30, 58 % des italiques maigres sont des titres d'ouvrage, du latin ou de
//! l'emphase. Le gras italique, lui, est un titre, jamais un terme.

const MOTS_VIDES_ANGLAIS: [&str; 14] = [
    "of", "on", "the", "and", "for", "to", "with", "from", "by", "at", "into", "its", "is", "are",
];
const MOTS_VIDES_FRANCAIS: [&str; 22] = [
    "le", "la", "les", "de", "des", "du", "à", "en", "et", "un", "une", "sur", "pour", "par",
    "dans", "au", "aux", "ou", "voir", "ce", "ci", "l",
];

/// Un mot vide anglais ; ou deux mots capitalisés au moins, sans accent ni mot
/// vide français. Ni chiffre ni guillemet, douze mots au plus.
pub fn anglais(texte: &str) -> bool {
    let texte = texte
        .trim()
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
    let mots: Vec<&str> = texte
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| !m.is_empty())
        .collect();
    if mots.is_empty()
        || mots.len() > 12
        || texte
            .chars()
            .any(|c| c.is_ascii_digit() || "«»“”\"".contains(c))
    {
        return false;
    }
    let bas: Vec<String> = mots.iter().map(|m| m.to_lowercase()).collect();
    if bas
        .iter()
        .any(|m| MOTS_VIDES_FRANCAIS.contains(&m.as_str()))
    {
        return false;
    }
    if bas.iter().any(|m| MOTS_VIDES_ANGLAIS.contains(&m.as_str())) {
        return true;
    }
    let capitalises = mots.iter().filter(|m| m.chars().count() >= 2).count() >= 2
        && mots
            .iter()
            .all(|m| m.chars().next().is_some_and(char::is_uppercase));
    texte.is_ascii() && capitalises
}

#[cfg(test)]
mod tests {
    use super::anglais;

    #[test]
    fn les_vrais_termes_du_guide_sont_anglais() {
        for terme in [
            "Global Goal on Adaptation",
            "Global Stocktake",
            "Like Minded Developing Countries",
            "Santiago network for Loss and Damage",
            "Adaptation Benefits Mechanism (ABM),",
        ] {
            assert!(anglais(terme), "{terme}");
        }
    }

    #[test]
    fn les_faux_termes_ne_le_sont_pas() {
        for faux in [
            "Le poids du temps",
            "« l’Accord de Paris a fait la différence »",
            "Guide",
            "a minima",
            "ad hoc",
            "voir ci-après",
            "Implementation of Article 9, paragraph 1, of the Paris Agreement",
        ] {
            assert!(!anglais(faux), "{faux}");
        }
    }
}
