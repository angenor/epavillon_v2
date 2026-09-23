//! De petits PDF écrits à la main, pour les cas qu'aucun logiciel de mise en
//! page ne produit — sommaire qui boucle, page démesurée ou illisible, document
//! vide, caractère nul — et pour une même page écrite dans plusieurs repères.

// Chaque binaire de test n'en prend qu'une partie.
#![allow(dead_code)]

/// Un PDF dont les objets sont numérotés à partir de 1, le premier étant le
/// catalogue. Les décalages de la table sont exacts : PDFium n'a rien à réparer.
pub fn pdf(objets: &[String]) -> Vec<u8> {
    let mut octets = b"%PDF-1.4\n".to_vec();
    let mut decalages = Vec::with_capacity(objets.len());
    for (i, objet) in objets.iter().enumerate() {
        decalages.push(octets.len());
        octets.extend(format!("{} 0 obj\n{objet}\nendobj\n", i + 1).bytes());
    }
    let table = octets.len();
    octets.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objets.len() + 1).bytes());
    for d in decalages {
        octets.extend(format!("{d:010} 00000 n \n").bytes());
    }
    octets.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{table}\n%%EOF\n",
            objets.len() + 1
        )
        .bytes(),
    );
    octets
}

pub const HELVETICA: &str = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";

/// Un flux au `/Length` exact.
pub fn flux(contenu: &str) -> String {
    format!(
        "<< /Length {} >>\nstream\n{contenu}\nendstream",
        contenu.len()
    )
}

/// Une seule page, écrite en Helvetica sous `/F1`. `entrees` complète son
/// dictionnaire : `/MediaBox`, `/CropBox`, `/Rotate`.
pub fn page_ecrite(entrees: &str, contenu: &str) -> Vec<u8> {
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        format!(
            "<< /Type /Page /Parent 2 0 R {entrees} \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
        ),
        flux(contenu),
        HELVETICA.into(),
    ])
}

/// `nombre` pages vides de la taille donnée, en points.
pub fn pages(nombre: usize, largeur: u32, hauteur: u32) -> Vec<u8> {
    let enfants: Vec<String> = (0..nombre).map(|i| format!("{} 0 R", i + 3)).collect();
    let mut objets = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {nombre} >>",
            enfants.join(" ")
        ),
    ];
    objets.extend(
        (0..nombre).map(|_| {
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {largeur} {hauteur}] >>")
        }),
    );
    pdf(&objets)
}

/// Deux signets A et B : A est son propre premier enfant, et B renvoie à A
/// comme frère suivant.
pub fn sommaire_qui_boucle() -> Vec<u8> {
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".into(),
        "<< /Type /Outlines /First 5 0 R /Last 6 0 R /Count 2 >>".into(),
        "<< /Title (A) /Parent 4 0 R /Next 6 0 R /First 5 0 R /Last 5 0 R /Count 1 /Dest [3 0 R /Fit] >>".into(),
        "<< /Title (B) /Parent 4 0 R /Prev 5 0 R /Next 5 0 R /Dest [3 0 R /Fit] >>".into(),
    ])
}

/// Un arbre qui annonce `annoncees` pages ; chaque enfant est une page, sauf là
/// où `lisibles` dit faux : l'entier 42 à la place du dictionnaire.
pub fn pages_dont_une_illisible(lisibles: &[bool], annoncees: usize) -> Vec<u8> {
    let enfants: Vec<String> = (0..lisibles.len())
        .map(|i| format!("{} 0 R", i + 3))
        .collect();
    let mut objets = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {annoncees} >>",
            enfants.join(" ")
        ),
    ];
    objets.extend(lisibles.iter().map(|&lisible| {
        if lisible {
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".to_owned()
        } else {
            "42".to_owned()
        }
    }));
    pdf(&objets)
}

/// Une page et trois signets : deux visent un numéro de page hors du document,
/// le troisième la page 1 par son numéro (0).
pub fn signets_hors_document() -> Vec<u8> {
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".into(),
        "<< /Type /Outlines /First 5 0 R /Last 7 0 R /Count 3 >>".into(),
        "<< /Title (Loin) /Parent 4 0 R /Next 6 0 R /Dest [41 /Fit] >>".into(),
        "<< /Title (Negatif) /Parent 4 0 R /Prev 5 0 R /Next 7 0 R /Dest [-7 /Fit] >>".into(),
        "<< /Title (Juste) /Parent 4 0 R /Prev 6 0 R /Dest [0 /Fit] >>".into(),
    ])
}

/// U+0000 là où un PDF peut le glisser : un code 0 sans équivalent Unicode dans
/// le texte, le préfixe d'étiquette de la page, le titre d'un signet.
pub fn caractere_nul() -> Vec<u8> {
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /Outlines 6 0 R \
         /PageLabels << /Nums [0 << /S /D /P (L\\000M) >>] >> >>"
            .into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .into(),
        flux("BT /F1 12 Tf 72 700 Td (A\\000B) Tj ET"),
        HELVETICA.into(),
        "<< /Type /Outlines /First 7 0 R /Last 7 0 R /Count 1 >>".into(),
        "<< /Title <FEFF0054000000550000> /Parent 6 0 R /Dest [3 0 R /Fit] >>".into(),
    ])
}

/// Une page paysage de 842 × 595 pt telle qu'on la voit : abscisse et ligne de
/// base depuis le haut, corps, texte.
pub const LIGNES: [(f32, f32, u8, &str); 5] = [
    (72.0, 60.0, 14, "RAPPORT DE LA SESSION"),
    (
        72.0,
        120.0,
        10,
        "Les Parties ont examine le projet de decision",
    ),
    (
        72.0,
        133.0,
        10,
        "et ont convenu de poursuivre les consultations.",
    ),
    (430.0, 300.0, 10, "Une seconde colonne, plus bas."),
    (72.0, 560.0, 8, "Page 1"),
];
/// Un filet de 0,8 pt : les règles reconnaissent un tableau à ses traits fins.
pub const FILET: [f32; 4] = [72.0, 145.0, 500.0, 145.8];

/// La même page paysage, écrite dans le repère que lui imposent `quarts` quarts
/// de tour (`/Rotate`) et une boîte dont le coin bas gauche est (`ox`, `oy`) :
/// la MediaBox, ou une CropBox dans une MediaBox plus grande si `recadree`.
pub fn paysage(quarts: u8, ox: f32, oy: f32, recadree: bool) -> Vec<u8> {
    let (bw, bh) = if quarts.is_multiple_of(2) {
        (842.0, 595.0)
    } else {
        (595.0, 842.0)
    };
    let vers_pdf = |dx: f32, dt: f32| match quarts {
        1 => (ox + dt, oy + dx),
        2 => (ox + bw - dx, oy + dt),
        3 => (ox + bw - dt, oy + bh - dx),
        _ => (ox + dx, oy + bh - dt),
    };
    let axes = ["1 0 0 1", "0 1 -1 0", "-1 0 0 -1", "0 -1 1 0"][usize::from(quarts % 4)];
    let mut contenu = String::new();
    for (dx, dt, corps, texte) in LIGNES {
        let (x, y) = vers_pdf(dx, dt);
        contenu += &format!("BT /F1 {corps} Tf {axes} {x} {y} Tm ({texte}) Tj ET\n");
    }
    let (xa, ya) = vers_pdf(FILET[0], FILET[1]);
    let (xb, yb) = vers_pdf(FILET[2], FILET[3]);
    contenu += &format!(
        "{} {} {} {} re f",
        xa.min(xb),
        ya.min(yb),
        (xa - xb).abs(),
        (ya - yb).abs()
    );
    let boite = format!("[{ox} {oy} {} {}]", ox + bw, oy + bh);
    let entrees = if recadree {
        format!("/MediaBox [-500 -500 2000 2000] /CropBox {boite}")
    } else {
        format!("/MediaBox {boite}")
    };
    page_ecrite(
        &format!("{entrees} /Rotate {}", 90 * u32::from(quarts)),
        &contenu,
    )
}
