//! La lecture d'un PDF par PDFium : les segments de texte et leur police, les
//! objets graphiques, les signets, et le rendu de chaque page en image.
//!
//! **Le worker seul s'en sert.** La bibliothèque se charge depuis
//! `PDFIUM_LIB_PATH` ; tout le reste de l'extraction est pur, et s'éprouve sans
//! elle (`domain::extraction`).

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use image::codecs::jpeg::JpegEncoder;
use pdfium_render::prelude::*;

use crate::domain::extraction::brut::{
    Cadre, DocumentBrut, GenreObjet, Objet, PageBrute, Segment, Signet,
};

/// La largeur des images de page : nette sur un téléphone, lisible zoomée.
pub const LARGEUR_IMAGE: i32 = 1080;
/// Réglée par l'essai (ADR-021) : la copie gardée d'un guide pèse environ un
/// quart de moins qu'en qualité 75, sans perte visible sur un tableau.
pub const QUALITE_JPEG: u8 = 60;
/// Une page étroite et très haute donnerait une image de plusieurs gigaoctets,
/// au-delà de ce que le JPEG sait écrire : elle s'affine plutôt.
pub const HAUTEUR_MAX_IMAGE: i32 = 16_384;
/// Au-delà, le fichier n'est pas un document de négociation : il est refusé
/// avant tout rendu.
pub const PAGES_MAX: usize = 2_000;
/// Un sommaire plus grand ou plus profond est un fichier fabriqué.
const SIGNETS_MAX: usize = 10_000;
const PROFONDEUR_MAX: u8 = 16;

pub struct LecteurPdf {
    pdfium: Pdfium,
}

static LECTEUR: OnceLock<Result<Arc<LecteurPdf>, String>> = OnceLock::new();

impl LecteurPdf {
    /// PDFium ne se lie qu'une fois par processus : le premier appel charge la
    /// bibliothèque, les suivants reçoivent le même lecteur.
    pub fn partage(dossier: &str) -> Result<Arc<Self>, String> {
        LECTEUR
            .get_or_init(|| {
                Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dossier))
                    .map(|liaison| {
                        Arc::new(Self {
                            pdfium: Pdfium::new(liaison),
                        })
                    })
                    .map_err(|e| format!("PDFium introuvable dans « {dossier} » : {e}"))
            })
            .clone()
    }

    fn ouvrir<'a>(&'a self, octets: &'a [u8]) -> Result<PdfDocument<'a>, String> {
        let document = self
            .pdfium
            .load_pdf_from_byte_slice(octets, None)
            .map_err(|e| format!("le fichier ne s'ouvre pas comme un PDF : {e}"))?;
        match document.pages().len() as usize {
            0 => Err("il ne contient aucune page".into()),
            n if n > PAGES_MAX => Err(format!(
                "il compte {n} pages, au-delà des {PAGES_MAX} que l'extraction accepte"
            )),
            _ => Ok(document),
        }
    }

    pub fn lire(&self, octets: &[u8]) -> Result<DocumentBrut, String> {
        let document = self.ouvrir(octets)?;
        let mut pages = Vec::new();
        for i in 0..document.pages().len() {
            let page = charger(&document, i)?;
            let n = i as usize + 1;
            let repere = Repere::de(&page).map_err(|e| format!("page {n} : {e}"))?;
            pages.push(PageBrute {
                indice: n,
                etiquette: page.label().map(sans_nul),
                largeur: page.width().value,
                hauteur: page.height().value,
                segments: segments(&page, &repere).map_err(|e| format!("page {n} : {e}"))?,
                objets: objets(&page, &repere),
            });
        }
        Ok(DocumentBrut {
            signets: signets(&document, pages.len()),
            pages,
        })
    }

    /// Une image JPEG par page, dans l'ordre du fichier, confiée à `deposer`
    /// aussitôt rendue : un long document ne tient jamais entier en mémoire.
    /// Rend le nombre de pages.
    pub fn rendre(
        &self,
        octets: &[u8],
        mut deposer: impl FnMut(Vec<u8>) -> Result<(), String>,
    ) -> Result<usize, String> {
        let document = self.ouvrir(octets)?;
        let config = PdfRenderConfig::new()
            .set_target_width(LARGEUR_IMAGE)
            .set_maximum_width(LARGEUR_IMAGE)
            .set_maximum_height(HAUTEUR_MAX_IMAGE);
        let mut nombre = 0;
        for i in 0..document.pages().len() {
            let image = charger(&document, i)?
                .render_with_config(&config)
                .and_then(|b| b.as_image())
                .map_err(|e| format!("page {} : rendu impossible : {e}", i + 1))?
                .to_rgb8();
            let mut jpeg = Vec::new();
            JpegEncoder::new_with_quality(&mut jpeg, QUALITE_JPEG)
                .encode_image(&image)
                .map_err(|e| format!("page {} : encodage impossible : {e}", i + 1))?;
            deposer(jpeg)?;
            nombre += 1;
        }
        Ok(nombre)
    }
}

/// `iter()` s'arrête sans rien dire à la première page qui ne se charge pas :
/// le document serait tronqué.
fn charger<'a>(document: &PdfDocument<'a>, i: PdfPageIndex) -> Result<PdfPage<'a>, String> {
    document
        .pages()
        .get(i)
        .map_err(|_| format!("la page {} ne se charge pas", i + 1))
}

/// PostgreSQL refuse U+0000 dans un `text` comme dans un `jsonb`.
fn sans_nul(texte: &str) -> String {
    texte.replace('\0', "")
}

/// Le passage de l'espace utilisateur du PDF, où PDFium place caractères et
/// objets, à la page telle qu'elle s'affiche, origine en haut à gauche : la
/// rotation et la boîte effective (CropBox dans la MediaBox), comme au rendu.
struct Repere {
    rotation: PdfPageRenderRotation,
    gauche: f32,
    bas: f32,
    droite: f32,
    haut: f32,
}

impl Repere {
    fn de(page: &PdfPage) -> Result<Self, PdfiumError> {
        // FPDF_GetPageBoundingBox rend la boîte effective, pas l'emprise du contenu.
        let boite = page.boundaries().bounding()?.bounds;
        Ok(Self {
            rotation: page.rotation()?,
            gauche: boite.left().value,
            bas: boite.bottom().value,
            droite: boite.right().value,
            haut: boite.top().value,
        })
    }

    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        match self.rotation {
            PdfPageRenderRotation::None => (x - self.gauche, self.haut - y),
            PdfPageRenderRotation::Degrees90 => (y - self.bas, x - self.gauche),
            PdfPageRenderRotation::Degrees180 => (self.droite - x, y - self.bas),
            PdfPageRenderRotation::Degrees270 => (self.haut - y, self.droite - x),
        }
    }

    fn cadre(&self, rect: &PdfRect) -> Cadre {
        let (xa, ya) = self.point(rect.left().value, rect.bottom().value);
        let (xb, yb) = self.point(rect.right().value, rect.top().value);
        Cadre::new(xa.min(xb), ya.min(yb), xa.max(xb), ya.max(yb))
    }
}

fn nom_italique(police: &str) -> bool {
    let p = police.to_lowercase();
    p.contains("italic") || p.contains("oblique")
}

/// Regroupe les caractères consécutifs de même police, même corps et même ligne
/// de base. Un saut de ligne engendré par PDFium ferme le segment.
fn segments(page: &PdfPage, repere: &Repere) -> Result<Vec<Segment>, PdfiumError> {
    let texte = page.text()?;
    let mut sortie: Vec<Segment> = Vec::new();
    let mut courant: Option<Segment> = None;

    for c in texte.chars().iter() {
        let Some(ch) = c.unicode_char().filter(|&ch| ch != '\0') else {
            continue;
        };
        if ch == '\r' || ch == '\n' {
            if let Some(mut s) = courant.take() {
                s.fin_de_ligne = true;
                sortie.push(s);
            }
            continue;
        }
        let Ok(bornes) = c.loose_bounds() else {
            continue;
        };
        let cadre = repere.cadre(&bornes);
        let base = c
            .origin()
            .map_or(cadre.y1, |(x, y)| repere.point(x.value, y.value).1);
        let police = sans_nul(&c.font_name());
        // Le corps suit l'axe vertical du glyphe : un texte tourné garde le sien.
        let echelle = c.matrix().map_or(0.0, |m| m.c().hypot(m.d()));
        let taille = (c.unscaled_font_size().value * echelle * 10.0).round() / 10.0;
        let italique = c.font_is_italic() || nom_italique(&police);
        let espace = ch == ' ';

        let continue_le_courant = courant.as_ref().is_some_and(|s| {
            (espace || (s.police == police && s.taille == taille && s.italique == italique))
                && (s.base - base).abs() < taille * 0.4
                && cadre.x0 - s.cadre.x1 < taille * 1.5
        });
        match courant.as_mut() {
            Some(s) if continue_le_courant => {
                s.texte.push(ch);
                if !espace {
                    s.cadre.unir(&cadre);
                }
            }
            _ => {
                if let Some(s) = courant.take() {
                    sortie.push(s);
                }
                if !espace {
                    courant = Some(Segment {
                        texte: ch.to_string(),
                        police,
                        italique,
                        taille,
                        base,
                        cadre,
                        fin_de_ligne: false,
                    });
                }
            }
        }
    }
    sortie.extend(courant);
    Ok(sortie)
}

fn objets(page: &PdfPage, repere: &Repere) -> Vec<Objet> {
    page.objects()
        .iter()
        .filter_map(|objet| {
            let genre = match objet.object_type() {
                PdfPageObjectType::Path => GenreObjet::Chemin,
                PdfPageObjectType::Image => GenreObjet::Image,
                PdfPageObjectType::Shading => GenreObjet::Degrade,
                PdfPageObjectType::XObjectForm => GenreObjet::Forme,
                _ => return None,
            };
            let bornes = objet.bounds().ok()?;
            Some(Objet {
                genre,
                cadre: repere.cadre(&bornes.to_rect()),
            })
        })
        .collect()
}

/// Parcours en profondeur, dans l'ordre du sommaire. Un signet déjà vu arrête
/// sa branche : un sommaire qui boucle ne fait pas tourner le worker sans fin.
fn signets(document: &PdfDocument, pages: usize) -> Vec<Signet> {
    let mut sortie = Vec::new();
    let mut vus = HashSet::new();
    let mut pile: Vec<(PdfBookmark, u8)> = document
        .bookmarks()
        .root()
        .into_iter()
        .map(|r| (r, 1))
        .collect();
    while let Some((s, niveau)) = pile.pop() {
        if sortie.len() >= SIGNETS_MAX || !vus.insert(s.clone()) {
            continue;
        }
        sortie.push(Signet {
            titre: sans_nul(&s.title().unwrap_or_default()),
            niveau,
            page_indice: s
                .destination()
                .and_then(|d| d.page_index().ok())
                .and_then(|i| usize::try_from(i).ok())
                .filter(|&i| i < pages)
                .map(|i| i + 1),
        });
        // Le frère se traite après tous les descendants : il entre le premier.
        if let Some(frere) = s.next_sibling() {
            pile.push((frere, niveau));
        }
        if niveau < PROFONDEUR_MAX {
            if let Some(enfant) = s.first_child() {
                pile.push((enfant, niveau + 1));
            }
        }
    }
    sortie
}
