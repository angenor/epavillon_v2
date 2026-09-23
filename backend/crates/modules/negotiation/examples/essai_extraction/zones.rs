//! Ce qui sort du corps : en-têtes et pieds, tableaux, figures, notes.

use std::collections::{HashMap, HashSet};

use crate::{lignes::Ligne, Cadre, Objet, PageBrute};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Raison {
    Tableau,
    Figure,
    /// Texte flottant sans autre dessin que son cadre : il se recompose.
    Encadre,
}

impl Raison {
    pub fn code(self) -> &'static str {
        match self {
            Raison::Tableau => "table",
            Raison::Figure => "figure",
            Raison::Encadre => "encadre",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Zone {
    pub raison: Raison,
    pub cadre: Cadre,
}

/// Le découpage d'une page, avant la composition des blocs.
pub struct Page {
    pub indice: usize,
    pub etiquette: String,
    pub corps: Vec<Ligne>,
    pub zones: Vec<(Zone, Vec<Ligne>)>,
    pub notes: Vec<Ligne>,
    pub ecartees: Vec<Ligne>,
}

fn cle_repetee(l: &Ligne) -> (String, i32) {
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

fn en_marge(l: &Ligne, hauteur: f32) -> bool {
    l.cadre.y1 < 0.12 * hauteur || l.cadre.y0 > 0.88 * hauteur
}

/// Une ligne qui revient au même endroit, au numéro près, sur trois pages et le
/// cinquième du document au moins : les annexes du guide ont leur propre pied.
pub fn repetitions(pages: &[(&PageBrute, Vec<Ligne>)]) -> HashSet<(String, i32)> {
    let mut vues: HashMap<(String, i32), HashSet<usize>> = HashMap::new();
    for (page, lignes) in pages {
        for l in lignes.iter().filter(|l| en_marge(l, page.hauteur)) {
            vues.entry(cle_repetee(l)).or_default().insert(page.indice);
        }
    }
    let seuil = (pages.len() / 5).max(3);
    vues.into_iter()
        .filter(|(_, p)| p.len() >= seuil)
        .map(|(cle, _)| cle)
        .collect()
}

fn fin(o: &Objet) -> (bool, bool) {
    let (l, h) = (o.cadre.x1 - o.cadre.x0, o.cadre.y1 - o.cadre.y0);
    (h < 1.5 && l > 5.0, l < 1.5 && h > 5.0)
}

fn proches(a: &Cadre, b: &Cadre, marge: f32) -> bool {
    a.x0 - marge <= b.x1 && b.x0 - marge <= a.x1 && a.y0 - marge <= b.y1 && b.y0 - marge <= a.y1
}

/// Les cadres qui se touchent, fusionnés jusqu'à stabilité.
fn grouper(cadres: Vec<(Cadre, usize)>, marge: f32) -> Vec<(Cadre, Vec<usize>)> {
    let mut groupes: Vec<(Cadre, Vec<usize>)> =
        cadres.into_iter().map(|(c, i)| (c, vec![i])).collect();
    loop {
        let mut fusion = None;
        'cherche: for i in 0..groupes.len() {
            for j in i + 1..groupes.len() {
                if proches(&groupes[i].0, &groupes[j].0, marge) {
                    fusion = Some((i, j));
                    break 'cherche;
                }
            }
        }
        let Some((i, j)) = fusion else { break };
        let (cadre, membres) = groupes.remove(j);
        groupes[i].0.unir(&cadre);
        groupes[i].1.extend(membres);
    }
    groupes
}

/// Un tableau : des filets horizontaux et verticaux qui se croisent.
fn tableaux(objets: &[Objet]) -> Vec<Cadre> {
    let filets: Vec<(Cadre, usize)> = objets
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            o.genre == "chemin" && {
                let (h, v) = fin(o);
                h || v
            }
        })
        .map(|(i, o)| (o.cadre.clone(), i))
        .collect();
    grouper(filets, 2.0)
        .into_iter()
        .filter(|(cadre, membres)| {
            let (h, v) = membres.iter().fold((0, 0), |(h, v), &i| {
                let (eh, ev) = fin(&objets[i]);
                (h + eh as usize, v + ev as usize)
            });
            h >= 2 && v >= 2 && cadre.x1 - cadre.x0 > 50.0 && cadre.y1 - cadre.y0 > 20.0
        })
        .map(|(cadre, _)| cadre)
        .collect()
}

/// Une ligne écrite après une autre qu'elle surplombe, dans les mêmes
/// abscisses : une forme flottante, que le flux relègue en fin de page. Les
/// marges ne comptent pas : un pied non répété ouvre souvent le flux.
fn flottantes(lignes: &[Ligne], hauteur: f32) -> Vec<usize> {
    let mut sortie = Vec::new();
    for (i, l) in lignes.iter().enumerate() {
        let surplombe = lignes[..i]
            .iter()
            .filter(|e| !en_marge(e, hauteur))
            .any(|e| e.cadre.y0 > l.cadre.y1 + 2.0 && l.chevauche_en_x(&e.cadre));
        if surplombe {
            sortie.push(i);
        }
    }
    sortie
}

/// Les zones flottantes, et si elles portent un dessin : une image, ou un aplat
/// qui n'est pas le simple cadre du texte.
fn figures(objets: &[Objet], lignes: &[Ligne], largeur: f32, hauteur: f32) -> Vec<(Cadre, bool)> {
    let images: Vec<(Cadre, usize)> = objets
        .iter()
        .filter(|o| o.genre == "image" || o.genre == "forme")
        .filter(|o| o.cadre.x1 - o.cadre.x0 > 40.0 && o.cadre.y1 - o.cadre.y0 > 40.0)
        .map(|o| (o.cadre.clone(), 1))
        .collect();
    let mut graines = images;
    graines.extend(
        flottantes(lignes, hauteur)
            .into_iter()
            .map(|i| (lignes[i].cadre.clone(), 0)),
    );
    let mut sortie = Vec::new();
    for (mut zone, membres) in grouper(graines, 20.0) {
        let mut dessin = membres.contains(&1);
        let texte = zone.clone();
        for o in objets
            .iter()
            .filter(|o| o.genre != "image" || o.cadre.x1 - o.cadre.x0 > 3.0)
        {
            let pleine_page =
                o.cadre.x1 - o.cadre.x0 > 0.9 * largeur && o.cadre.y1 - o.cadre.y0 > 0.9 * hauteur;
            if pleine_page || !proches(&zone, &o.cadre, 12.0) {
                continue;
            }
            let (l, h) = (o.cadre.x1 - o.cadre.x0, o.cadre.y1 - o.cadre.y0);
            let aire_texte = (texte.x1 - texte.x0) * (texte.y1 - texte.y0);
            let cadre_du_texte = l * h >= 0.6 * aire_texte;
            if l > 5.0 && h > 5.0 && !cadre_du_texte {
                dessin = true;
            }
            zone.unir(&o.cadre);
        }
        sortie.push((zone, dessin));
    }
    sortie
}

/// Le filet court et isolé qui sépare les notes du corps, en bas à gauche.
/// Isolé : le bord d'un aplat n'en est pas un.
fn separateur_de_notes(objets: &[Objet], gauche: f32, hauteur: f32) -> Option<f32> {
    objets
        .iter()
        .filter(|o| {
            let (h, _) = fin(o);
            let l = o.cadre.x1 - o.cadre.x0;
            let isole = !objets.iter().any(|a| {
                !std::ptr::eq(*o, a)
                    && a.cadre.x1 - a.cadre.x0 > 3.0
                    && proches(&o.cadre, &a.cadre, 1.0)
            });
            h && isole
                && (80.0..=260.0).contains(&l)
                && (o.cadre.x0 - gauche).abs() < 12.0
                && o.cadre.y0 > 0.55 * hauteur
        })
        .map(|o| o.cadre.y0)
        .reduce(f32::min)
}

pub fn decouper(
    page: &PageBrute,
    lignes: Vec<Ligne>,
    repetees: &HashSet<(String, i32)>,
    corps_du_document: f32,
) -> Page {
    let (ecartees, lignes): (Vec<_>, Vec<_>) = lignes
        .into_iter()
        .partition(|l| en_marge(l, page.hauteur) && repetees.contains(&cle_repetee(l)));
    let etiquette = ecartees
        .iter()
        .map(|l| l.texte().trim().to_owned())
        .find(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or_else(|| page.indice.to_string());

    // La marge gauche : l'abscisse où commencent le plus de lignes, pas la plus
    // petite — une puce peut déborder dans la marge.
    let mut departs: HashMap<i32, usize> = HashMap::new();
    for l in &lignes {
        *departs.entry(l.cadre.x0.round() as i32).or_default() += 1;
    }
    let gauche = departs
        .into_iter()
        .max_by_key(|(x, n)| (*n, -x))
        .map_or(72.0, |(x, _)| x as f32);
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
    for (cadre, dessin) in figures(&page.objets, &lignes, page.largeur, page.hauteur) {
        let deja = zones
            .iter_mut()
            .find(|(z, _)| proches(&z.cadre, &cadre, 0.0));
        let raison = if dessin {
            Raison::Figure
        } else {
            Raison::Encadre
        };
        match deja {
            Some((z, _)) => z.cadre.unir(&cadre),
            None => zones.push((Zone { raison, cadre }, Vec::new())),
        }
    }

    let separateur = separateur_de_notes(&page.objets, gauche, page.hauteur);
    let mut corps = Vec::new();
    let mut notes = Vec::new();
    for l in lignes {
        let (cx, cy) = l.centre();
        if let Some((_, captees)) = zones.iter_mut().find(|(z, _)| {
            cx >= z.cadre.x0 - 2.0
                && cx <= z.cadre.x1 + 2.0
                && cy >= z.cadre.y0 - 2.0
                && cy <= z.cadre.y1 + 2.0
        }) {
            captees.push(l);
        } else if separateur.is_some_and(|y| l.cadre.y0 > y) {
            notes.push(l);
        } else {
            corps.push(l);
        }
    }

    let (encadres, zones): (Vec<_>, Vec<_>) = zones
        .into_iter()
        .partition(|(z, _)| z.raison == Raison::Encadre);
    for (n, (zone, captees)) in encadres.into_iter().enumerate() {
        let place = corps
            .iter()
            .find(|l: &&Ligne| l.cadre.y0 >= zone.cadre.y0 - 1.0 && l.chevauche_en_x(&zone.cadre))
            .map_or(usize::MAX - 1000, |l| l.ordre);
        let mut captees = captees;
        captees.sort_by_key(|l| l.ordre);
        for (k, mut l) in captees.into_iter().enumerate() {
            l.ordre = place - 999 + k;
            l.groupe = n + 1;
            corps.push(l);
        }
    }
    corps.sort_by_key(|l| l.ordre);

    // Sans filet : les lignes en petit corps qui ferment la page.
    if separateur.is_none() {
        while corps.last().is_some_and(|l| {
            l.taille <= 0.8 * corps_du_document
                && l.cadre.y0 > 0.7 * page.hauteur
                && corps
                    .iter()
                    .all(|a| a.taille <= 0.8 * corps_du_document || a.cadre.y1 <= l.cadre.y0 + 1.0)
        }) {
            notes.insert(0, corps.pop().expect("dernière ligne"));
        }
    }

    Page {
        indice: page.indice,
        etiquette,
        corps,
        zones,
        notes,
        ecartees,
    }
}
