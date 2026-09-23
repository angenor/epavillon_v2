//! Recoller deux lignes. Le guide de la CdP30, sorti de Word sans coupure
//! automatique, n'a que de vrais traits d'union en fin de ligne : le tiret se
//! garde, sauf preuve qu'il coupe un mot.

use std::collections::HashSet;

use super::forme::Span;
use super::lignes::TIRET_DE_FIN;

/// Les mots vus en milieu de ligne dans tout le document. Le dernier mot d'une
/// ligne n'atteste rien : il peut être coupé.
#[derive(Debug, Default)]
pub struct Lexique {
    mots: HashSet<String>,
}

fn mots_de(texte: &str) -> impl Iterator<Item = String> + '_ {
    texte
        .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '\u{2011}'))
        .filter(|m| m.chars().count() > 1)
        .map(str::to_lowercase)
}

impl Lexique {
    pub fn depuis<'a>(lignes: impl IntoIterator<Item = &'a str>) -> Self {
        let mut mots = HashSet::new();
        for texte in lignes {
            let interieur = texte.trim_end().trim_end_matches(TIRET_DE_FIN);
            let jetons: Vec<&str> = interieur.split_whitespace().collect();
            for j in jetons.iter().take(jetons.len().saturating_sub(1)) {
                mots.extend(mots_de(j));
            }
        }
        Self { mots }
    }

    pub fn atteste(&self, mot: &str) -> bool {
        self.mots.contains(&mot.to_lowercase())
    }
}

/// Une césure décidée, pour le rapport de l'extraction.
#[derive(Debug, Clone, PartialEq)]
pub struct Cesure {
    pub avant: String,
    pub apres: String,
    pub tiret_garde: bool,
}

fn texte_de(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
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

/// Recolle `suite` au bout de `bloc`. Le trait d'union de fin de ligne — le
/// caractère de contrôle de PDFium, ou un tiret ordinaire — se garde, sauf si le
/// mot recollé est attesté ailleurs et pas le mot à tiret. Une adresse coupée se
/// recolle sans blanc. Toute autre fin de ligne vaut un blanc.
pub fn joindre(bloc: &mut Vec<Span>, suite: Vec<Span>, lexique: &Lexique) -> Option<Cesure> {
    let queue = texte_de(bloc);
    let queue = queue.trim_end();
    let tete = texte_de(&suite);
    let premier_mot = tete.split_whitespace().next().unwrap_or("");
    let dernier_mot = queue.split_whitespace().last().unwrap_or("");

    let coupe = queue.ends_with(TIRET_DE_FIN) || (queue.ends_with('-') && dernier_mot.len() > 1);
    let mut cesure = None;
    let jonction = if coupe && premier_mot.starts_with(char::is_alphanumeric) {
        let avant = dernier_mot.trim_end_matches([TIRET_DE_FIN, '-']);
        let avant = avant
            .rsplit(|c: char| !c.is_alphanumeric())
            .next()
            .unwrap_or(avant);
        let apres: String = premier_mot
            .chars()
            .take_while(|c| c.is_alphanumeric())
            .collect();
        let tiret_garde = !(lexique.atteste(&format!("{avant}{apres}"))
            && !lexique.atteste(&format!("{avant}-{apres}")));
        cesure = Some(Cesure {
            avant: avant.to_owned(),
            apres,
            tiret_garde,
        });
        rogner_fin(bloc);
        if let Some(d) = bloc.last_mut() {
            d.text = d.text.trim_end_matches([TIRET_DE_FIN, '-']).to_owned();
        }
        if tiret_garde {
            "-"
        } else {
            ""
        }
    } else if (dernier_mot.contains("://") || dernier_mot.starts_with("www."))
        && !premier_mot.is_empty()
        && premier_mot
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "/-_.%?=&#~".contains(c))
    {
        ""
    } else {
        " "
    };
    rogner_fin(bloc);
    if !jonction.is_empty() {
        bloc.push(Span::simple(jonction));
    }
    bloc.extend(suite);
    cesure
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joint(a: &str, b: &str, lexique: &Lexique) -> String {
        let mut bloc = vec![Span::simple(a)];
        joindre(&mut bloc, vec![Span::simple(b)], lexique);
        texte_de(&bloc)
    }

    #[test]
    fn un_vrai_trait_dunion_se_garde() {
        let lexique = Lexique::depuis(["la Convention-cadre des Nations Unies"]);
        assert_eq!(
            joint("de la Convention\u{2}", "cadre des Nations", &lexique),
            "de la Convention-cadre des Nations"
        );
        assert_eq!(
            joint("aux États-", "Unis et", &Lexique::default()),
            "aux États-Unis et"
        );
    }

    #[test]
    fn une_cesure_typographique_se_recolle_si_le_mot_est_atteste() {
        let lexique = Lexique::depuis(["la négociation reprend demain"]);
        assert_eq!(
            joint("la négo-", "ciation reprend", &lexique),
            "la négociation reprend"
        );
    }

    #[test]
    fn une_adresse_coupee_se_recolle_sans_blanc() {
        assert_eq!(
            joint(
                "Lien : https://unfccc.int/",
                "cop29/auvs",
                &Lexique::default()
            ),
            "Lien : https://unfccc.int/cop29/auvs"
        );
    }

    #[test]
    fn une_fin_de_ligne_ordinaire_vaut_un_blanc() {
        assert_eq!(
            joint("les CdP se tiennent ", "chaque année", &Lexique::default()),
            "les CdP se tiennent chaque année"
        );
    }

    #[test]
    fn le_dernier_mot_dune_ligne_natteste_rien() {
        let lexique = Lexique::depuis(["une ligne qui finit par négo"]);
        assert!(lexique.atteste("ligne"));
        assert!(!lexique.atteste("négo"));
    }
}
