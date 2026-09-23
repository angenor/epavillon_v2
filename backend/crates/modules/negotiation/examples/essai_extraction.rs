//! Essai d'extraction de specs/011 (phase 1) : ce que PDFium rend du guide.
//!
//! `cargo run -p negotiation --example essai_extraction -- ../.essais/guide-cdp30.pdf`
//! (`--sans-images` réutilise les images déjà rendues, pour itérer sur les règles.)
//!
//! Écrit dans `.essais/sortie/` : `brut.json` (segments, polices, cadres,
//! objets graphiques, signets), une image JPEG par page, et — une fois les règles
//! de R7 appliquées — `forme-lisible.json` et `rapport.md`. Jetable : le travail
//! `negotiation.document.extract` reprendra ce qui aura tenu.

use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use image::codecs::jpeg::JpegEncoder;
use pdfium_render::prelude::*;
use serde::Serialize;

#[path = "essai_extraction/blocs.rs"]
mod blocs;
#[path = "essai_extraction/forme.rs"]
mod forme;
#[path = "essai_extraction/lignes.rs"]
mod lignes;
#[path = "essai_extraction/regles.rs"]
mod regles;
#[path = "essai_extraction/zones.rs"]
mod zones;

const LARGEUR_IMAGE: i32 = 1080;
const QUALITE_JPEG: u8 = 75;

#[derive(Serialize, Clone, Debug)]
pub struct Cadre {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Cadre {
    /// PDF : origine en bas à gauche. Ici : en haut à gauche, comme on lit.
    fn depuis(rect: &PdfRect, hauteur: f32) -> Self {
        Cadre {
            x0: rect.left().value,
            y0: hauteur - rect.top().value,
            x1: rect.right().value,
            y1: hauteur - rect.bottom().value,
        }
    }

    fn unir(&mut self, autre: &Cadre) {
        self.x0 = self.x0.min(autre.x0);
        self.y0 = self.y0.min(autre.y0);
        self.x1 = self.x1.max(autre.x1);
        self.y1 = self.y1.max(autre.y1);
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Segment {
    pub texte: String,
    pub police: String,
    pub graisse: u32,
    pub italique: bool,
    pub taille: f32,
    /// Ligne de base, depuis le haut de la page.
    pub base: f32,
    pub cadre: Cadre,
    /// Le segment finit sur un saut de ligne que PDFium a engendré.
    pub fin_de_ligne: bool,
    /// Le dernier caractère est un trait d'union que PDFium tient pour une césure.
    pub cesure: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Objet {
    pub genre: &'static str,
    pub cadre: Cadre,
}

#[derive(Serialize, Clone, Debug)]
pub struct PageBrute {
    pub indice: usize,
    pub etiquette: Option<String>,
    pub largeur: f32,
    pub hauteur: f32,
    pub segments: Vec<Segment>,
    pub objets: Vec<Objet>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Signet {
    pub titre: String,
    pub niveau: u8,
    pub page_indice: Option<usize>,
}

#[derive(Serialize)]
pub struct Brut {
    pub pages: Vec<PageBrute>,
    pub signets: Vec<Signet>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let sans_images = arguments.iter().any(|a| a == "--sans-images");
    let pdf = arguments
        .iter()
        .find(|a| !a.starts_with("--"))
        .ok_or("usage : essai_extraction <fichier.pdf> [--sans-images]")?;
    let racine = racine_du_depot();
    let sortie = racine.join(".essais/sortie");
    fs::create_dir_all(sortie.join("pages"))?;

    let pdfium = Pdfium::new(Pdfium::bind_to_library(
        Pdfium::pdfium_platform_library_name_at_path(&chemin_pdfium(&racine)),
    )?);

    let debut = Instant::now();
    let document = pdfium.load_pdf_from_file(&pdf, None)?;
    let brut = lire(&document)?;
    let duree_texte = debut.elapsed();

    let debut_images = Instant::now();
    let poids_par_page = if sans_images {
        (1..=brut.pages.len())
            .map(|i| fs::metadata(sortie.join(format!("pages/{i}.jpg"))).map_or(0, |m| m.len()))
            .collect()
    } else {
        rendre_images(&document, &sortie.join("pages"))?
    };
    let duree_images = debut_images.elapsed();

    fs::write(sortie.join("brut.json"), serde_json::to_vec_pretty(&brut)?)?;

    let mesures = regles::Mesures {
        poids_pdf: fs::metadata(pdf)?.len(),
        poids_images: poids_par_page.iter().sum(),
        poids_par_page,
        duree_texte,
        duree_images,
    };
    regles::composer(&brut, &mesures, &sortie)?;

    println!(
        "{} pages, {} signets — texte {:.1} s, images {:.1} s → {}",
        brut.pages.len(),
        brut.signets.len(),
        duree_texte.as_secs_f32(),
        duree_images.as_secs_f32(),
        sortie.display()
    );
    Ok(())
}

fn racine_du_depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn chemin_pdfium(racine: &Path) -> PathBuf {
    match std::env::var("PDFIUM_LIB_PATH") {
        Ok(chemin) if Path::new(&chemin).is_absolute() => PathBuf::from(chemin),
        Ok(chemin) => racine.join("backend").join(chemin),
        Err(_) => racine.join(".outils/pdfium/lib"),
    }
}

fn lire(document: &PdfDocument) -> Result<Brut, PdfiumError> {
    let mut pages = Vec::new();
    for (i, page) in document.pages().iter().enumerate() {
        let hauteur = page.height().value;
        pages.push(PageBrute {
            indice: i + 1,
            etiquette: page.label().map(str::to_owned),
            largeur: page.width().value,
            hauteur,
            segments: segments(&page, hauteur)?,
            objets: objets(&page, hauteur),
        });
    }
    Ok(Brut {
        pages,
        signets: signets(document),
    })
}

/// Regroupe les caractères consécutifs de même police, même corps et même
/// ligne de base. Un saut de ligne engendré par PDFium ferme le segment.
fn segments(page: &PdfPage, hauteur: f32) -> Result<Vec<Segment>, PdfiumError> {
    let texte = page.text()?;
    let mut sortie: Vec<Segment> = Vec::new();
    let mut courant: Option<Segment> = None;

    for c in texte.chars().iter() {
        let Some(ch) = c.unicode_char() else { continue };
        if ch == '\r' || ch == '\n' {
            if let Some(mut s) = courant.take() {
                s.fin_de_ligne = true;
                s.cesure = c.index() > 0
                    && texte
                        .chars()
                        .get(c.index() - 1)
                        .ok()
                        .and_then(|p| p.is_hyphen().ok())
                        .unwrap_or(false);
                sortie.push(s);
            }
            continue;
        }
        let Ok(bornes) = c.loose_bounds() else {
            continue;
        };
        let cadre = Cadre::depuis(&bornes, hauteur);
        let base = hauteur
            - c.origin_y()
                .map(|y| y.value)
                .unwrap_or(bornes.bottom().value);
        let police = c.font_name();
        let taille = (c.scaled_font_size().value * 10.0).round() / 10.0;
        let graisse = graisse(c.font_weight());
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
                if espace {
                    continue;
                }
                courant = Some(Segment {
                    texte: ch.to_string(),
                    police,
                    graisse,
                    italique,
                    taille,
                    base,
                    cadre,
                    fin_de_ligne: false,
                    cesure: false,
                });
            }
        }
    }
    if let Some(s) = courant.take() {
        sortie.push(s);
    }
    Ok(sortie)
}

fn graisse(g: Option<PdfFontWeight>) -> u32 {
    match g {
        Some(PdfFontWeight::Weight100) => 100,
        Some(PdfFontWeight::Weight200) => 200,
        Some(PdfFontWeight::Weight300) => 300,
        Some(PdfFontWeight::Weight500) => 500,
        Some(PdfFontWeight::Weight600) => 600,
        Some(PdfFontWeight::Weight700Bold) => 700,
        Some(PdfFontWeight::Weight800) => 800,
        Some(PdfFontWeight::Weight900) => 900,
        Some(PdfFontWeight::Custom(v)) => v,
        _ => 400,
    }
}

fn nom_italique(police: &str) -> bool {
    let p = police.to_lowercase();
    p.contains("italic") || p.contains("oblique")
}

/// Les filets, images et formes : ce qui trahit un tableau ou une figure.
fn objets(page: &PdfPage, hauteur: f32) -> Vec<Objet> {
    let mut sortie = Vec::new();
    for objet in page.objects().iter() {
        let genre = match objet.object_type() {
            PdfPageObjectType::Path => "chemin",
            PdfPageObjectType::Image => "image",
            PdfPageObjectType::Shading => "degrade",
            PdfPageObjectType::XObjectForm => "forme",
            _ => continue,
        };
        if let Ok(bornes) = objet.bounds() {
            sortie.push(Objet {
                genre,
                cadre: Cadre::depuis(&bornes.to_rect(), hauteur),
            });
        }
    }
    sortie
}

fn signets(document: &PdfDocument) -> Vec<Signet> {
    fn parcourir(signet: PdfBookmark, niveau: u8, sortie: &mut Vec<Signet>) {
        let mut courant = Some(signet);
        while let Some(s) = courant {
            sortie.push(Signet {
                titre: s.title().unwrap_or_default(),
                niveau,
                page_indice: s
                    .destination()
                    .and_then(|d| d.page_index().ok())
                    .map(|i| i as usize + 1),
            });
            if let Some(enfant) = s.first_child() {
                parcourir(enfant, niveau + 1, sortie);
            }
            courant = s.next_sibling();
        }
    }
    let mut sortie = Vec::new();
    if let Some(racine) = document.bookmarks().root() {
        parcourir(racine, 1, &mut sortie);
    }
    sortie
}

fn rendre_images(
    document: &PdfDocument,
    dossier: &Path,
) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let config = PdfRenderConfig::new().set_target_width(LARGEUR_IMAGE);
    let mut poids = Vec::new();
    for (i, page) in document.pages().iter().enumerate() {
        let image = page.render_with_config(&config)?.as_image()?.to_rgb8();
        let chemin = dossier.join(format!("{}.jpg", i + 1));
        let mut octets = Vec::new();
        JpegEncoder::new_with_quality(&mut octets, QUALITE_JPEG).encode_image(&image)?;
        poids.push(octets.len() as u64);
        fs::write(chemin, octets)?;
    }
    Ok(poids)
}
