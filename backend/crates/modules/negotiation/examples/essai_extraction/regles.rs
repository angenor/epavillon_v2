//! L'assemblage : pages brutes → lignes → zones → blocs → forme lisible, puis
//! le rapport qui sert à remplir la grille d'`essai-extraction.md`.

use std::{collections::HashMap, fmt::Write as _, fs, path::Path, time::Duration};

use crate::{
    blocs::{self, Journal, Lexique},
    forme::{Block, DocumentReading, OutlineEntry, ReadingPage},
    lignes::{self, Ligne},
    zones::{self, Page},
    Brut,
};

pub struct Mesures {
    pub poids_pdf: u64,
    pub poids_images: u64,
    pub poids_par_page: Vec<u64>,
    pub duree_texte: Duration,
    pub duree_images: Duration,
}

const TEMOINS: [usize; 20] = [
    6, 7, 9, 11, 12, 13, 14, 15, 21, 25, 47, 48, 50, 51, 59, 64, 66, 67, 72, 88,
];

/// Le corps du texte courant : la taille qui porte le plus de caractères.
fn corps_du_document(brut: &Brut) -> f32 {
    let mut compte: HashMap<i32, usize> = HashMap::new();
    for s in brut.pages.iter().flat_map(|p| &p.segments) {
        *compte.entry((s.taille * 10.0).round() as i32).or_default() += s.texte.len();
    }
    compte
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map_or(11.0, |(t, _)| t as f32 / 10.0)
}

pub fn composer(brut: &Brut, mesures: &Mesures, sortie: &Path) -> std::io::Result<()> {
    let corps = corps_du_document(brut);
    let lignes: Vec<(&crate::PageBrute, Vec<Ligne>)> = brut
        .pages
        .iter()
        .map(|p| (p, lignes::construire(&p.segments)))
        .collect();
    let repetees = zones::repetitions(&lignes);
    let pages: Vec<Page> = lignes
        .into_iter()
        .map(|(p, l)| zones::decouper(p, l, &repetees, corps))
        .collect();

    let lexique = Lexique::depuis(&pages);
    let mut journal = Journal::default();
    let blocs_par_page: Vec<Vec<Block>> = pages
        .iter()
        .map(|p| blocs::composer(p, corps, &lexique, &mut journal))
        .collect();

    let a_origine: Vec<bool> = blocs_par_page
        .iter()
        .map(|b| b.iter().any(|b| matches!(b, Block::Origin { .. })))
        .collect();
    let lecture = DocumentReading {
        id: "essai".into(),
        version: "cdp30-2025".into(),
        mode: "reflow",
        page_count: pages.len(),
        outline: sommaire(&pages, &blocs_par_page),
        pages: pages
            .iter()
            .zip(&blocs_par_page)
            .zip(&a_origine)
            .map(|((p, b), origine)| ReadingPage {
                index: p.indice,
                label: p.etiquette.clone(),
                image: origine.then(|| format!("pages/{}.jpg", p.indice)),
                blocks: b.clone(),
            })
            .collect(),
    };
    let json = serde_json::to_vec(&lecture).map_err(std::io::Error::other)?;
    fs::write(sortie.join("forme-lisible.json"), &json)?;

    let rapport = rapport(
        &pages,
        &blocs_par_page,
        &lecture,
        &journal,
        mesures,
        json.len(),
        corps,
    );
    fs::write(sortie.join("rapport.md"), rapport)
}

/// Titres de niveau 1 et 2, et de niveau 3 s'ils sont numérotés.
fn sommaire(pages: &[Page], blocs: &[Vec<Block>]) -> Vec<OutlineEntry> {
    let mut plat: Vec<OutlineEntry> = Vec::new();
    for (page, blocs) in pages.iter().zip(blocs) {
        for b in blocs {
            if let Block::Heading { level, .. } = b {
                let titre = b.texte().split_whitespace().collect::<Vec<_>>().join(" ");
                let numerote = blocs::numero_de_titre(&titre).is_some();
                if *level <= 2 || numerote {
                    plat.push(OutlineEntry {
                        title: titre,
                        level: *level,
                        page_index: page.indice,
                        children: vec![],
                    });
                }
            }
        }
    }
    fn imbriquer(
        plat: &mut std::iter::Peekable<std::vec::IntoIter<OutlineEntry>>,
        niveau: u8,
    ) -> Vec<OutlineEntry> {
        let mut sortie = Vec::new();
        while let Some(e) = plat.peek() {
            if e.level < niveau {
                break;
            }
            let mut e = plat.next().expect("entrée vue");
            e.children = imbriquer(plat, e.level + 1);
            sortie.push(e);
        }
        sortie
    }
    imbriquer(&mut plat.into_iter().peekable(), 1)
}

fn aplatir(sommaire: &[OutlineEntry], sortie: &mut Vec<(u8, usize, String)>) {
    for e in sommaire {
        sortie.push((e.level, e.page_index, e.title.clone()));
        aplatir(&e.children, sortie);
    }
}

fn rendu(b: &Block) -> String {
    let texte: String = b
        .spans()
        .iter()
        .map(|s| {
            if s.term {
                format!("⟦{}⟧", s.text)
            } else if s.italic {
                format!("_{}_", s.text)
            } else if s.bold {
                format!("**{}**", s.text)
            } else {
                s.text.clone()
            }
        })
        .collect();
    match b {
        Block::Heading { level, .. } => format!("[T{level}] {texte}"),
        Block::Paragraph { .. } => format!("[P] {texte}"),
        Block::ListItem { depth, marker, .. } => format!("[{marker}{depth}] {texte}"),
        Block::Note { mark, .. } => format!("[NOTE {mark}] {texte}"),
        Block::Origin { reason, text } => {
            let t: String = text.iter().map(|s| s.text.as_str()).collect();
            format!(
                "[ORIGINE {reason}] ({} car.) {}…",
                t.chars().count(),
                t.chars().take(120).collect::<String>()
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rapport(
    pages: &[Page],
    blocs: &[Vec<Block>],
    lecture: &DocumentReading,
    journal: &Journal,
    m: &Mesures,
    poids_forme: usize,
    corps: f32,
) -> String {
    let mut r = String::new();
    let avec_texte = pages
        .iter()
        .filter(|p| !p.corps.is_empty() || !p.zones.is_empty())
        .count();
    let origines: Vec<(usize, Vec<&'static str>)> = blocs
        .iter()
        .zip(pages)
        .filter_map(|(b, p)| {
            let raisons: Vec<&'static str> = b
                .iter()
                .filter_map(|b| match b {
                    Block::Origin { reason, .. } => Some(*reason),
                    _ => None,
                })
                .collect();
            (!raisons.is_empty()).then_some((p.indice, raisons))
        })
        .collect();
    let poids_origine: u64 = origines.iter().map(|(i, _)| m.poids_par_page[i - 1]).sum();

    let _ = writeln!(r, "# Rapport de l'essai d'extraction\n");
    let _ = writeln!(r, "## Mesures\n");
    let _ = writeln!(r, "- Corps du texte courant : {corps} pt");
    let _ = writeln!(
        r,
        "- Pages : {} ; avec du texte : {avec_texte}",
        pages.len()
    );
    let _ = writeln!(r, "- Poids du PDF : {} octets", m.poids_pdf);
    let _ = writeln!(r, "- Forme lisible brute : {poids_forme} octets");
    let _ = writeln!(
        r,
        "- Pages à bloc `origin` : {} — {:?}",
        origines.len(),
        origines
    );
    let _ = writeln!(
        r,
        "- Images de page : {} octets au total, {} en moyenne",
        m.poids_images,
        m.poids_images / pages.len().max(1) as u64
    );
    let _ = writeln!(
        r,
        "- Images des seules pages à bloc `origin` : {poids_origine} octets"
    );
    let _ = writeln!(
        r,
        "- Durée : texte {:.2} s, images {:.2} s\n",
        m.duree_texte.as_secs_f32(),
        m.duree_images.as_secs_f32()
    );

    let _ = writeln!(r, "## Sommaire repéré\n");
    let mut plat = Vec::new();
    aplatir(&lecture.outline, &mut plat);
    for (niveau, page, titre) in &plat {
        let _ = writeln!(
            r,
            "{}- p. {page} — {titre}",
            "  ".repeat(*niveau as usize - 1)
        );
    }

    let _ = writeln!(r, "\n## Césures ({})\n", journal.cesures.len());
    for (page, c, _) in &journal.cesures {
        let _ = writeln!(r, "- p. {page} : {c}");
    }

    let vrais = journal.termes.iter().filter(|t| t.2).count();
    let _ = writeln!(
        r,
        "\n## Italiques non gras ({}), dont {vrais} termes\n",
        journal.termes.len()
    );
    for (page, t, terme) in &journal.termes {
        let _ = writeln!(r, "- p. {page} {} {t}", if *terme { "TERME" } else { "—" });
    }

    let _ = writeln!(r, "\n## Pages témoins\n");
    for &i in &TEMOINS {
        let (p, b) = (&pages[i - 1], &blocs[i - 1]);
        let _ = writeln!(r, "### Page {i} — étiquette « {} »\n", p.etiquette);
        let ecartees: Vec<String> = p
            .ecartees
            .iter()
            .map(|l| l.texte().trim().to_owned())
            .collect();
        let _ = writeln!(r, "Écartées (en-têtes et pieds) : {ecartees:?}\n");
        for bloc in b {
            let _ = writeln!(r, "{}\n", rendu(bloc));
        }
    }
    r
}
