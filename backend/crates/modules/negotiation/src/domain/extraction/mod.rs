//! L'extraction : ce que rend la lecture du PDF, recomposé en forme lisible.
//!
//! Les règles sont celles de R7 (`specs/011-guide-nego-documents/research.md`),
//! éprouvées sur le vrai guide de la CdP30 : un fichier par règle, chacune
//! testée sur des pages construites à la main. **Rien ici ne dépend de PDFium**
//! — `crate::pdf` lit le fichier, ce module ne voit que des segments.

pub mod blocs;
pub mod brut;
pub mod cesures;
pub mod decoupage;
pub mod entetes;
pub mod forme;
pub mod lignes;
pub mod notes;
pub mod ordre;
pub mod origine;
pub mod pages;
pub mod sommaire;
pub mod termes;
pub mod titres;

use std::collections::HashMap;

use serde::Serialize;

use self::blocs::Releve;
use self::brut::DocumentBrut;
use self::cesures::Lexique;
use self::forme::{texte_de_page, Block, OriginReason, OutlineEntry};
use self::lignes::Ligne;

/// Une page recomposée, telle que `negotiation.document_pages` la garde.
#[derive(Debug, Clone, PartialEq)]
pub struct PageExtraite {
    pub index: usize,
    pub label: String,
    pub blocks: Vec<Block>,
    pub plain_text: String,
    /// Un tableau ou une figure : l'image de la page part avec la copie gardée.
    pub has_origin_block: bool,
}

/// Les indicateurs du verdict, affichés à l'aperçu.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Qualite {
    pub pages: usize,
    pub pages_avec_texte: usize,
    pub pages_a_origine: usize,
    pub tableaux: usize,
    pub figures: usize,
    pub notes: usize,
    pub titres: usize,
    pub entrees_du_sommaire: usize,
    pub sommaire_depuis_signets: bool,
    #[serde(flatten)]
    pub releve: Releve,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extraction {
    pub pages: Vec<PageExtraite>,
    pub outline: Vec<OutlineEntry>,
    /// Recomposable : du texte sur neuf pages sur dix, et un tiers de pages
    /// d'origine au plus — au-delà, le document se lit mieux « tel quel ».
    pub is_reflowable: bool,
    pub quality: Qualite,
}

/// Le corps du texte courant : la taille qui porte le plus de caractères.
fn corps_du_document(doc: &DocumentBrut) -> f32 {
    let mut compte: HashMap<i32, usize> = HashMap::new();
    for s in doc.pages.iter().flat_map(|p| &p.segments) {
        *compte.entry((s.taille * 10.0).round() as i32).or_default() += s.texte.chars().count();
    }
    compte
        .into_iter()
        .max_by_key(|&(taille, n)| (n, taille))
        .map_or(11.0, |(t, _)| t as f32 / 10.0)
}

pub fn extraire(doc: &DocumentBrut) -> Extraction {
    let corps = corps_du_document(doc);
    let lignes: Vec<Vec<Ligne>> = doc
        .pages
        .iter()
        .map(|p| lignes::construire(&p.segments))
        .collect();
    let repetees = entetes::repetitions(
        doc.pages
            .iter()
            .zip(&lignes)
            .map(|(p, l)| (p.indice, p.hauteur, l.as_slice())),
    );
    let decor = entetes::decor(&doc.pages);
    let decoupes: Vec<decoupage::Decoupe> = doc
        .pages
        .iter()
        .zip(lignes)
        .map(|(p, l)| decoupage::decouper(p, l, &repetees, &decor, corps))
        .collect();

    let lexique = Lexique::depuis(
        decoupes
            .iter()
            .flat_map(|d| d.corps.iter().chain(&d.notes))
            .map(Ligne::texte)
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str),
    );
    let mut releve = Releve::default();
    let pages: Vec<PageExtraite> = decoupes
        .iter()
        .map(|d| {
            let blocks = blocs::composer(d, corps, &lexique, &mut releve);
            PageExtraite {
                index: d.indice,
                label: d.etiquette.clone(),
                plain_text: texte_de_page(&blocks),
                has_origin_block: blocks.iter().any(|b| matches!(b, Block::Origin { .. })),
                blocks,
            }
        })
        .collect();

    let (outline, depuis_signets) = if doc.signets.iter().any(|s| s.page_indice.is_some()) {
        (sommaire::depuis_signets(&doc.signets), true)
    } else {
        (
            sommaire::depuis_titres(pages.iter().map(|p| (p.index, p.blocks.as_slice()))),
            false,
        )
    };

    let compter = |f: &dyn Fn(&Block) -> bool| {
        pages
            .iter()
            .flat_map(|p| &p.blocks)
            .filter(|b| f(b))
            .count()
    };
    fn entrees(o: &[OutlineEntry]) -> usize {
        o.iter().map(|e| 1 + entrees(&e.children)).sum()
    }
    let quality = Qualite {
        pages: pages.len(),
        pages_avec_texte: pages
            .iter()
            .filter(|p| !p.plain_text.trim().is_empty())
            .count(),
        pages_a_origine: pages.iter().filter(|p| p.has_origin_block).count(),
        tableaux: compter(&|b| {
            matches!(
                b,
                Block::Origin {
                    reason: OriginReason::Table,
                    ..
                }
            )
        }),
        figures: compter(&|b| {
            matches!(
                b,
                Block::Origin {
                    reason: OriginReason::Figure,
                    ..
                }
            )
        }),
        notes: compter(&|b| matches!(b, Block::Note { .. })),
        titres: compter(&|b| matches!(b, Block::Heading { .. })),
        entrees_du_sommaire: entrees(&outline),
        sommaire_depuis_signets: depuis_signets,
        releve,
    };
    let is_reflowable = quality.pages > 0
        && quality.pages_avec_texte * 10 >= quality.pages * 9
        && quality.pages_a_origine * 3 <= quality.pages;

    Extraction {
        pages,
        outline,
        is_reflowable,
        quality,
    }
}
