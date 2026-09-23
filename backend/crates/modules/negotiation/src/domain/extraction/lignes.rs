//! Des segments aux lignes, dans l'ordre du flux.

use super::brut::{Cadre, Segment};

/// PDFium remplace le trait d'union d'une fin de ligne par ce caractère de
/// contrôle : le tiret est bien imprimé, mais tenu pour une coupure.
pub const TIRET_DE_FIN: char = '\u{2}';

#[derive(Debug, Clone, PartialEq)]
pub struct Ligne {
    pub segments: Vec<Segment>,
    pub cadre: Cadre,
    pub base: f32,
    /// Le corps dominant, exposants exclus.
    pub taille: f32,
    /// Rang dans le flux de la page, espacé de mille pour qu'un encadré
    /// flottant puisse s'insérer entre deux lignes.
    pub ordre: usize,
    /// 0 pour le corps ; n pour le n-ième encadré flottant recomposé.
    pub groupe: usize,
}

impl Ligne {
    pub fn texte(&self) -> String {
        self.segments.iter().map(|s| s.texte.as_str()).collect()
    }

    /// Tout segment non blanc est en gras.
    pub fn grasse(&self) -> bool {
        self.segments
            .iter()
            .filter(|s| !s.texte.trim().is_empty())
            .all(|s| est_gras(&s.police))
    }

    /// Le plus grand corps de la ligne : un numéro de titre peut être plus gros
    /// que son intitulé.
    pub fn taille_max(&self) -> f32 {
        self.segments
            .iter()
            .filter(|s| !s.texte.trim().is_empty())
            .map(|s| s.taille)
            .fold(0.0, f32::max)
    }

    pub fn chevauche_en_x(&self, autre: &Cadre) -> bool {
        let recouvrement = self.cadre.x1.min(autre.x1) - self.cadre.x0.max(autre.x0);
        recouvrement > 0.3 * self.cadre.largeur().min(autre.largeur()).max(1.0)
    }

    pub fn centre(&self) -> (f32, f32) {
        (
            (self.cadre.x0 + self.cadre.x1) / 2.0,
            (self.cadre.y0 + self.cadre.y1) / 2.0,
        )
    }
}

/// Le gras se lit au nom de la police : la graisse que rend PDFium ne vaut rien.
pub fn est_gras(police: &str) -> bool {
    let p = police.to_lowercase();
    p.contains("bold") || p.contains("black") || p.contains("heavy")
}

/// Un segment en petit corps, remonté dans la hauteur de la ligne : un appel de
/// note ou un « ème ».
pub fn est_exposant(s: &Segment, ligne: &Ligne) -> bool {
    s.taille < 0.85 * ligne.taille && s.base < ligne.base - 0.5 && s.cadre.y1 > ligne.cadre.y0 - 2.0
}

fn exposant_de(s: &Segment, l: &Ligne) -> bool {
    s.taille < 0.85 * l.taille && s.cadre.y1 > l.cadre.y0 - 2.0 && s.cadre.y0 < l.cadre.y1
}

pub fn construire(segments: &[Segment]) -> Vec<Ligne> {
    let mut lignes: Vec<Ligne> = Vec::new();
    let mut coupure = true;
    for s in segments {
        // PDFium coupe aussi après un exposant : la ligne continue si le segment
        // suivant reprend à droite, sur la même base ou en exposant.
        let reprend = lignes.last().is_some_and(|l| {
            let dernier = l.segments.last().expect("une ligne a un segment");
            // Le cadre d'un glyphe déborde un peu de son pas : deux points de
            // chevauchement restent « à droite ».
            let a_droite =
                s.cadre.x0 >= dernier.cadre.x1 - 2.0 && s.cadre.x0 - dernier.cadre.x1 < l.taille;
            a_droite && ((s.base - l.base).abs() < 0.3 * l.taille || exposant_de(s, l))
        });
        let nouvelle = (coupure && !reprend)
            || lignes.last().is_none_or(|l| {
                let dernier = l.segments.last().expect("une ligne a un segment");
                let autre_base = (s.base - l.base).abs() > 0.5 * l.taille.max(s.taille);
                let revient = s.cadre.x0 < dernier.cadre.x1 - 2.0;
                let saute = s.cadre.x0 - dernier.cadre.x1 > 4.0 * l.taille;
                (autre_base && !exposant_de(s, l)) || revient || saute
            });
        if nouvelle {
            lignes.push(Ligne {
                segments: vec![s.clone()],
                cadre: s.cadre,
                base: s.base,
                taille: s.taille,
                ordre: 0,
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
    // Dès 1000 : un encadré se range avant la première ligne sans passer sous zéro.
    for (i, l) in lignes.iter_mut().enumerate() {
        l.ordre = (i + 1) * 1000;
    }
    lignes
}

fn fixer_corps(l: &mut Ligne) {
    let principal = l
        .segments
        .iter()
        .filter(|s| !s.texte.trim().is_empty())
        .max_by_key(|s| s.texte.trim().chars().count());
    if let Some(p) = principal {
        l.taille = p.taille;
        l.base = p.base;
    }
}

#[cfg(test)]
pub(crate) mod essai {
    use super::super::brut::{Cadre, Segment};

    /// Un segment de corps 11, sur la ligne de base `base`.
    pub fn seg(texte: &str, x0: f32, base: f32) -> Segment {
        let largeur = texte.chars().count() as f32 * 5.5;
        Segment {
            texte: texte.to_owned(),
            police: "ArialMT".into(),
            italique: false,
            taille: 11.0,
            base,
            cadre: Cadre::new(x0, base - 9.0, x0 + largeur, base + 2.0),
            fin_de_ligne: false,
        }
    }

    pub fn fin(mut s: Segment) -> Segment {
        s.fin_de_ligne = true;
        s
    }

    pub fn taille(mut s: Segment, taille: f32) -> Segment {
        s.taille = taille;
        s
    }

    pub fn gras(mut s: Segment) -> Segment {
        s.police = "Arial-BoldMT".into();
        s
    }
}

#[cfg(test)]
mod tests {
    use super::essai::*;
    use super::*;

    #[test]
    fn un_saut_de_pdfium_ferme_la_ligne() {
        let lignes = construire(&[
            fin(seg("Première ligne", 72.0, 100.0)),
            fin(seg("Seconde", 72.0, 114.0)),
        ]);
        assert_eq!(lignes.len(), 2);
        assert_eq!(lignes[1].ordre, 2000);
    }

    #[test]
    fn deux_segments_sur_la_meme_base_font_une_ligne() {
        let lignes = construire(&[
            seg("La Conférence ", 72.0, 100.0),
            fin(seg("des Parties", 150.0, 100.0)),
        ]);
        assert_eq!(lignes.len(), 1);
        assert_eq!(lignes[0].texte(), "La Conférence des Parties");
    }

    /// PDFium coupe après l'appel de note ; « . Les principales » continue la
    /// même ligne.
    #[test]
    fn un_appel_de_note_ne_coupe_pas_la_ligne() {
        let texte = seg("de la CCNUCC", 72.0, 100.0);
        let x = texte.cadre.x1;
        let mut appel = taille(seg("1", x, 97.0), 7.0);
        appel.cadre = Cadre::new(x, 91.0, x + 4.0, 98.0);
        appel.fin_de_ligne = true;
        let suite = fin(seg(". Les principales", x + 4.0, 100.0));
        let lignes = construire(&[texte, appel, suite]);
        assert_eq!(lignes.len(), 1);
        assert_eq!(lignes[0].texte(), "de la CCNUCC1. Les principales");
        assert!(est_exposant(&lignes[0].segments[1], &lignes[0]));
    }

    #[test]
    fn une_gouttiere_separe_deux_lignes() {
        let lignes = construire(&[
            seg("colonne gauche", 72.0, 100.0),
            seg("droite", 320.0, 100.0),
        ]);
        assert_eq!(lignes.len(), 2);
    }

    #[test]
    fn le_gras_se_lit_au_nom_de_la_police() {
        assert!(est_gras("BCDMEE+Arial-BoldMT"));
        assert!(est_gras("Arial-Black"));
        assert!(!est_gras("ArialMT"));
    }
}
