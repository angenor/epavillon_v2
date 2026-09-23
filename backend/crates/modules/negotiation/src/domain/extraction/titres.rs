//! Les titres : un corps nettement plus grand, ou une ligne toute en gras.

use super::lignes::Ligne;

/// La profondeur d'un numéro de titre : « 3.6.1. » → 3, « A.2. » → 2,
/// « 3.1.L'adoption » → 2, « II. » → 2.
pub fn numero(texte: &str) -> Option<usize> {
    let texte = texte.trim_start();
    if let Some((tete, _)) = texte.split_once(". ") {
        if !tete.is_empty() && tete.chars().all(|c| matches!(c, 'I' | 'V' | 'X')) {
            return Some(2);
        }
    }
    let mut car = texte.chars().peekable();
    let mut parties = 0;
    loop {
        let unite = match car.peek() {
            Some(c) if c.is_ascii_digit() => {
                while car.peek().is_some_and(char::is_ascii_digit) {
                    car.next();
                }
                true
            }
            Some(c) if parties == 0 && c.is_ascii_uppercase() => {
                car.next();
                true
            }
            _ => false,
        };
        if !unite || car.next() != Some('.') {
            break;
        }
        parties += 1;
    }
    (parties > 0).then_some(parties)
}

/// Une ligne à points de conduite : une entrée de sommaire imprimé.
pub fn a_points_de_conduite(texte: &str) -> bool {
    texte.contains(".....") || texte.contains("…..")
}

/// Le niveau d'une ligne de titre, ou `None`. `courte` : la ligne s'arrête avant
/// la marge droite.
pub fn niveau(l: &Ligne, corps: f32, courte: bool) -> Option<u8> {
    let texte = l.texte();
    let texte = texte.trim();
    if texte.is_empty() || texte.chars().count() > 160 {
        return None;
    }
    let taille = l.taille_max();
    let numero = numero(texte);
    let grand = taille >= 1.25 * corps;
    let gras = l.grasse();
    let de_corps =
        gras && courte && texte.chars().count() <= 100 && !texte.ends_with(['.', ',', ';']);
    if !(grand || de_corps || (gras && (taille >= 1.08 * corps || numero.is_some()))) {
        return None;
    }
    Some(if taille >= 1.4 * corps {
        1
    } else if let Some(profondeur) = numero {
        let profondeur = if taille < 1.08 * corps {
            profondeur.max(3)
        } else {
            profondeur
        };
        profondeur.clamp(2, 3) as u8
    } else {
        3
    })
}

#[cfg(test)]
mod tests {
    use super::super::lignes::{construire, essai::*};
    use super::*;

    #[test]
    fn la_profondeur_du_numero() {
        assert_eq!(numero("3.6.1. Objectif mondial"), Some(3));
        assert_eq!(numero("A.2. FICHES THÉMATIQUES"), Some(2));
        assert_eq!(numero("3.1.L’adoption de l’ordre du jour"), Some(2));
        assert_eq!(numero("II. Importance du cadre"), Some(2));
        assert_eq!(numero("1992"), None);
        assert_eq!(numero("La CdP30"), None);
    }

    #[test]
    fn le_corps_et_le_gras_font_le_niveau() {
        let chapitre = construire(&[fin(taille(seg("INTRODUCTION GÉNÉRALE", 72.0, 100.0), 16.0))]);
        assert_eq!(niveau(&chapitre[0], 11.0, true), Some(1));

        let section = construire(&[fin(gras(taille(
            seg("3.6. Adaptation", 106.0, 100.0),
            12.0,
        )))]);
        assert_eq!(niveau(&section[0], 11.0, true), Some(2));

        let sous = construire(&[fin(gras(seg("3.6.1. Objectif mondial", 134.0, 100.0)))]);
        assert_eq!(niveau(&sous[0], 11.0, true), Some(3));

        let intertitre = construire(&[fin(gras(seg("Points Clés de la COP29", 72.0, 100.0)))]);
        assert_eq!(niveau(&intertitre[0], 11.0, true), Some(3));
        assert_eq!(
            niveau(&intertitre[0], 11.0, false),
            None,
            "une ligne pleine en gras est un paragraphe"
        );

        let texte = construire(&[fin(seg("Le texte courant.", 72.0, 100.0))]);
        assert_eq!(niveau(&texte[0], 11.0, true), None);
    }

    #[test]
    fn les_points_de_conduite_signent_un_sommaire() {
        assert!(a_points_de_conduite("1.1. Nouvel Objectif ............ 16"));
        assert!(!a_points_de_conduite("Fin de phrase..."));
    }
}
