//! Le code d'invitation : sa forme de comparaison, et sa fabrication.
//!
//! Il circule **recopié à la main** depuis un message WhatsApp, parfois dicté
//! au téléphone. Tout ce qui suit découle de là.

use kernel::crypto;

/// Huit caractères, tirets compris (FR-010). `NEGO-24` de la maquette en porte
/// sept et se corrige.
pub const LONGUEUR: usize = 8;

/// Ni `0`/`O` ni `1`/`I`/`L` : un code se recopie à l'œil, et ces cinq lettres
/// se confondent deux à deux dans presque toutes les polices.
const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";

/// **La même expression que `code_normalized`**, qui est une colonne
/// `GENERATED` : majuscules, sans séparateur. `nego-024`, `NEGO 024` et
/// `Nego024` désignent le même code.
///
/// La recopier ici n'est pas un doublon d'invariant : la base la calcule pour
/// *stocker*, le code la calcule pour *chercher*. Les deux doivent rendre la
/// même chaîne, et un test les compare sur la base réelle plutôt que de le
/// supposer.
pub fn normaliser(saisi: &str) -> String {
    saisi
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Un code neuf, de la forme `XXXX-XXX`.
///
/// **Le préfixe est tiré au sort lui aussi**, alors que la maquette écrit
/// `NEGO-024`. Le figer ne laisserait que trois caractères aléatoires — une
/// trentaine de milliers de possibilités —, et le code est un secret partagé
/// que rien d'autre ne protège : la limite d'essais ralentit une personne, pas
/// un ensemble de comptes. Sept caractères sur l'alphabet retenu en donnent
/// vingt-sept milliards.
pub fn engendrer() -> String {
    let tire = crypto::random_string(ALPHABET, LONGUEUR - 1);
    format!("{}-{}", &tire[..4], &tire[4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_normalisation_ignore_la_casse_et_les_separateurs() {
        assert_eq!(normaliser(" nego-024 "), "NEGO024");
        assert_eq!(normaliser("NEGO 024"), "NEGO024");
        assert_eq!(normaliser("Nego024"), "NEGO024");
    }

    #[test]
    fn le_code_engendre_fait_huit_caracteres_tirets_compris() {
        for _ in 0..50 {
            let code = engendrer();
            assert_eq!(code.chars().count(), LONGUEUR, "{code}");
            assert_eq!(code.chars().nth(4), Some('-'), "{code}");
            assert!(
                code.chars()
                    .all(|c| c == '-' || ALPHABET.contains(&(c as u8))),
                "{code}"
            );
        }
    }
}
