//! Le sommaire : les signets du PDF s'il en a ; sinon les titres de niveau 1 et
//! 2, et ceux de niveau 3 qui sont numérotés — les intertitres répétés
//! (« Points de convergence ») ne l'encombrent pas.

use super::brut::Signet;
use super::forme::{Block, OutlineEntry};
use super::titres;

pub fn depuis_signets(signets: &[Signet]) -> Vec<OutlineEntry> {
    imbriquer(
        signets
            .iter()
            .filter_map(|s| {
                let titre = s.titre.split_whitespace().collect::<Vec<_>>().join(" ");
                Some(OutlineEntry {
                    title: titre,
                    level: s.niveau.clamp(1, 3),
                    page_index: s.page_indice?,
                    children: vec![],
                })
            })
            .collect(),
    )
}

pub fn depuis_titres<'a>(
    pages: impl IntoIterator<Item = (usize, &'a [Block])>,
) -> Vec<OutlineEntry> {
    let mut plat = Vec::new();
    for (index, blocs) in pages {
        for b in blocs {
            if let Block::Heading { level, .. } = b {
                let titre = b.texte().split_whitespace().collect::<Vec<_>>().join(" ");
                if *level <= 2 || titres::numero(&titre).is_some() {
                    plat.push(OutlineEntry {
                        title: titre,
                        level: *level,
                        page_index: index,
                        children: vec![],
                    });
                }
            }
        }
    }
    imbriquer(plat)
}

fn imbriquer(plat: Vec<OutlineEntry>) -> Vec<OutlineEntry> {
    fn niveau(
        plat: &mut std::iter::Peekable<std::vec::IntoIter<OutlineEntry>>,
        seuil: u8,
    ) -> Vec<OutlineEntry> {
        let mut sortie = Vec::new();
        while let Some(e) = plat.peek() {
            if e.level < seuil {
                break;
            }
            let mut e = plat.next().expect("entrée vue");
            e.children = niveau(plat, e.level + 1);
            sortie.push(e);
        }
        sortie
    }
    niveau(&mut plat.into_iter().peekable(), 1)
}

#[cfg(test)]
mod tests {
    use super::super::forme::Span;
    use super::*;

    fn titre(level: u8, texte: &str) -> Block {
        Block::Heading {
            level,
            spans: vec![Span::simple(texte)],
        }
    }

    #[test]
    fn les_titres_simbriquent_et_les_intertitres_sont_ecartes() {
        let p14 = [titre(1, "1. DÉCRYPTAGE DES RÉSULTATS")];
        let p16 = [
            titre(2, "1.1. Nouvel Objectif"),
            titre(3, "Points de convergence"),
        ];
        let p18 = [titre(3, "1.3.1. Objectif mondial")];
        let sommaire = depuis_titres([(14, &p14[..]), (16, &p16[..]), (18, &p18[..])]);
        assert_eq!(sommaire.len(), 1);
        let chapitre = &sommaire[0];
        assert_eq!(chapitre.page_index, 14);
        assert_eq!(chapitre.children.len(), 1);
        assert_eq!(chapitre.children[0].title, "1.1. Nouvel Objectif");
        assert_eq!(
            chapitre.children[0].children[0].title,
            "1.3.1. Objectif mondial"
        );
    }

    #[test]
    fn les_signets_font_foi_quand_il_y_en_a() {
        let signets = [
            Signet {
                titre: "Annexes".into(),
                niveau: 1,
                page_indice: Some(65),
            },
            Signet {
                titre: "A.1".into(),
                niveau: 2,
                page_indice: Some(66),
            },
            Signet {
                titre: "Sans page".into(),
                niveau: 1,
                page_indice: None,
            },
        ];
        let sommaire = depuis_signets(&signets);
        assert_eq!(sommaire.len(), 1);
        assert_eq!(sommaire[0].children[0].page_index, 66);
    }
}
