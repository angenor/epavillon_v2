//! L'essai d'extraction de specs/011, rejoué avec les règles du crate : il lit
//! un PDF, l'extrait et le rend comme le fera le worker, et écrit dans
//! `.essais/sortie/` la forme lisible, les images de page et un rapport.
//!
//! `cargo run --release -p negotiation --example essai_extraction -- <fichier.pdf> [page…]`
//!
//! Les pages nommées sont rendues bloc par bloc dans le rapport.

use std::{fmt::Write as _, fs, path::Path, time::Instant};

use negotiation::domain::extraction::{self, forme::Block};
use negotiation::pdf::LecteurPdf;

fn rendu(b: &Block) -> String {
    let texte: String = b
        .spans()
        .iter()
        .map(|s| match (s.term, s.italic, s.bold) {
            (true, _, _) => format!("⟦{}⟧", s.text),
            (_, true, _) => format!("_{}_", s.text),
            (_, _, true) => format!("**{}**", s.text),
            _ => s.text.clone(),
        })
        .collect();
    match b {
        Block::Heading { level, .. } => format!("[T{level}] {texte}"),
        Block::Paragraph { .. } => format!("[P] {texte}"),
        Block::ListItem { depth, marker, .. } => format!("[{marker}{depth}] {texte}"),
        Block::Note { mark, .. } => format!("[NOTE {mark}] {texte}"),
        Block::Origin { reason, .. } => format!(
            "[ORIGINE {reason:?}] {}",
            texte.chars().take(120).collect::<String>()
        ),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let pdf = arguments
        .first()
        .ok_or("usage : essai_extraction <fichier.pdf> [page…]")?;
    let temoins: Vec<usize> = arguments[1..]
        .iter()
        .filter_map(|a| a.parse().ok())
        .collect();
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let sortie = racine.join(".essais/sortie");
    fs::create_dir_all(sortie.join("pages"))?;
    let pdfium = std::env::var("PDFIUM_LIB_PATH")
        .ok()
        .filter(|c| Path::new(c).is_absolute())
        .unwrap_or_else(|| {
            racine
                .join(".outils/pdfium/lib")
                .to_string_lossy()
                .into_owned()
        });

    let octets = fs::read(pdf)?;
    let lecteur = LecteurPdf::partage(&pdfium)?;
    let debut = Instant::now();
    let brut = lecteur.lire(&octets)?;
    let lecture = debut.elapsed();
    let debut = Instant::now();
    let extraction = extraction::extraire(&brut);
    let regles = debut.elapsed();
    let debut = Instant::now();
    let mut images = Vec::new();
    lecteur.rendre(&octets, |jpeg| {
        images.push(jpeg);
        Ok(())
    })?;
    let rendu_images = debut.elapsed();

    let mut forme = 0;
    for p in &extraction.pages {
        forme += serde_json::to_string(&p.blocks)?.len();
    }
    let origines: usize = extraction
        .pages
        .iter()
        .zip(&images)
        .filter(|(p, _)| p.has_origin_block)
        .map(|(_, i)| i.len())
        .sum();
    for (i, image) in images.iter().enumerate() {
        fs::write(sortie.join(format!("pages/{}.jpg", i + 1)), image)?;
    }
    let pages: Vec<_> = extraction
        .pages
        .iter()
        .map(|p| serde_json::json!({"index": p.index, "label": p.label, "blocks": p.blocks}))
        .collect();
    fs::write(
        sortie.join("forme-lisible.json"),
        serde_json::to_vec(&serde_json::json!({"outline": extraction.outline, "pages": pages}))?,
    )?;

    let mut r = String::new();
    writeln!(r, "# Extraction de {pdf}\n")?;
    writeln!(
        r,
        "- Pages : {} ; recomposable : {}",
        extraction.pages.len(),
        extraction.is_reflowable
    )?;
    writeln!(
        r,
        "- Indicateurs : {}",
        serde_json::to_string(&extraction.quality)?
    )?;
    writeln!(
        r,
        "- Forme lisible : {forme} octets ; images des pages d'origine : {origines} octets"
    )?;
    writeln!(
        r,
        "- Images : {} octets au total, JPEG {} à {} px",
        images.iter().map(Vec::len).sum::<usize>(),
        negotiation::pdf::QUALITE_JPEG,
        negotiation::pdf::LARGEUR_IMAGE
    )?;
    writeln!(
        r,
        "- Durées : lecture {:.2} s, règles {:.2} s, images {:.2} s\n",
        lecture.as_secs_f32(),
        regles.as_secs_f32(),
        rendu_images.as_secs_f32()
    )?;
    for p in extraction
        .pages
        .iter()
        .filter(|p| temoins.contains(&p.index))
    {
        writeln!(r, "## Page {} — « {} »\n", p.index, p.label)?;
        for b in &p.blocks {
            writeln!(r, "{}\n", rendu(b))?;
        }
    }
    fs::write(sortie.join("rapport.md"), &r)?;
    print!("{r}");
    Ok(())
}
