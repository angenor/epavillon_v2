//! Des segments de PDFium aux lignes, dans l'ordre du flux.

use crate::{Cadre, Segment};

/// PDFium remplace le trait d'union d'une fin de ligne par ce caractère de
/// contrôle : le tiret est bien imprimé, mais tenu pour une coupure.
pub const TIRET_DE_FIN: char = '\u{2}';

#[derive(Clone, Debug)]
pub struct Ligne {
    pub segments: Vec<Segment>,
    pub cadre: Cadre,
    pub base: f32,
    /// Le corps dominant, exposants exclus.
    pub taille: f32,
    /// Rang dans le flux de la page.
    pub ordre: usize,
    /// 0 pour le corps ; n pour le n-ième encadré flottant recomposé.
    pub groupe: usize,
}

impl Ligne {
    pub fn texte(&self) -> String {
        self.segments.iter().map(|s| s.texte.as_str()).collect()
    }

    pub fn largeur(&self) -> f32 {
        self.cadre.x1 - self.cadre.x0
    }

    /// Tout segment non blanc est en gras.
    pub fn grasse(&self) -> bool {
        self.segments
            .iter()
            .filter(|s| !s.texte.trim().is_empty())
            .all(|s| est_gras(&s.police))
    }

    pub fn chevauche_en_x(&self, autre: &Cadre) -> bool {
        let recouvrement = self.cadre.x1.min(autre.x1) - self.cadre.x0.max(autre.x0);
        recouvrement > 0.3 * self.largeur().min(autre.x1 - autre.x0).max(1.0)
    }

    pub fn centre(&self) -> (f32, f32) {
        (
            (self.cadre.x0 + self.cadre.x1) / 2.0,
            (self.cadre.y0 + self.cadre.y1) / 2.0,
        )
    }
}

pub fn est_gras(police: &str) -> bool {
    let p = police.to_lowercase();
    p.contains("bold") || p.contains("black") || p.contains("heavy")
}

/// Un segment en petit corps, remonté dans la hauteur de la ligne : un appel de
/// note ou un « ème ».
pub fn est_exposant(s: &Segment, ligne: &Ligne) -> bool {
    s.taille < 0.85 * ligne.taille && s.base < ligne.base - 0.5 && s.cadre.y1 > ligne.cadre.y0 - 2.0
}

pub fn construire(segments: &[Segment]) -> Vec<Ligne> {
    let mut lignes: Vec<Ligne> = Vec::new();
    let mut coupure = true;
    for s in segments {
        // PDFium coupe aussi après un exposant : la ligne continue si le
        // segment suivant reprend sur la même ligne de base, à droite.
        let reprend = lignes.last().is_some_and(|l| {
            let dernier = l.segments.last().expect("une ligne a un segment");
            let a_droite =
                s.cadre.x0 >= dernier.cadre.x1 - 1.0 && s.cadre.x0 - dernier.cadre.x1 < l.taille;
            let meme_base = (s.base - l.base).abs() < 0.3 * l.taille;
            let exposant = s.taille < 0.85 * l.taille
                && s.cadre.y1 > l.cadre.y0 - 2.0
                && s.cadre.y0 < l.cadre.y1;
            a_droite && (meme_base || exposant)
        });
        let nouvelle = (coupure && !reprend)
            || lignes.last().is_none_or(|l| {
                let dernier = l.segments.last().expect("une ligne a un segment");
                let autre_base = (s.base - l.base).abs() > 0.5 * l.taille.max(s.taille);
                let exposant = s.taille < 0.85 * l.taille
                    && s.cadre.y1 > l.cadre.y0 - 2.0
                    && s.cadre.y0 < l.cadre.y1;
                let revient = s.cadre.x0 < dernier.cadre.x1 - 2.0;
                let saute = s.cadre.x0 - dernier.cadre.x1 > 4.0 * l.taille;
                (autre_base && !exposant) || revient || saute
            });
        if nouvelle {
            lignes.push(Ligne {
                segments: vec![s.clone()],
                cadre: s.cadre.clone(),
                base: s.base,
                taille: s.taille,
                ordre: lignes.len(),
                groupe: 0,
            });
        } else {
            let l = lignes.last_mut().expect("ligne ouverte");
            l.cadre.unir(&s.cadre);
            l.segments.push(s.clone());
        }
        coupure = s.fin_de_ligne;
    }
    for l in &mut lignes {
        fixer_corps(l);
    }
    lignes.retain(|l| !l.texte().trim().is_empty());
    for (i, l) in lignes.iter_mut().enumerate() {
        l.ordre = i * 1000;
    }
    lignes
}

fn fixer_corps(l: &mut Ligne) {
    let principal = l
        .segments
        .iter()
        .filter(|s| !s.texte.trim().is_empty())
        .max_by_key(|s| s.texte.trim().chars().count())
        .cloned();
    if let Some(p) = principal {
        l.taille = p.taille;
        l.base = p.base;
    }
}
