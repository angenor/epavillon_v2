//! Des lignes aux blocs : titres, paragraphes, éléments de liste, notes et zones
//! d'origine ; et, dans chaque bloc, les segments typés.

use serde::Serialize;

use super::cesures::{joindre, Lexique};
use super::decoupage::Decoupe;
use super::forme::{Block, OriginReason, Span};
use super::lignes::{est_exposant, est_gras, Ligne, TIRET_DE_FIN};
use super::origine::Raison;
use super::{notes, termes, titres};

/// Ce que la composition a décidé, pour les indicateurs de l'extraction.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize)]
pub struct Releve {
    pub italiques_maigres: usize,
    pub termes: usize,
    pub cesures_gardees: usize,
    pub cesures_recollees: usize,
}

impl Releve {
    fn joindre(&mut self, bloc: &mut Vec<Span>, suite: Vec<Span>, lexique: &Lexique) {
        if let Some(c) = joindre(bloc, suite, lexique) {
            if c.tiret_garde {
                self.cesures_gardees += 1;
            } else {
                self.cesures_recollees += 1;
            }
        }
    }
}

const PUCES: [&str; 8] = ["▪", "•", "■", "◦", "o", "-", "–", "➢"];

/// La marque d'un élément de liste et son abscisse : une puce isolée, une puce
/// en tête de segment, ou un numéro « 1. », « a) ».
fn puce(l: &Ligne) -> Option<(String, f32)> {
    let (rang, premier) = l
        .segments
        .iter()
        .enumerate()
        .find(|(_, s)| !s.texte.trim().is_empty())?;
    let tete = premier.texte.trim();
    let isolee = l
        .segments
        .get(rang + 1)
        .is_some_and(|s| s.cadre.x0 - premier.cadre.x1 > 3.0)
        || (tete.chars().count() == 1 && premier.texte.ends_with(' '));
    let en_tete = tete
        .split_once(' ')
        .map(|(m, _)| m)
        .filter(|m| PUCES.contains(m) && *m != "o");
    if let Some(m) = en_tete.or((PUCES.contains(&tete) && isolee).then_some(tete)) {
        let marque = if matches!(m, "-" | "–") {
            "–"
        } else {
            "•"
        };
        return Some((marque.to_owned(), premier.cadre.x0));
    }
    let texte = l.texte();
    let jeton = texte.split_whitespace().next()?;
    let nu = jeton.trim_end_matches(['.', ')']);
    let numerote = jeton.len() > nu.len()
        && !nu.is_empty()
        && nu.len() <= 3
        && (nu.chars().all(|c| c.is_ascii_digit()) || nu.chars().all(|c| c.is_ascii_lowercase()));
    (numerote
        && texte.split_whitespace().count() > 1
        && titres::numero(&texte).is_none_or(|p| p < 2))
    .then(|| (jeton.to_owned(), l.cadre.x0))
}

fn en_exposant(texte: &str) -> String {
    const EXPOSANTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    texte
        .chars()
        .map(|c| c.to_digit(10).map_or(c, |d| EXPOSANTS[d as usize]))
        .collect()
}

/// Les segments d'une ligne, puce ôtée ; un appel de note devient un chiffre en
/// exposant.
fn segments_de(l: &Ligne, sans_puce: bool) -> Vec<Span> {
    let mut sortie: Vec<Span> = Vec::new();
    let mut precedent_x1: Option<f32> = None;
    let mut a_sauter = sans_puce;
    for s in &l.segments {
        let style = |texte: String| Span {
            text: texte,
            italic: s.italique,
            bold: est_gras(&s.police),
            term: false,
        };
        if a_sauter && !s.texte.trim().is_empty() {
            a_sauter = false;
            let tete = s.texte.trim();
            if PUCES.contains(&tete) {
                precedent_x1 = Some(s.cadre.x1);
                continue;
            }
            if let Some((m, reste)) = tete.split_once(' ') {
                if PUCES.contains(&m) {
                    sortie.push(style(reste.to_owned()));
                    precedent_x1 = Some(s.cadre.x1);
                    continue;
                }
            }
        }
        let mut texte = s.texte.clone();
        if est_exposant(s, l) && texte.trim().chars().all(|c| c.is_ascii_digit()) {
            texte = en_exposant(texte.trim());
        }
        let blanc = precedent_x1.is_some_and(|x| s.cadre.x0 - x > 0.25 * s.taille)
            && !sortie
                .last()
                .is_some_and(|d| d.text.ends_with(char::is_whitespace))
            && !texte.starts_with(char::is_whitespace);
        if blanc {
            sortie.push(Span::simple(" "));
        }
        sortie.push(style(texte));
        precedent_x1 = Some(s.cadre.x1);
    }
    sortie
}

fn oter_marque(spans: &mut [Span], marque: &str) {
    if let Some(s) = spans.iter_mut().find(|s| !s.text.trim().is_empty()) {
        if let Some(reste) = s.text.trim_start().strip_prefix(marque) {
            s.text = reste.trim_start().to_owned();
        }
    }
}

/// Fusionne les segments voisins de même style, sort les blancs des bords d'un
/// segment stylé, et marque les termes anglais.
fn finir(spans: Vec<Span>, releve: &mut Releve) -> Vec<Span> {
    let mut fusion: Vec<Span> = Vec::new();
    for s in spans {
        let texte = s.text.replace(TIRET_DE_FIN, "-");
        match fusion.last_mut() {
            Some(d) if d.italic == s.italic && d.bold == s.bold => d.text.push_str(&texte),
            Some(d) if texte.trim().is_empty() => d.text.push_str(&texte),
            _ => fusion.push(Span { text: texte, ..s }),
        }
    }
    let mut sortie: Vec<Span> = Vec::new();
    for mut s in fusion {
        let fin_blanche = s.text.ends_with(char::is_whitespace);
        s.text = s.text.split_whitespace().collect::<Vec<_>>().join(" ")
            + if fin_blanche { " " } else { "" };
        if s.text.starts_with(' ') || sortie.last().is_some_and(|d: &Span| d.text.ends_with(' ')) {
            s.text = s.text.trim_start().to_owned();
            if !sortie.is_empty() && !sortie.last().is_some_and(|d| d.text.ends_with(' ')) {
                sortie.push(Span::simple(" "));
            }
        }
        if (s.italic || s.bold) && !s.text.trim().is_empty() {
            s.text = s.text.trim_end().to_owned();
            if s.italic && !s.bold {
                s.term = termes::anglais(&s.text);
                releve.italiques_maigres += 1;
                releve.termes += usize::from(s.term);
            }
            sortie.push(s);
            if fin_blanche {
                sortie.push(Span::simple(" "));
            }
        } else if !s.text.is_empty() {
            sortie.push(s);
        }
    }
    if let Some(d) = sortie.last_mut() {
        d.text = d.text.trim_end().to_owned();
    }
    sortie.retain(|s| !s.text.is_empty());
    let mut propre: Vec<Span> = Vec::new();
    for s in sortie {
        match propre.last_mut() {
            Some(d) if !d.italic && !d.bold && !s.italic && !s.bold => d.text.push_str(&s.text),
            _ => propre.push(s),
        }
    }
    propre
}

enum Encours {
    Titre(u8, f32),
    Paragraphe,
    Element {
        marque: String,
        profondeur: u8,
        retrait: f32,
    },
}

fn fermer(
    blocs: &mut Vec<Block>,
    spans: &mut Vec<Span>,
    encours: &mut Option<Encours>,
    releve: &mut Releve,
) {
    let contenu = finir(std::mem::take(spans), releve);
    if let (Some(e), false) = (encours.take(), contenu.is_empty()) {
        blocs.push(match e {
            Encours::Titre(level, _) => Block::Heading {
                level,
                spans: contenu,
            },
            Encours::Paragraphe => Block::Paragraph { spans: contenu },
            Encours::Element {
                marque, profondeur, ..
            } => Block::ListItem {
                spans: contenu,
                depth: profondeur,
                marker: marque,
            },
        });
    }
}

/// L'interligne courant de la page : la médiane des écarts de base plausibles.
fn interligne(lignes: &[Ligne], corps: f32) -> f32 {
    let mut ecarts: Vec<f32> = lignes
        .windows(2)
        .map(|p| p[1].base - p[0].base)
        .filter(|e| *e > 0.9 * corps && *e < 2.0 * corps)
        .collect();
    ecarts.sort_by(f32::total_cmp);
    ecarts.get(ecarts.len() / 2).copied().unwrap_or(1.3 * corps)
}

/// Où s'insère chaque zone : à la place de sa première ligne dans le flux, ou,
/// flottante, avant la première ligne du corps qui la suit.
fn places_des_zones(page: &Decoupe) -> Vec<(usize, usize)> {
    let mut places: Vec<(usize, usize)> = page
        .zones
        .iter()
        .enumerate()
        .map(|(i, (zone, captees))| {
            let premiere = captees.iter().map(|l| l.ordre).min();
            let suivante = page
                .corps
                .iter()
                .find(|l| l.cadre.y0 >= zone.cadre.y0 - 1.0 && l.chevauche_en_x(&zone.cadre))
                .map(|l| l.ordre);
            let ordre = match (premiere, suivante) {
                (Some(_), Some(s)) if captees.iter().all(|l| l.ordre > s) => s,
                (Some(p), _) => p,
                (None, Some(s)) => s,
                (None, None) => usize::MAX,
            };
            (ordre, i)
        })
        .collect();
    places.sort_unstable();
    places
}

/// Un paragraphe se ferme sur un écart de plus de 1,25 interligne ; sur une
/// ligne qui s'arrête avant la marge droite en finissant une phrase ; sur une
/// puce ; sur le retour à la marge d'un retrait suspendu.
pub fn composer(page: &Decoupe, corps: f32, lexique: &Lexique, releve: &mut Releve) -> Vec<Block> {
    let interligne = interligne(&page.corps, corps);
    let page_de_sommaire = page
        .corps
        .iter()
        .filter(|l| titres::a_points_de_conduite(&l.texte()))
        .count()
        >= 5;
    let justifiee = page
        .corps
        .iter()
        .filter(|l| {
            let droite = page
                .corps
                .iter()
                .filter(|a| a.chevauche_en_x(&l.cadre))
                .map(|a| a.cadre.x1)
                .fold(l.cadre.x1, f32::max);
            l.cadre.x1 >= droite - 2.0
        })
        .count() as f32
        >= 0.6 * page.corps.len() as f32;
    // Une ligne justifiée touche la marge : celle qui s'arrête avant, sur un
    // point, finit un paragraphe.
    let retrait_de_fin = if justifiee { 3.0 } else { 1.5 * corps };
    let droite_de = |p: &Ligne| {
        page.corps
            .iter()
            .filter(|a| a.chevauche_en_x(&p.cadre) && (a.base - p.base).abs() < 4.0 * interligne)
            .map(|a| a.cadre.x1)
            .fold(p.cadre.x1, f32::max)
    };

    let places = places_des_zones(page);
    let mut zones = places.iter().peekable();
    let mut blocs: Vec<Block> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut encours: Option<Encours> = None;
    let mut pile_puces: Vec<f32> = Vec::new();
    let mut precedente: Option<&Ligne> = None;
    let mut suite_x0: Option<f32> = None;

    for l in &page.corps {
        while let Some(&&(ordre, i)) = zones.peek() {
            if ordre > l.ordre {
                break;
            }
            zones.next();
            fermer(&mut blocs, &mut spans, &mut encours, releve);
            blocs.push(bloc_d_origine(page, i, lexique, releve));
            precedente = None;
        }

        let entree_de_sommaire = page_de_sommaire && l.taille < 1.4 * corps;
        let courte = l.cadre.x1 < droite_de(l) - 2.0 * corps;
        let titre = (!entree_de_sommaire)
            .then(|| titres::niveau(l, corps, courte))
            .flatten();
        let marque = (titre.is_none() && !entree_de_sommaire)
            .then(|| puce(l))
            .flatten();
        let ecart = precedente.map_or(0.0, |p| l.base - p.base);
        let seuil = match encours {
            Some(Encours::Titre(_, taille)) => 1.9 * taille,
            _ => 1.25 * interligne,
        };
        let saut = ecart > seuil || precedente.is_some_and(|p| p.groupe != l.groupe);
        let close = precedente.is_some_and(|p| {
            let fin = p.texte();
            let fin = fin.trim_end();
            (p.cadre.x1 < droite_de(p) - retrait_de_fin
                && fin.ends_with(['.', '!', '?', ':', ';', '»', ')']))
                || (entree_de_sommaire && titres::a_points_de_conduite(fin))
        });

        let continue_le_bloc = match (&encours, titre, &marque) {
            (Some(Encours::Titre(_, taille)), Some(_), None) => {
                (l.taille - taille).abs() < 0.6 && !saut && titres::numero(&l.texte()).is_none()
            }
            (Some(Encours::Paragraphe), None, None) => {
                let revient = suite_x0.is_some_and(|x| l.cadre.x0 < x - 1.5 * corps && ecart > 0.0);
                !saut && !close && !revient
            }
            (Some(Encours::Element { retrait, .. }), None, None) => {
                !saut && !close && l.cadre.x0 > *retrait + 4.0
            }
            _ => false,
        };

        let mut ligne = segments_de(l, marque.is_some());
        if let Some((m, _)) = &marque {
            oter_marque(&mut ligne, m);
        }
        if continue_le_bloc {
            suite_x0.get_or_insert(l.cadre.x0);
            releve.joindre(&mut spans, ligne, lexique);
        } else {
            suite_x0 = None;
            fermer(&mut blocs, &mut spans, &mut encours, releve);
            encours = Some(match (titre, marque) {
                (Some(n), _) => Encours::Titre(n, l.taille),
                (None, Some((m, x))) => {
                    while pile_puces.last().is_some_and(|&p| p > x + 6.0) {
                        pile_puces.pop();
                    }
                    if pile_puces.last().is_none_or(|&p| x > p + 6.0) {
                        pile_puces.push(x);
                    }
                    Encours::Element {
                        marque: m,
                        profondeur: pile_puces.len().saturating_sub(1).min(2) as u8,
                        retrait: x,
                    }
                }
                (None, None) => {
                    pile_puces.clear();
                    Encours::Paragraphe
                }
            });
            spans.append(&mut ligne);
        }
        precedente = Some(l);
    }
    fermer(&mut blocs, &mut spans, &mut encours, releve);
    for &(_, i) in zones {
        blocs.push(bloc_d_origine(page, i, lexique, releve));
    }
    blocs.extend(notes_de(page, lexique, releve));
    blocs
}

fn bloc_d_origine(page: &Decoupe, i: usize, lexique: &Lexique, releve: &mut Releve) -> Block {
    let (zone, captees) = &page.zones[i];
    let mut lignes: Vec<&Ligne> = captees.iter().collect();
    lignes.sort_by_key(|l| l.ordre);
    let mut texte: Vec<Span> = Vec::new();
    for l in lignes {
        if texte.is_empty() {
            texte = segments_de(l, false);
        } else {
            releve.joindre(&mut texte, segments_de(l, false), lexique);
        }
    }
    // Le texte d'une zone n'est pas relevé : ses italiques ne sont pas des termes.
    let mut texte = finir(texte, &mut Releve::default());
    for s in &mut texte {
        s.term = false;
    }
    Block::Origin {
        reason: match zone.raison {
            Raison::Tableau => OriginReason::Table,
            _ => OriginReason::Figure,
        },
        text: texte,
    }
}

/// Une note commence par sa marque ; les lignes suivantes la continuent.
fn notes_de(page: &Decoupe, lexique: &Lexique, releve: &mut Releve) -> Vec<Block> {
    let mut sortie: Vec<(String, Vec<Span>)> = Vec::new();
    for l in &page.notes {
        let mut spans = segments_de(l, false);
        match notes::marque(l) {
            Some(m) => {
                // La marque a pu passer en exposant : on ôte les deux formes.
                if let Some(s) = spans.iter_mut().find(|s| !s.text.trim().is_empty()) {
                    s.text = s
                        .text
                        .trim_start()
                        .trim_start_matches(|c: char| {
                            c.is_ascii_digit() || "⁰¹²³⁴⁵⁶⁷⁸⁹".contains(c)
                        })
                        .to_owned();
                }
                sortie.push((m, spans));
            }
            None => match sortie.last_mut() {
                Some((_, suite)) => releve.joindre(suite, spans, lexique),
                None => sortie.push((String::new(), spans)),
            },
        }
    }
    sortie
        .into_iter()
        .map(|(mark, spans)| Block::Note {
            spans: finir(spans, releve),
            mark,
        })
        .filter(|b| !b.texte().trim().is_empty())
        .collect()
}
