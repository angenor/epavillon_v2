//! Fabrique `tests/fixtures/petit.pdf`, le PDF sur lequel s'éprouve
//! l'extraction : quatre pages qui portent, chacune, une difficulté du vrai
//! guide de la CdP30 — un pied répété, une note sous son filet, un appel de
//! note, une césure, un vrai trait d'union, un terme anglais en italique, deux
//! colonnes, un tableau, une liste.
//!
//! `cargo run -p negotiation --example fabriquer_petit_pdf` : le fichier se
//! refait à l'identique, et se commite.

use std::path::Path;

use pdfium_render::prelude::*;

const HAUTEUR: f32 = 842.0;

struct Polices {
    normale: PdfFontToken,
    grasse: PdfFontToken,
    italique: PdfFontToken,
}

/// Écrit un texte, `y` mesuré depuis le haut de la page, comme on lit. Rend son
/// bord droit, pour y accoler la suite de la ligne.
fn texte(
    page: &mut PdfPage,
    x: f32,
    y: f32,
    contenu: &str,
    police: PdfFontToken,
    taille: f32,
) -> f32 {
    page.objects_mut()
        .create_text_object(
            PdfPoints::new(x),
            PdfPoints::new(HAUTEUR - y),
            contenu,
            police,
            PdfPoints::new(taille),
        )
        .expect("objet de texte")
        .bounds()
        .expect("cadre du texte")
        .right()
        .value
}

fn filet(page: &mut PdfPage, x1: f32, y1: f32, x2: f32, y2: f32) {
    page.objects_mut()
        .create_path_object_line(
            PdfPoints::new(x1),
            PdfPoints::new(HAUTEUR - y1),
            PdfPoints::new(x2),
            PdfPoints::new(HAUTEUR - y2),
            PdfColor::BLACK,
            PdfPoints::new(0.6),
        )
        .expect("filet");
}

fn pied(page: &mut PdfPage, p: &Polices, numero: u32) {
    texte(
        page,
        72.0,
        800.0,
        "© PETIT GUIDE D'ESSAI, ÉDITION 2025.",
        p.grasse,
        6.0,
    );
    texte(page, 515.0, 800.0, &numero.to_string(), p.normale, 11.0);
}

fn lignes(page: &mut PdfPage, p: &Polices, x: f32, y: f32, contenu: &[&str]) -> f32 {
    let mut y = y;
    for ligne in contenu {
        texte(page, x, y, ligne, p.normale, 11.0);
        y += 14.5;
    }
    y
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR"));
    let pdfium_dir = std::env::var("PDFIUM_LIB_PATH")
        .ok()
        .filter(|c| Path::new(c).is_absolute())
        .unwrap_or_else(|| {
            racine
                .join("../../../../.outils/pdfium/lib")
                .to_string_lossy()
                .into_owned()
        });
    let pdfium = Pdfium::new(Pdfium::bind_to_library(
        Pdfium::pdfium_platform_library_name_at_path(&pdfium_dir),
    )?);
    let mut document = pdfium.create_new_pdf()?;
    let p = Polices {
        normale: document.fonts_mut().helvetica(),
        grasse: document.fonts_mut().helvetica_bold(),
        italique: document.fonts_mut().helvetica_oblique(),
    };

    // Page 1 — titre, césure, vrai trait d'union, terme anglais, appel de note.
    let mut page = document
        .pages_mut()
        .create_page_at_end(PdfPagePaperSize::a4())?;
    texte(&mut page, 72.0, 80.0, "1. INTRODUCTION", p.grasse, 16.0);
    let y = lignes(
        &mut page,
        &p,
        72.0,
        120.0,
        &[
            "Les négociations climatiques réunissent chaque année les Parties à la",
            "Convention-cadre des Nations Unies. Cette année, les négo-",
            "ciations reprennent à Belém, sous la présidence brésilienne, et la",
            "Convention-",
            "cadre fixe le cadre du dialogue.",
        ],
    );
    let x = texte(
        &mut page,
        72.0,
        y + 6.0,
        "Le bilan mondial (en anglais ",
        p.normale,
        11.0,
    );
    let x = texte(
        &mut page,
        x + 3.0,
        y + 6.0,
        "Global Stocktake",
        p.italique,
        11.0,
    );
    texte(
        &mut page,
        x,
        y + 6.0,
        ") est adopté à Dubaï.",
        p.normale,
        11.0,
    );
    let x = texte(
        &mut page,
        72.0,
        y + 26.5,
        "Aucune CdP n'a eu lieu en 2020",
        p.normale,
        11.0,
    );
    let x = texte(&mut page, x, y + 22.5, "1", p.normale, 7.0);
    texte(
        &mut page,
        x,
        y + 26.5,
        ". La suite du texte reprend ici.",
        p.normale,
        11.0,
    );
    filet(&mut page, 72.0, 740.0, 216.0, 740.0);
    texte(&mut page, 72.0, 752.0, "1", p.normale, 5.0);
    texte(
        &mut page,
        78.0,
        755.0,
        "Une note de bas de page, entière.",
        p.normale,
        8.0,
    );
    pied(&mut page, &p, 1);

    // Page 2 — deux colonnes, écrites l'une après l'autre, puis la pleine largeur.
    let mut page = document
        .pages_mut()
        .create_page_at_end(PdfPagePaperSize::a4())?;
    lignes(
        &mut page,
        &p,
        72.0,
        100.0,
        &[
            "Colonne gauche, première",
            "ligne, puis la deuxième et",
            "la fin de la gauche.",
        ],
    );
    lignes(
        &mut page,
        &p,
        315.0,
        100.0,
        &[
            "Colonne droite, première",
            "ligne, puis la deuxième et",
            "la fin de la droite.",
        ],
    );
    lignes(
        &mut page,
        &p,
        72.0,
        170.0,
        &["Un paragraphe pleine largeur suit les deux colonnes."],
    );
    pied(&mut page, &p, 2);

    // Page 3 — un tableau, entre deux paragraphes.
    let mut page = document
        .pages_mut()
        .create_page_at_end(PdfPagePaperSize::a4())?;
    lignes(
        &mut page,
        &p,
        72.0,
        100.0,
        &["Le tableau suivant résume les décisions."],
    );
    for y in [120.0, 150.0, 180.0] {
        filet(&mut page, 72.0, y, 523.0, y);
    }
    for x in [72.0, 250.0, 523.0] {
        filet(&mut page, x, 120.0, x, 180.0);
    }
    texte(&mut page, 80.0, 139.0, "Thématique", p.grasse, 11.0);
    texte(&mut page, 258.0, 139.0, "Décision", p.grasse, 11.0);
    texte(&mut page, 80.0, 169.0, "Adaptation", p.normale, 11.0);
    texte(
        &mut page,
        258.0,
        169.0,
        "Cent indicateurs adoptés",
        p.normale,
        11.0,
    );
    lignes(
        &mut page,
        &p,
        72.0,
        210.0,
        &["Après le tableau, le texte reprend."],
    );
    pied(&mut page, &p, 3);

    // Page 4 — une liste à puces, sous un titre de section.
    let mut page = document
        .pages_mut()
        .create_page_at_end(PdfPagePaperSize::a4())?;
    texte(&mut page, 72.0, 80.0, "1.1. Les enjeux", p.grasse, 12.0);
    for (i, element) in [
        "Finaliser les indicateurs.",
        "Renforcer les plans nationaux.",
    ]
    .iter()
    .enumerate()
    {
        let y = 110.0 + 20.0 * i as f32;
        texte(&mut page, 90.0, y, "•", p.normale, 11.0);
        texte(&mut page, 108.0, y, element, p.normale, 11.0);
    }
    pied(&mut page, &p, 4);

    let sortie = racine.join("tests/fixtures/petit.pdf");
    std::fs::create_dir_all(sortie.parent().expect("dossier des fixtures"))?;
    document.save_to_file(&sortie)?;
    println!("{}", sortie.display());
    Ok(())
}
