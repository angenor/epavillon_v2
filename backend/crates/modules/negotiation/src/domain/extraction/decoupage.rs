//! Le découpage d'une page : ce qui est écarté (en-têtes et pieds), ce qui part
//! en zone d'origine (tableaux, figures), les notes, et le corps — où les
//! encadrés flottants reprennent leur place dans le flux.

use std::collections::HashSet;

use super::brut::{Objet, PageBrute};
use super::entetes::{est_decor, est_repetee, Cle, CleDecor};
use super::lignes::Ligne;
use super::origine::{tableaux, zones_flottantes, Raison, Zone};
use super::{notes, pages};

pub struct Decoupe {
    pub indice: usize,
    pub etiquette: String,
    pub corps: Vec<Ligne>,
    /// Tableaux et figures, avec les lignes qu'ils captent.
    pub zones: Vec<(Zone, Vec<Ligne>)>,
    pub notes: Vec<Ligne>,
}

pub fn decouper(
    page: &PageBrute,
    lignes: Vec<Ligne>,
    repetees: &HashSet<Cle>,
    decor: &HashSet<CleDecor>,
    corps: f32,
) -> Decoupe {
    let (ecartees, lignes): (Vec<Ligne>, Vec<Ligne>) = lignes
        .into_iter()
        .partition(|l| est_repetee(l, page.hauteur, repetees));
    let etiquette = pages::etiquette(page.etiquette.as_deref(), &ecartees, page.indice);

    let mut zones: Vec<(Zone, Vec<Ligne>)> = tableaux(&page.objets)
        .into_iter()
        .map(|cadre| {
            (
                Zone {
                    raison: Raison::Tableau,
                    cadre,
                },
                Vec::new(),
            )
        })
        .collect();
    let sans_decor: Vec<Objet> = page
        .objets
        .iter()
        .filter(|o| !est_decor(o, page, decor))
        .copied()
        .collect();
    for zone in zones_flottantes(&sans_decor, &lignes, page.largeur, page.hauteur) {
        match zones
            .iter_mut()
            .find(|(z, _)| z.cadre.proche(&zone.cadre, 0.0))
        {
            Some((z, _)) => z.cadre.unir(&zone.cadre),
            None => zones.push((zone, Vec::new())),
        }
    }

    // Un fond répété touche tout filet : le séparateur se cherche sans le décor.
    let separateur = notes::separateur(&sans_decor, notes::marge_gauche(&lignes), page.hauteur);
    let mut texte = Vec::new();
    let mut bas = Vec::new();
    for l in lignes {
        let (cx, cy) = l.centre();
        if let Some((_, captees)) = zones
            .iter_mut()
            .find(|(z, _)| z.cadre.contient(cx, cy, 2.0))
        {
            captees.push(l);
        } else if separateur.is_some_and(|y| l.cadre.y0 > y) {
            bas.push(l);
        } else {
            texte.push(l);
        }
    }

    let (encadres, zones): (Vec<_>, Vec<_>) = zones
        .into_iter()
        .partition(|(z, _)| z.raison == Raison::Encadre);
    replacer(&mut texte, encadres);

    // Sans filet : les lignes en petit corps qui ferment la page.
    if separateur.is_none() {
        while texte.last().is_some_and(|l| {
            l.taille <= 0.8 * corps
                && l.cadre.y0 > 0.7 * page.hauteur
                && texte
                    .iter()
                    .all(|a| a.taille <= 0.8 * corps || a.cadre.y1 <= l.cadre.y0 + 1.0)
        }) {
            bas.insert(0, texte.pop().expect("dernière ligne"));
        }
    }

    Decoupe {
        indice: page.indice,
        etiquette,
        corps: texte,
        zones,
        notes: bas,
    }
}

/// Les encadrés rejoignent le corps, à la place de la première ligne du corps
/// qui les suit dans leurs abscisses — la fin de page s'il n'y en a pas. Sous
/// une même place, ils se partagent les 999 ordres libres, rangés de haut en
/// bas puis de gauche à droite.
fn replacer(texte: &mut Vec<Ligne>, encadres: Vec<(Zone, Vec<Ligne>)>) {
    let mut places: Vec<(usize, Zone, Vec<Ligne>)> = encadres
        .into_iter()
        .map(|(zone, captees)| {
            let place = texte
                .iter()
                .find(|l| l.cadre.y0 >= zone.cadre.y0 - 1.0 && l.chevauche_en_x(&zone.cadre))
                .map_or(usize::MAX - 1000, |l| l.ordre);
            (place, zone, captees)
        })
        .collect();
    places.sort_by(|(pa, a, _), (pb, b, _)| {
        pa.cmp(pb)
            .then(a.cadre.y0.total_cmp(&b.cadre.y0))
            .then(a.cadre.x0.total_cmp(&b.cadre.x0))
    });
    let mut rang = 0;
    let mut precedente = None;
    for (n, (place, _, mut captees)) in places.into_iter().enumerate() {
        if precedente != Some(place) {
            (precedente, rang) = (Some(place), 0);
        }
        captees.sort_by_key(|l| l.ordre);
        for mut l in captees {
            // Au-delà de 999 lignes, les dernières partagent l'ordre place - 1 :
            // le tri stable garde leur suite.
            l.ordre = place.saturating_sub(999 - rang.min(998));
            l.groupe = n + 1;
            rang += 1;
            texte.push(l);
        }
    }
    texte.sort_by_key(|l| l.ordre);
}

#[cfg(test)]
mod tests {
    use super::super::lignes::{construire, essai::*};
    use super::*;

    #[test]
    fn un_encadre_au_dessus_de_la_premiere_ligne_passe_avant_elle() {
        let lignes = construire(&[
            fin(seg("premier paragraphe", 72.0, 300.0)),
            fin(seg("second paragraphe", 72.0, 500.0)),
            fin(seg("encadré", 72.0, 100.0)),
        ]);
        let page = PageBrute {
            indice: 1,
            etiquette: None,
            largeur: 595.0,
            hauteur: 842.0,
            segments: vec![],
            objets: vec![],
        };
        let d = decouper(&page, lignes, &HashSet::new(), &HashSet::new(), 11.0);
        let textes: Vec<String> = d.corps.iter().map(Ligne::texte).collect();
        assert_eq!(
            textes,
            ["encadré", "premier paragraphe", "second paragraphe"]
        );
    }

    /// Un fond ou une page importée revient à l'identique sur chaque page :
    /// c'est du décor, et le texte posé dessus reste du texte.
    #[test]
    fn un_fond_repete_ne_capte_pas_le_texte() {
        use super::super::brut::{Cadre, GenreObjet};
        use super::super::entetes;
        let fond = Objet {
            genre: GenreObjet::Forme,
            cadre: Cadre::new(0.0, 0.0, 595.0, 842.0),
        };
        let pages: Vec<PageBrute> = (1..=5)
            .map(|indice| PageBrute {
                indice,
                etiquette: None,
                largeur: 595.0,
                hauteur: 842.0,
                segments: vec![],
                objets: vec![fond],
            })
            .collect();
        let lignes = construire(&[
            fin(seg("premier paragraphe", 72.0, 300.0)),
            fin(seg("second paragraphe", 72.0, 500.0)),
        ]);
        let decor = entetes::decor(&pages);
        let d = decouper(&pages[2], lignes.clone(), &HashSet::new(), &decor, 11.0);
        assert!(d.zones.is_empty());
        assert_eq!(d.corps.len(), 2);

        let sans_regle = decouper(&pages[2], lignes, &HashSet::new(), &HashSet::new(), 11.0);
        assert_eq!(
            sans_regle.zones.len(),
            1,
            "sans la règle, le fond avale la page"
        );
    }

    use super::super::blocs::{composer, Releve};
    use super::super::brut::{Cadre, GenreObjet, Segment};
    use super::super::cesures::Lexique;
    use super::super::entetes;
    use super::super::forme::Block;

    fn bout(texte: &str, x0: f32, x1: f32, base: f32) -> Segment {
        let mut s = fin(seg(texte, x0, base));
        s.cadre.x1 = x1;
        s
    }

    fn page_de(indice: usize, objets: Vec<Objet>) -> PageBrute {
        PageBrute {
            indice,
            etiquette: None,
            largeur: 595.0,
            hauteur: 842.0,
            segments: vec![],
            objets,
        }
    }

    fn textes(lignes: &[Ligne]) -> Vec<String> {
        lignes.iter().map(Ligne::texte).collect()
    }

    fn cote_a_cote(dessous: Segment) -> Vec<Ligne> {
        construire(&[
            dessous,
            bout("gauche un", 72.0, 280.0, 300.0),
            bout("gauche deux", 72.0, 280.0, 314.0),
            bout("gauche trois", 72.0, 280.0, 328.0),
            bout("droite un", 320.0, 520.0, 300.0),
            bout("droite deux", 320.0, 520.0, 314.0),
            bout("droite trois", 320.0, 520.0, 328.0),
        ])
    }

    const COTE_A_COTE: [&str; 6] = [
        "gauche un",
        "gauche deux",
        "gauche trois",
        "droite un",
        "droite deux",
        "droite trois",
    ];

    #[test]
    fn deux_encadres_cote_a_cote_au_dessus_dun_paragraphe_ne_se_melent_pas() {
        let lignes = cote_a_cote(bout("paragraphe pleine largeur", 72.0, 520.0, 500.0));
        let d = decouper(
            &page_de(1, vec![]),
            lignes,
            &HashSet::new(),
            &HashSet::new(),
            11.0,
        );
        let mut attendu = COTE_A_COTE.to_vec();
        attendu.push("paragraphe pleine largeur");
        assert_eq!(textes(&d.corps), attendu);

        let blocs = composer(&d, 11.0, &Lexique::default(), &mut Releve::default());
        let paragraphes: Vec<String> = blocs.iter().map(Block::texte).collect();
        assert_eq!(
            paragraphes,
            [
                "gauche un gauche deux gauche trois",
                "droite un droite deux droite trois",
                "paragraphe pleine largeur",
            ]
        );
    }

    /// La seule ligne qui suit les encadrés est la légende d'une figure : aucune
    /// ligne du corps ne leur donne de place, ils ferment la page.
    #[test]
    fn deux_encadres_sans_ligne_qui_les_suive_ne_se_melent_pas() {
        let figure = Objet {
            genre: GenreObjet::Image,
            cadre: Cadre::new(72.0, 600.0, 520.0, 760.0),
        };
        let lignes = cote_a_cote(bout("légende de la figure", 72.0, 520.0, 700.0));
        let d = decouper(
            &page_de(1, vec![figure]),
            lignes,
            &HashSet::new(),
            &HashSet::new(),
            11.0,
        );
        assert_eq!(d.zones.len(), 1);
        assert_eq!(textes(&d.corps), COTE_A_COTE);
    }

    /// L'encadré du haut ne chevauche que celui du bas, rangé avant la première
    /// ligne du corps : il ne prend pas sa place dans celle d'un autre encadré.
    #[test]
    fn un_encadre_qui_ne_surplombe_quun_autre_encadre_ne_deborde_pas_sous_zero() {
        let lignes = construire(&[
            bout("corps un", 72.0, 297.0, 459.0),
            bout("corps deux", 72.0, 297.0, 473.0),
            bout("encadré du bas", 200.0, 447.0, 409.0),
            bout("encadré du haut", 330.0, 528.0, 109.0),
        ]);
        let d = decouper(
            &page_de(1, vec![]),
            lignes,
            &HashSet::new(),
            &HashSet::new(),
            11.0,
        );
        assert_eq!(
            textes(&d.corps),
            [
                "encadré du bas",
                "corps un",
                "corps deux",
                "encadré du haut"
            ]
        );
        assert!(d.corps.iter().all(|l| l.ordre >= 1));
    }

    #[test]
    fn un_encadre_de_plus_de_999_lignes_reste_avant_sa_place() {
        let mut texte = construire(&[bout("corps", 72.0, 520.0, 500.0)]);
        let modele = construire(&[bout("encadré", 72.0, 520.0, 100.0)]).remove(0);
        let captees: Vec<Ligne> = (0..1200)
            .map(|i| {
                let mut l = modele.clone();
                l.segments[0].texte = i.to_string();
                l.ordre = (i + 2) * 1000;
                l
            })
            .collect();
        let zone = Zone {
            raison: Raison::Encadre,
            cadre: Cadre::new(72.0, 90.0, 520.0, 400.0),
        };
        replacer(&mut texte, vec![(zone, captees)]);
        let mut attendu: Vec<String> = (0..1200).map(|i| i.to_string()).collect();
        attendu.push("corps".into());
        assert_eq!(textes(&texte), attendu);
        assert!(texte[..1200].iter().all(|l| (1..1000).contains(&l.ordre)));
    }

    fn fiches(n: usize, objets: &[Objet]) -> Vec<PageBrute> {
        (1..=n).map(|i| page_de(i, objets.to_vec())).collect()
    }

    const FOND: Objet = Objet {
        genre: GenreObjet::Forme,
        cadre: Cadre {
            x0: 0.0,
            y0: 0.0,
            x1: 595.0,
            y1: 842.0,
        },
    };

    /// Chaque fiche pose son propre graphique dans le même cadre du gabarit :
    /// répété, il reste une figure, et ses étiquettes ne passent pas au corps.
    #[test]
    fn dix_fiches_gardent_chacune_leur_graphique() {
        let graphique = Objet {
            genre: GenreObjet::Image,
            cadre: Cadre::new(72.0, 400.0, 520.0, 700.0),
        };
        let logo = Objet {
            genre: GenreObjet::Image,
            cadre: Cadre::new(500.0, 20.0, 560.0, 70.0),
        };
        let pages = fiches(10, &[FOND, logo, graphique]);
        let decor = entetes::decor(&pages);
        let lignes = construire(&[
            bout("Émissions par secteur", 72.0, 300.0, 200.0),
            bout("Le graphique suivant les ventile.", 72.0, 400.0, 300.0),
            bout("2019", 100.0, 130.0, 500.0),
            bout("2020", 100.0, 130.0, 600.0),
        ]);
        let d = decouper(&pages[4], lignes, &HashSet::new(), &decor, 11.0);
        assert_eq!(d.zones.len(), 1);
        assert_eq!(d.zones[0].0.raison, Raison::Figure);
        assert_eq!(textes(&d.zones[0].1), ["2019", "2020"]);
        assert_eq!(d.corps.len(), 2);
    }

    /// Le fond répété touche tout filet : il ne doit pas empêcher de trouver
    /// celui qui sépare les notes.
    #[test]
    fn un_fond_repete_ne_cache_pas_le_filet_des_notes() {
        let filet = Objet {
            genre: GenreObjet::Chemin,
            cadre: Cadre::new(72.0, 660.0, 216.0, 660.6),
        };
        let compter = |objets: &[Objet]| -> usize {
            let pages = fiches(6, objets);
            let decor = entetes::decor(&pages);
            pages
                .iter()
                .map(|p| {
                    let lignes = construire(&[
                        bout("Le texte courant de la page.", 72.0, 520.0, 300.0),
                        taille(bout("1 Une première note.", 72.0, 300.0, 680.0), 9.0),
                        taille(bout("2 Une seconde note.", 72.0, 300.0, 695.0), 9.0),
                    ]);
                    decouper(p, lignes, &HashSet::new(), &decor, 11.0)
                        .notes
                        .len()
                })
                .sum()
        };
        assert_eq!(compter(&[filet]), 12);
        assert_eq!(compter(&[FOND, filet]), 12);
    }
}
