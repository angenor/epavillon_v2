//! En-têtes et pieds : une ligne qui revient au même endroit, au numéro près,
//! et le décor — un logo, un bandeau — qui revient au même endroit.

use std::collections::{HashMap, HashSet};

use super::brut::{Cadre, GenreObjet, Objet, PageBrute};
use super::lignes::Ligne;
use super::origine::pleine_page;

/// Le texte d'une ligne, une suite de chiffres valant un joker, et sa hauteur à
/// quatre points près.
pub type Cle = (String, i32);

pub fn cle(l: &Ligne) -> Cle {
    let mut texte = String::new();
    for c in l.texte().to_lowercase().chars() {
        let c = if c.is_ascii_digit() { '#' } else { c };
        if !(c == '#' && texte.ends_with('#')) {
            texte.push(c);
        }
    }
    let texte = texte.split_whitespace().collect::<Vec<_>>().join(" ");
    (texte, (l.cadre.y0 / 4.0).round() as i32)
}

/// Dans les 12 % du haut ou du bas de la page.
pub fn en_marge(l: &Ligne, hauteur: f32) -> bool {
    cadre_en_marge(&l.cadre, hauteur)
}

fn cadre_en_marge(c: &Cadre, hauteur: f32) -> bool {
    c.y1 < 0.12 * hauteur || c.y0 > 0.88 * hauteur
}

/// Les lignes répétées sur trois pages et le cinquième du document au moins —
/// et non la moitié : les annexes d'un guide ont souvent leur propre pied.
pub fn repetitions<'a>(pages: impl IntoIterator<Item = (usize, f32, &'a [Ligne])>) -> HashSet<Cle> {
    let mut vues: HashMap<Cle, HashSet<usize>> = HashMap::new();
    let mut nombre = 0;
    for (indice, hauteur, lignes) in pages {
        nombre += 1;
        for l in lignes.iter().filter(|l| en_marge(l, hauteur)) {
            vues.entry(cle(l)).or_default().insert(indice);
        }
    }
    let seuil = (nombre / 5).max(3);
    vues.into_iter()
        .filter(|(_, p)| p.len() >= seuil)
        .map(|(c, _)| c)
        .collect()
}

pub fn est_repetee(l: &Ligne, hauteur: f32, repetees: &HashSet<Cle>) -> bool {
    en_marge(l, hauteur) && repetees.contains(&cle(l))
}

/// Une image ou une forme, et son cadre à quatre points près.
pub type CleDecor = (GenreObjet, [i32; 4]);

/// Seul un objet de la marge — logo, bandeau — ou de toute la page — fond —
/// peut être décor : dans le corps, un même cadre de gabarit porte d'une fiche
/// à l'autre des graphiques différents.
fn cle_decor(o: &Objet, page: &PageBrute) -> Option<CleDecor> {
    let c = o.cadre;
    let place = cadre_en_marge(&c, page.hauteur) || pleine_page(&c, page.largeur, page.hauteur);
    (place && matches!(o.genre, GenreObjet::Image | GenreObjet::Forme)).then(|| {
        (
            o.genre,
            [c.x0, c.y0, c.x1, c.y1].map(|v| (v / 4.0).round() as i32),
        )
    })
}

/// Les images et formes qui reviennent au même endroit, au même seuil que les
/// pieds : un logo d'en-tête n'est pas une figure.
pub fn decor(pages: &[PageBrute]) -> HashSet<CleDecor> {
    let mut vues: HashMap<CleDecor, HashSet<usize>> = HashMap::new();
    for p in pages {
        for c in p.objets.iter().filter_map(|o| cle_decor(o, p)) {
            vues.entry(c).or_default().insert(p.indice);
        }
    }
    let seuil = (pages.len() / 5).max(3);
    vues.into_iter()
        .filter(|(_, p)| p.len() >= seuil)
        .map(|(c, _)| c)
        .collect()
}

pub fn est_decor(o: &Objet, page: &PageBrute, decor: &HashSet<CleDecor>) -> bool {
    cle_decor(o, page).is_some_and(|c| decor.contains(&c))
}

#[cfg(test)]
mod tests {
    use super::super::lignes::{construire, essai::*};
    use super::*;

    fn page(numero: &str, y: f32) -> Vec<Ligne> {
        construire(&[
            fin(seg("Corps du texte", 72.0, 300.0)),
            fin(seg("© GUIDE DES NÉGOCIATIONS, 2025.", 72.0, y)),
            fin(seg(numero, 510.0, y)),
        ])
    }

    #[test]
    fn le_pied_et_son_numero_sont_repetes_au_numero_pres() {
        let pages: Vec<Vec<Ligne>> = (1..=10)
            .map(|i| page(&(i * 7).to_string(), 790.0))
            .collect();
        let repetees = repetitions(
            pages
                .iter()
                .enumerate()
                .map(|(i, l)| (i + 1, 842.0, l.as_slice())),
        );
        let p = &pages[3];
        assert!(
            !est_repetee(&p[0], 842.0, &repetees),
            "le corps n'est pas un pied"
        );
        assert!(est_repetee(&p[1], 842.0, &repetees));
        assert!(
            est_repetee(&p[2], 842.0, &repetees),
            "« 28 » et « 7 » sont le même pied"
        );
    }

    #[test]
    fn un_logo_repete_est_un_decor_une_figure_unique_non() {
        use super::super::brut::Cadre;
        let logo = Objet {
            genre: GenreObjet::Image,
            cadre: Cadre::new(500.0, 20.0, 560.0, 70.0),
        };
        let figure = Objet {
            genre: GenreObjet::Image,
            cadre: Cadre::new(72.0, 300.0, 400.0, 500.0),
        };
        let pages: Vec<PageBrute> = (1..=10)
            .map(|i| PageBrute {
                indice: i,
                etiquette: None,
                largeur: 595.0,
                hauteur: 842.0,
                segments: vec![],
                objets: if i == 4 {
                    vec![logo, figure]
                } else {
                    vec![logo]
                },
            })
            .collect();
        let decor = decor(&pages);
        assert!(est_decor(&logo, &pages[3], &decor));
        assert!(!est_decor(&figure, &pages[3], &decor));
    }

    /// Chaque fiche pose son graphique dans le même cadre du gabarit : répété
    /// dans le corps, il reste une figure ; le logo et le fond restent du décor.
    #[test]
    fn un_graphique_au_meme_cadre_sur_dix_fiches_nest_pas_un_decor() {
        let objet = |genre, x0, y0, x1, y1| Objet {
            genre,
            cadre: Cadre::new(x0, y0, x1, y1),
        };
        let graphique = objet(GenreObjet::Image, 72.0, 400.0, 520.0, 700.0);
        let logo = objet(GenreObjet::Image, 500.0, 20.0, 560.0, 70.0);
        let bandeau = objet(GenreObjet::Forme, 0.0, 790.0, 595.0, 842.0);
        let fond = objet(GenreObjet::Forme, 0.0, 0.0, 595.0, 842.0);
        let pages: Vec<PageBrute> = (1..=10)
            .map(|i| PageBrute {
                indice: i,
                etiquette: None,
                largeur: 595.0,
                hauteur: 842.0,
                segments: vec![],
                objets: vec![fond, logo, bandeau, graphique],
            })
            .collect();
        let decor = decor(&pages);
        let p = &pages[6];
        assert!(!est_decor(&graphique, p, &decor));
        assert!(est_decor(&logo, p, &decor));
        assert!(est_decor(&bandeau, p, &decor));
        assert!(est_decor(&fond, p, &decor));
    }

    #[test]
    fn un_pied_rare_nest_pas_ecarte() {
        let pages = [page("1", 790.0), page("2", 790.0)];
        let repetees = repetitions(
            pages
                .iter()
                .enumerate()
                .map(|(i, l)| (i + 1, 842.0, l.as_slice())),
        );
        assert!(repetees.is_empty());
    }
}
