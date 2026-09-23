//! Des lignes aux blocs : titres, paragraphes, éléments de liste, notes,
//! zones d'origine ; et, dans les blocs, les segments typés.

use std::collections::HashSet;

use crate::{
    forme::{Block, Span},
    lignes::{est_exposant, est_gras, Ligne, TIRET_DE_FIN},
    zones::Page,
};

/// Les mots vus en milieu de ligne dans tout le document : c'est ce qui dit si
/// « négo-⏎ciation » se recolle, et si « Convention-⏎cadre » garde son tiret.
pub struct Lexique {
    mots: HashSet<String>,
}

#[derive(Default)]
pub struct Journal {
    pub cesures: Vec<(usize, String, bool)>,
    pub termes: Vec<(usize, String, bool)>,
}

fn mots_de(texte: &str) -> impl Iterator<Item = String> + '_ {
    texte
        .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '\u{2011}'))
        .filter(|m| m.chars().count() > 1)
        .map(str::to_lowercase)
}

impl Lexique {
    pub fn depuis(pages: &[Page]) -> Self {
        let mut mots = HashSet::new();
        for page in pages {
            for l in page.corps.iter().chain(page.notes.iter()) {
                let texte = l.texte();
                let interieur = texte.trim_end().trim_end_matches(TIRET_DE_FIN);
                let tokens: Vec<&str> = interieur.split_whitespace().collect();
                // Le dernier mot peut être coupé : il n'atteste rien.
                for t in tokens.iter().take(tokens.len().saturating_sub(1)) {
                    mots.extend(mots_de(t));
                }
            }
        }
        Lexique { mots }
    }

    fn atteste(&self, mot: &str) -> bool {
        self.mots.contains(&mot.to_lowercase())
    }
}

const PUCES: [&str; 8] = ["▪", "•", "■", "◦", "o", "-", "–", "➢"];

/// La profondeur d'un numéro de titre : « 3.6.1. » → 3, « A.2. » → 2,
/// « 3.1.L'adoption » → 2, « II. » → 2.
pub fn numero_de_titre(texte: &str) -> Option<usize> {
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
fn a_points_de_conduite(texte: &str) -> bool {
    texte.contains(".....") || texte.contains("…..")
}

/// Titre : un corps nettement plus grand, ou une ligne toute en gras un peu
/// plus grande que le texte ou numérotée, ou une ligne courte toute en gras
/// qui ne finit pas une phrase.
fn niveau_de_titre(l: &Ligne, corps: f32, courte: bool) -> Option<u8> {
    let texte = l.texte();
    let texte = texte.trim();
    if texte.is_empty() || texte.chars().count() > 160 {
        return None;
    }
    let taille = l
        .segments
        .iter()
        .filter(|s| !s.texte.trim().is_empty())
        .map(|s| s.taille)
        .fold(0.0, f32::max);
    let numero = numero_de_titre(texte);
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

fn puce(l: &Ligne) -> Option<(String, f32)> {
    let premier = l.segments.iter().find(|s| !s.texte.trim().is_empty())?;
    let tete = premier.texte.trim();
    let suivant = l
        .segments
        .iter()
        .skip_while(|s| !std::ptr::eq(*s, premier))
        .nth(1);
    let isolee = suivant.is_some_and(|s| s.cadre.x0 - premier.cadre.x1 > 3.0)
        || tete.chars().count() == 1 && premier.texte.ends_with(' ');
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
    let corps = jeton.trim_end_matches(['.', ')']);
    let numerote = jeton.len() > corps.len()
        && !corps.is_empty()
        && corps.len() <= 3
        && (corps.chars().all(|c| c.is_ascii_digit())
            || corps.chars().all(|c| c.is_ascii_lowercase()));
    (numerote
        && texte.split_whitespace().count() > 1
        && numero_de_titre(&texte).is_none_or(|p| p < 2))
    .then(|| (jeton.to_owned(), l.cadre.x0))
}

fn exposant(texte: &str) -> String {
    texte
        .chars()
        .map(|c| match c {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            autre => autre,
        })
        .collect()
}

/// Les segments d'une ligne, marque de liste ôtée.
fn segments_de(l: &Ligne, sans_puce: bool) -> Vec<Span> {
    let mut sortie: Vec<Span> = Vec::new();
    let mut precedent_x1: Option<f32> = None;
    let mut a_sauter = sans_puce;
    for s in &l.segments {
        if a_sauter && !s.texte.trim().is_empty() {
            a_sauter = false;
            let tete = s.texte.trim();
            if PUCES.contains(&tete) {
                precedent_x1 = Some(s.cadre.x1);
                continue;
            }
            if let Some((m, reste)) = tete.split_once(' ') {
                if PUCES.contains(&m) {
                    sortie.push(Span {
                        text: reste.to_owned(),
                        italic: s.italique,
                        bold: est_gras(&s.police),
                        term: false,
                    });
                    precedent_x1 = Some(s.cadre.x1);
                    continue;
                }
            }
        }
        let mut texte = s.texte.clone();
        if est_exposant(s, l) && texte.trim().chars().all(|c| c.is_ascii_digit()) {
            texte = exposant(texte.trim());
        }
        let blanc = precedent_x1.is_some_and(|x| s.cadre.x0 - x > 0.25 * s.taille)
            && !sortie
                .last()
                .is_some_and(|d| d.text.ends_with(char::is_whitespace))
            && !texte.starts_with(char::is_whitespace);
        if blanc {
            sortie.push(Span {
                text: " ".into(),
                ..Span::default()
            });
        }
        sortie.push(Span {
            text: texte,
            italic: s.italique,
            bold: est_gras(&s.police),
            term: false,
        });
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

/// Recolle deux lignes. Le tiret de fin marqué par PDFium, ou un tiret
/// ordinaire, se garde sauf si le mot recollé est attesté ailleurs et pas le
/// mot à tiret. Une adresse coupée se recolle sans blanc.
fn joindre(
    bloc: &mut Vec<Span>,
    suite: Vec<Span>,
    lexique: &Lexique,
    page: usize,
    journal: &mut Journal,
) {
    let queue: String = bloc.iter().map(|s| s.text.as_str()).collect();
    let queue = queue.trim_end();
    let tete: String = suite.iter().map(|s| s.text.as_str()).collect();
    let premier_mot = tete.split_whitespace().next().unwrap_or("");
    let dernier_mot = queue.split_whitespace().last().unwrap_or("");

    let coupe = queue.ends_with(TIRET_DE_FIN) || (queue.ends_with('-') && dernier_mot.len() > 1);
    let lie_de = |c: char| c.is_alphanumeric();
    let mut jonction = " ";
    if coupe && premier_mot.starts_with(lie_de) {
        let avant = dernier_mot.trim_end_matches([TIRET_DE_FIN, '-']);
        let avant = avant
            .rsplit(|c: char| !c.is_alphanumeric())
            .next()
            .unwrap_or(avant);
        let apres: String = premier_mot
            .chars()
            .take_while(|c| c.is_alphanumeric())
            .collect();
        let recolle = format!("{avant}{apres}");
        let a_tiret = format!("{avant}-{apres}");
        let garde = !(lexique.atteste(&recolle) && !lexique.atteste(&a_tiret));
        journal.cesures.push((
            page,
            format!(
                "{avant}-⏎{apres} → {}",
                if garde { &a_tiret } else { &recolle }
            ),
            garde,
        ));
        jonction = if garde { "-" } else { "" };
        retirer_fin(bloc);
    } else if (dernier_mot.contains("://") || dernier_mot.starts_with("www."))
        && !premier_mot.is_empty()
        && premier_mot
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "/-_.%?=&#~".contains(c))
    {
        jonction = "";
    }
    rogner_fin(bloc);
    if !jonction.is_empty() {
        bloc.push(Span {
            text: jonction.into(),
            ..Span::default()
        });
    }
    bloc.extend(suite);
}

fn rogner_fin(bloc: &mut Vec<Span>) {
    while let Some(d) = bloc.last_mut() {
        let t = d.text.trim_end().to_owned();
        if t.is_empty() {
            bloc.pop();
        } else {
            d.text = t;
            break;
        }
    }
}

fn retirer_fin(bloc: &mut Vec<Span>) {
    rogner_fin(bloc);
    if let Some(d) = bloc.last_mut() {
        d.text = d.text.trim_end_matches([TIRET_DE_FIN, '-']).to_owned();
    }
}

const MOTS_VIDES_ANGLAIS: [&str; 14] = [
    "of", "on", "the", "and", "for", "to", "with", "from", "by", "at", "into", "its", "is", "are",
];
const MOTS_VIDES_FRANCAIS: [&str; 22] = [
    "le", "la", "les", "de", "des", "du", "à", "en", "et", "un", "une", "sur", "pour", "par",
    "dans", "au", "aux", "ou", "voir", "ce", "ci", "l",
];

/// Un italique n'est un terme que s'il est anglais : un mot vide anglais, ou
/// des mots capitalisés sans accent ni mot vide français.
fn anglais(texte: &str) -> bool {
    let texte = texte
        .trim()
        .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
    let mots: Vec<String> = texte
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| !m.is_empty())
        .map(str::to_owned)
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
    let sans_accent = texte.is_ascii();
    let capitalises = mots.iter().filter(|m| m.chars().count() >= 2).count() >= 2
        && mots
            .iter()
            .all(|m| m.chars().next().is_some_and(char::is_uppercase));
    sans_accent && capitalises
}

/// Fusionne les segments voisins de même style, isole les blancs des bords
/// d'un italique, et marque les termes.
fn finir(spans: Vec<Span>, page: usize, journal: &mut Journal) -> Vec<Span> {
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
        s.text = s.text.split_whitespace().collect::<Vec<_>>().join(" ")
            + if s.text.ends_with(char::is_whitespace) {
                " "
            } else {
                ""
            };
        if s.text.starts_with(' ') || sortie.last().is_some_and(|d: &Span| d.text.ends_with(' ')) {
            s.text = s.text.trim_start().to_owned();
            if !sortie.last().is_some_and(|d| d.text.ends_with(' ')) && !sortie.is_empty() {
                sortie.push(Span {
                    text: " ".into(),
                    ..Span::default()
                });
            }
        }
        if (s.italic || s.bold) && !s.text.trim().is_empty() {
            let fin_blanche = s.text.ends_with(' ');
            s.text = s.text.trim_end().to_owned();
            if s.italic && !s.bold {
                s.term = anglais(&s.text);
                journal.termes.push((page, s.text.clone(), s.term));
            }
            sortie.push(s);
            if fin_blanche {
                sortie.push(Span {
                    text: " ".into(),
                    ..Span::default()
                });
            }
        } else if !s.text.is_empty() {
            sortie.push(s);
        }
    }
    if let Some(d) = sortie.last_mut() {
        d.text = d.text.trim_end().to_owned();
    }
    sortie.retain(|s| !s.text.is_empty());
    // Deux segments de même style que l'isolement des blancs a séparés.
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

pub fn composer(page: &Page, corps: f32, lexique: &Lexique, journal: &mut Journal) -> Vec<Block> {
    // Où s'insère chaque zone : à la place de sa première ligne dans le flux,
    // sinon avant la première ligne du corps qui la suit.
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
    places.sort();

    let interligne = interligne(&page.corps, corps);
    let page_de_sommaire = page
        .corps
        .iter()
        .filter(|l| a_points_de_conduite(&l.texte()))
        .count()
        >= 5;
    let justifiee = {
        let pleines = page.corps.iter().filter(|l| {
            let d = page
                .corps
                .iter()
                .filter(|a| a.chevauche_en_x(&l.cadre))
                .map(|a| a.cadre.x1)
                .fold(l.cadre.x1, f32::max);
            l.cadre.x1 >= d - 2.0
        });
        pleines.count() as f32 >= 0.6 * page.corps.len() as f32
    };
    // Une ligne justifiée touche la marge droite : celle qui s'arrête avant, sur
    // un point, finit un paragraphe.
    let retrait_de_fin = if justifiee { 3.0 } else { 1.5 * corps };
    let droite_de = |p: &Ligne| {
        page.corps
            .iter()
            .filter(|a| a.chevauche_en_x(&p.cadre) && (a.base - p.base).abs() < 4.0 * interligne)
            .map(|a| a.cadre.x1)
            .fold(p.cadre.x1, f32::max)
    };
    let mut blocs: Vec<Block> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut encours: Option<Encours> = None;
    let mut pile_puces: Vec<f32> = Vec::new();
    let mut precedente: Option<&Ligne> = None;
    let mut suite_x0: Option<f32> = None;
    let mut zones = places.iter().peekable();

    let fermer = |blocs: &mut Vec<Block>,
                  spans: &mut Vec<Span>,
                  encours: &mut Option<Encours>,
                  journal: &mut Journal| {
        let contenu = finir(std::mem::take(spans), page.indice, journal);
        if let Some(e) = encours.take() {
            if !contenu.is_empty() {
                blocs.push(match e {
                    Encours::Titre(niveau, _) => Block::Heading {
                        level: niveau,
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
    };

    for l in &page.corps {
        while let Some(&&(ordre, i)) = zones.peek() {
            if ordre > l.ordre {
                break;
            }
            zones.next();
            fermer(&mut blocs, &mut spans, &mut encours, journal);
            blocs.push(bloc_d_origine(page, i, lexique, journal));
            precedente = None;
        }

        let entree_de_sommaire = page_de_sommaire && l.taille < 1.4 * corps;
        let courte = l.cadre.x1 < droite_de(l) - 2.0 * corps;
        let titre = if entree_de_sommaire {
            None
        } else {
            niveau_de_titre(l, corps, courte)
        };
        let marque = if titre.is_none() && !entree_de_sommaire {
            puce(l)
        } else {
            None
        };
        let ecart = precedente.map_or(0.0, |p| l.base - p.base);
        let seuil = match encours {
            Some(Encours::Titre(_, taille)) => 1.9 * taille,
            _ => 1.25 * interligne,
        };
        let saut = ecart > seuil || precedente.is_some_and(|p| p.groupe != l.groupe);
        let courte_close = precedente.is_some_and(|p| {
            let fin = p.texte();
            let fin = fin.trim_end();
            (p.cadre.x1 < droite_de(p) - retrait_de_fin
                && fin.ends_with(['.', '!', '?', ':', ';', '»', ')']))
                || (entree_de_sommaire && a_points_de_conduite(fin))
        });

        let continue_le_bloc = match (&encours, titre, &marque) {
            (Some(Encours::Titre(_, taille)), Some(_), None) => {
                (l.taille - taille).abs() < 0.6 && !saut && numero_de_titre(&l.texte()).is_none()
            }
            (Some(Encours::Paragraphe), None, None) => {
                // Un retrait suspendu (bibliographie) : la ligne qui revient à la
                // marge ouvre l'entrée suivante.
                let revient = suite_x0.is_some_and(|x| l.cadre.x0 < x - 1.5 * corps && ecart > 0.0);
                !saut && !courte_close && !revient
            }
            (Some(Encours::Element { retrait, .. }), None, None) => {
                !saut && !courte_close && l.cadre.x0 > *retrait + 4.0
            }
            _ => false,
        };

        let mut ligne = segments_de(l, marque.is_some());
        if let Some((m, _)) = &marque {
            oter_marque(&mut ligne, m);
        }
        if continue_le_bloc {
            suite_x0.get_or_insert(l.cadre.x0);
            joindre(&mut spans, ligne, lexique, page.indice, journal);
        } else {
            suite_x0 = None;
            fermer(&mut blocs, &mut spans, &mut encours, journal);
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
                        profondeur: (pile_puces.len().saturating_sub(1)).min(2) as u8,
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
    fermer(&mut blocs, &mut spans, &mut encours, journal);
    for &(_, i) in zones {
        blocs.push(bloc_d_origine(page, i, lexique, journal));
    }
    blocs.extend(notes(page, lexique, journal));
    blocs
}

fn interligne(lignes: &[Ligne], corps: f32) -> f32 {
    let mut ecarts: Vec<f32> = lignes
        .windows(2)
        .map(|p| p[1].base - p[0].base)
        .filter(|e| *e > 0.9 * corps && *e < 2.0 * corps)
        .collect();
    ecarts.sort_by(f32::total_cmp);
    ecarts.get(ecarts.len() / 2).copied().unwrap_or(1.3 * corps)
}

fn bloc_d_origine(page: &Page, i: usize, lexique: &Lexique, journal: &mut Journal) -> Block {
    let (zone, captees) = &page.zones[i];
    let mut ordre: Vec<&Ligne> = captees.iter().collect();
    ordre.sort_by_key(|l| l.ordre);
    let mut texte: Vec<Span> = Vec::new();
    for l in ordre {
        if texte.is_empty() {
            texte = segments_de(l, false);
        } else {
            joindre(
                &mut texte,
                segments_de(l, false),
                lexique,
                page.indice,
                journal,
            );
        }
    }
    let mut texte = finir(texte, page.indice, journal);
    for s in &mut texte {
        s.term = false;
    }
    Block::Origin {
        reason: zone.raison.code(),
        text: texte,
    }
}

/// Une note commence par sa marque : des chiffres en tête de ligne, en plus
/// petit corps ou suivis d'un blanc.
fn notes(page: &Page, lexique: &Lexique, journal: &mut Journal) -> Vec<Block> {
    let mut sortie: Vec<(String, Vec<Span>)> = Vec::new();
    for l in &page.notes {
        let premier = l.segments.iter().find(|s| !s.texte.trim().is_empty());
        let marque = premier.and_then(|s| {
            let t = s.texte.trim();
            let chiffres: String = t.chars().take_while(char::is_ascii_digit).collect();
            let seul = chiffres.len() == t.len();
            (!chiffres.is_empty()
                && chiffres.len() <= 3
                && (seul || t[chiffres.len()..].starts_with(' ')))
            .then_some(chiffres)
        });
        let mut spans = segments_de(l, false);
        match marque {
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
                Some((_, suite)) => joindre(suite, spans, lexique, page.indice, journal),
                None => sortie.push((String::new(), spans)),
            },
        }
    }
    sortie
        .into_iter()
        .map(|(mark, spans)| Block::Note {
            spans: finir(spans, page.indice, journal),
            mark,
        })
        .filter(|b| !b.texte().trim().is_empty())
        .collect()
}
