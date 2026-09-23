//! Les notes de bas de page : ce qui est sous le filet qui les sépare du corps,
//! ou, sans filet, les lignes en petit corps qui ferment la page.

use std::collections::HashMap;

use super::brut::{GenreObjet, Objet};
use super::lignes::Ligne;

/// L'abscisse où commencent le plus de lignes : la marge gauche. Pas la plus
/// petite — une puce peut déborder dans la marge.
pub fn marge_gauche(lignes: &[Ligne]) -> f32 {
    let mut departs: HashMap<i32, usize> = HashMap::new();
    for l in lignes {
        *departs.entry(l.cadre.x0.round() as i32).or_default() += 1;
    }
    departs
        .into_iter()
        .max_by_key(|(x, n)| (*n, -x))
        .map_or(72.0, |(x, _)| x as f32)
}

/// Le filet court et isolé qui part de la marge gauche, dans le bas de la page.
/// Isolé : le bord d'un aplat — une barre de couleur — n'en est pas un.
pub fn separateur(objets: &[Objet], gauche: f32, hauteur: f32) -> Option<f32> {
    objets
        .iter()
        .filter(|o| {
            let horizontal = o.genre == GenreObjet::Chemin && o.cadre.hauteur() < 1.5;
            let isole = !objets.iter().any(|a| {
                !std::ptr::eq(*o, a) && a.cadre.largeur() > 3.0 && o.cadre.proche(&a.cadre, 1.0)
            });
            horizontal
                && isole
                && (80.0..=260.0).contains(&o.cadre.largeur())
                && (o.cadre.x0 - gauche).abs() < 12.0
                && o.cadre.y0 > 0.55 * hauteur
        })
        .map(|o| o.cadre.y0)
        .reduce(f32::min)
}

/// La marque en tête d'une ligne de note : des chiffres seuls, ou suivis d'un
/// blanc.
pub fn marque(l: &Ligne) -> Option<String> {
    let premier = l.segments.iter().find(|s| !s.texte.trim().is_empty())?;
    let t = premier.texte.trim();
    let chiffres: String = t.chars().take_while(char::is_ascii_digit).collect();
    let seul = chiffres.len() == t.len();
    (!chiffres.is_empty() && chiffres.len() <= 3 && (seul || t[chiffres.len()..].starts_with(' ')))
        .then_some(chiffres)
}

#[cfg(test)]
mod tests {
    use super::super::brut::Cadre;
    use super::super::lignes::{construire, essai::*};
    use super::*;

    fn chemin(x0: f32, y0: f32, x1: f32, y1: f32) -> Objet {
        Objet {
            genre: GenreObjet::Chemin,
            cadre: Cadre::new(x0, y0, x1, y1),
        }
    }

    #[test]
    fn le_filet_isole_separe_les_notes() {
        let objets = [chemin(72.0, 740.0, 216.0, 740.6)];
        assert_eq!(separateur(&objets, 72.0, 842.0), Some(740.0));
    }

    #[test]
    fn le_bord_dune_barre_de_couleur_nest_pas_un_separateur() {
        let objets = [
            chemin(72.0, 655.3, 201.0, 655.8),
            chemin(72.0, 655.9, 201.0, 766.9),
        ];
        assert_eq!(separateur(&objets, 72.0, 842.0), None);
    }

    #[test]
    fn la_marge_gauche_ignore_une_puce_qui_deborde() {
        let lignes = construire(&[
            fin(seg("▪", 54.0, 100.0)),
            fin(seg("texte", 72.0, 114.0)),
            fin(seg("texte", 72.0, 128.0)),
        ]);
        assert_eq!(marge_gauche(&lignes), 72.0);
    }

    #[test]
    fn la_marque_dune_note() {
        let note = construire(&[fin(seg("12 Voir [en ligne]", 72.0, 760.0))]);
        assert_eq!(marque(&note[0]).as_deref(), Some("12"));
        // Une année en tête de ligne n'est pas une marque.
        let suite = construire(&[fin(seg("2020 fut une année", 72.0, 770.0))]);
        assert_eq!(marque(&suite[0]), None);
    }
}
