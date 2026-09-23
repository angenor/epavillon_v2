//! Les zones qui ne se recomposent pas — tableaux et figures —, et les encadrés,
//! qui se recomposent à leur place.

use super::brut::{Cadre, GenreObjet, Objet};
use super::lignes::Ligne;
use super::ordre::flottantes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Raison {
    Tableau,
    Figure,
    /// Texte flottant sans autre dessin que son cadre.
    Encadre,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub raison: Raison,
    pub cadre: Cadre,
}

/// Plus de 90 % de la page dans les deux sens : un fond, une page importée.
pub fn pleine_page(c: &Cadre, largeur: f32, hauteur: f32) -> bool {
    c.largeur() > 0.9 * largeur && c.hauteur() > 0.9 * hauteur
}

/// Un filet : horizontal ou vertical, fin.
fn filet(o: &Objet) -> (bool, bool) {
    let (l, h) = (o.cadre.largeur(), o.cadre.hauteur());
    (h < 1.5 && l > 5.0, l < 1.5 && h > 5.0)
}

/// Les cadres qui se touchent, fusionnés jusqu'à stabilité, avec leurs membres.
fn grouper(cadres: Vec<(Cadre, usize)>, marge: f32) -> Vec<(Cadre, Vec<usize>)> {
    let mut groupes: Vec<(Cadre, Vec<usize>)> =
        cadres.into_iter().map(|(c, i)| (c, vec![i])).collect();
    loop {
        let paire = (0..groupes.len()).find_map(|i| {
            (i + 1..groupes.len())
                .find(|&j| groupes[i].0.proche(&groupes[j].0, marge))
                .map(|j| (i, j))
        });
        let Some((i, j)) = paire else { break };
        let (cadre, membres) = groupes.remove(j);
        groupes[i].0.unir(&cadre);
        groupes[i].1.extend(membres);
    }
    groupes
}

/// Un tableau : deux filets fins horizontaux et deux verticaux au moins, qui se
/// touchent.
pub fn tableaux(objets: &[Objet]) -> Vec<Cadre> {
    let filets: Vec<(Cadre, usize)> = objets
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            o.genre == GenreObjet::Chemin && {
                let (h, v) = filet(o);
                h || v
            }
        })
        .map(|(i, o)| (o.cadre, i))
        .collect();
    grouper(filets, 2.0)
        .into_iter()
        .filter(|(cadre, membres)| {
            let (h, v) = membres.iter().fold((0, 0), |(h, v), &i| {
                let (eh, ev) = filet(&objets[i]);
                (h + usize::from(eh), v + usize::from(ev))
            });
            h >= 2 && v >= 2 && cadre.largeur() > 50.0 && cadre.hauteur() > 20.0
        })
        .map(|(cadre, _)| cadre)
        .collect()
}

/// Les zones flottantes, et si elles portent un dessin : une image de plus de
/// 40 pt, ou un aplat qui n'est pas le simple cadre du texte. Les pointillés —
/// des centaines d'images d'un point — ne comptent pas.
pub fn zones_flottantes(
    objets: &[Objet],
    lignes: &[Ligne],
    largeur: f32,
    hauteur: f32,
) -> Vec<Zone> {
    const IMAGE: usize = usize::MAX;
    let mut graines: Vec<(Cadre, usize)> = objets
        .iter()
        .filter(|o| matches!(o.genre, GenreObjet::Image | GenreObjet::Forme))
        .filter(|o| o.cadre.largeur() > 40.0 && o.cadre.hauteur() > 40.0)
        .map(|o| (o.cadre, IMAGE))
        .collect();
    graines.extend(
        flottantes(lignes, hauteur)
            .into_iter()
            .map(|i| (lignes[i].cadre, i)),
    );

    grouper(graines, 20.0)
        .into_iter()
        .map(|(mut cadre, membres)| {
            let texte = cadre;
            let mut dessin = membres.contains(&IMAGE);
            for o in objets
                .iter()
                .filter(|o| o.genre != GenreObjet::Image || o.cadre.largeur() > 3.0)
            {
                if pleine_page(&o.cadre, largeur, hauteur) || !cadre.proche(&o.cadre, 12.0) {
                    continue;
                }
                let aire_texte = texte.largeur() * texte.hauteur();
                let cadre_du_texte = o.cadre.largeur() * o.cadre.hauteur() >= 0.6 * aire_texte;
                if o.cadre.largeur() > 5.0 && o.cadre.hauteur() > 5.0 && !cadre_du_texte {
                    dessin = true;
                }
                cadre.unir(&o.cadre);
            }
            Zone {
                raison: if dessin {
                    Raison::Figure
                } else {
                    Raison::Encadre
                },
                cadre,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::lignes::{construire, essai::*};
    use super::*;

    fn chemin(x0: f32, y0: f32, x1: f32, y1: f32) -> Objet {
        Objet {
            genre: GenreObjet::Chemin,
            cadre: Cadre::new(x0, y0, x1, y1),
        }
    }

    #[test]
    fn une_grille_de_filets_est_un_tableau() {
        let objets = [
            chemin(72.0, 100.0, 520.0, 100.5),
            chemin(72.0, 150.0, 520.0, 150.5),
            chemin(72.0, 200.0, 520.0, 200.5),
            chemin(72.0, 100.0, 72.5, 200.0),
            chemin(300.0, 100.0, 300.5, 200.0),
            chemin(520.0, 100.0, 520.5, 200.0),
        ];
        assert_eq!(
            tableaux(&objets),
            vec![Cadre::new(72.0, 100.0, 520.5, 200.5)]
        );
    }

    #[test]
    fn des_soulignements_ne_sont_pas_un_tableau() {
        let objets = [
            chemin(100.0, 120.0, 300.0, 120.8),
            chemin(100.0, 140.0, 250.0, 140.8),
        ];
        assert!(tableaux(&objets).is_empty());
    }

    #[test]
    fn un_schema_colore_est_une_figure_un_encadre_ne_lest_pas() {
        let lignes = construire(&[
            fin(seg("haut", 72.0, 100.0)),
            fin(seg("bas", 72.0, 500.0)),
            fin(seg("CdP29", 80.0, 300.0)),
            fin(seg("La 29e session de la", 72.0, 330.0)),
            fin(seg("Conférence des Parties", 72.0, 345.0)),
        ]);
        // L'aplat coloré derrière la seule étiquette : un dessin, pas un cadre.
        let aplat = [Objet {
            genre: GenreObjet::Chemin,
            cadre: Cadre::new(75.0, 288.0, 115.0, 305.0),
        }];
        let figure = zones_flottantes(&aplat, &lignes, 595.0, 842.0);
        assert_eq!(figure.len(), 1);
        assert_eq!(figure[0].raison, Raison::Figure);

        let encadre = zones_flottantes(&[], &lignes, 595.0, 842.0);
        assert_eq!(encadre[0].raison, Raison::Encadre);
    }

    /// Une couverture : la photographie est la page, sans texte par-dessus.
    #[test]
    fn une_image_pleine_page_seule_reste_une_figure() {
        let couverture = [Objet {
            genre: GenreObjet::Image,
            cadre: Cadre::new(0.0, 0.0, 595.0, 842.0),
        }];
        let zones = zones_flottantes(&couverture, &[], 595.0, 842.0);
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].raison, Raison::Figure);
    }
}
