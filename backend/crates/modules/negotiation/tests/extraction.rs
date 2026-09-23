//! **L'extraction, sur un vrai PDF lu par le vrai PDFium** : `fixtures/petit.pdf`,
//! fabriqué par `examples/fabriquer_petit_pdf.rs`, porte une difficulté du guide
//! de la CdP30 par règle. La forme produite doit tenir le contrat
//! (`contracts/forme-lisible.md`), et chaque règle y est vérifiée.

use negotiation::domain::extraction::{
    self,
    brut::{Cadre, DocumentBrut, PageBrute},
    forme::{texte_de_page, Block, OriginReason},
    Extraction, PageExtraite,
};
mod fabrique;

use negotiation::pdf::{LecteurPdf, HAUTEUR_MAX_IMAGE, LARGEUR_IMAGE, PAGES_MAX};

const PETIT: &[u8] = include_bytes!("fixtures/petit.pdf");

fn lecteur() -> std::sync::Arc<LecteurPdf> {
    let config = kernel::testing::test_config("postgres://inutile");
    let dossier = config
        .negotiation
        .pdfium_lib_path
        .expect("PDFium : faire `make pdfium`");
    LecteurPdf::partage(&dossier).expect("PDFium : faire `make pdfium`")
}

fn extraire() -> Extraction {
    extraction::extraire(&lecteur().lire(PETIT).expect("lecture du PDF"))
}

fn page(e: &Extraction, index: usize) -> &PageExtraite {
    &e.pages[index - 1]
}

fn blocs<'a>(p: &'a PageExtraite, f: impl Fn(&Block) -> bool + 'a) -> Vec<&'a Block> {
    p.blocks.iter().filter(|b| f(b)).collect()
}

#[test]
fn la_forme_tient_les_invariants_du_contrat() {
    let e = extraire();
    assert_eq!(e.pages.len(), 4);
    for (i, p) in e.pages.iter().enumerate() {
        assert_eq!(p.index, i + 1, "index contigus de 1 à N");
        assert_eq!(
            p.plain_text,
            texte_de_page(&p.blocks),
            "rien ne se cherche qui ne s'affiche pas"
        );
    }
    fn pages_du_sommaire(o: &[extraction::forme::OutlineEntry], acc: &mut Vec<usize>) {
        for e in o {
            acc.push(e.page_index);
            pages_du_sommaire(&e.children, acc);
        }
    }
    let mut renvois = Vec::new();
    pages_du_sommaire(&e.outline, &mut renvois);
    assert!(!renvois.is_empty());
    assert!(
        renvois.iter().all(|&i| (1..=4).contains(&i)),
        "le sommaire renvoie à des pages qui existent"
    );
    assert!(e.is_reflowable);
}

#[test]
fn le_pied_repete_est_ecarte_et_son_numero_devient_letiquette() {
    let e = extraire();
    for p in &e.pages {
        assert!(
            !p.plain_text.contains("PETIT GUIDE"),
            "page {} : pied resté",
            p.index
        );
        assert_eq!(p.label, p.index.to_string());
    }
}

#[test]
fn le_titre_la_cesure_le_vrai_tiret_et_le_terme_anglais() {
    let e = extraire();
    let p1 = page(&e, 1);
    assert!(matches!(&p1.blocks[0], Block::Heading { level: 1, .. }));
    assert_eq!(p1.blocks[0].texte(), "1. INTRODUCTION");

    let texte = &p1.plain_text;
    assert!(
        texte.contains("les négociations reprennent"),
        "césure recollée : {texte}"
    );
    assert!(
        texte.contains("la Convention-cadre fixe"),
        "vrai trait d'union gardé : {texte}"
    );
    assert!(!texte.contains('\u{2}'));

    let termes: Vec<String> = p1
        .blocks
        .iter()
        .flat_map(Block::spans)
        .filter(|s| s.term)
        .map(|s| s.text.clone())
        .collect();
    assert_eq!(termes, ["Global Stocktake"]);
    assert!(texte.contains("(en anglais Global Stocktake) est adopté"));
}

#[test]
fn la_note_sort_du_corps_et_lappel_reste_en_exposant() {
    let e = extraire();
    let p1 = page(&e, 1);
    let notes = blocs(p1, |b| matches!(b, Block::Note { .. }));
    assert_eq!(notes.len(), 1);
    let Block::Note { mark, .. } = notes[0] else {
        unreachable!()
    };
    assert_eq!(mark, "1");
    assert_eq!(notes[0].texte(), "Une note de bas de page, entière.");
    assert!(
        matches!(p1.blocks.last(), Some(Block::Note { .. })),
        "la note est rendue en fin de page"
    );
    assert!(p1.plain_text.contains("en 2020¹. La suite"));
}

#[test]
fn deux_colonnes_se_lisent_lune_apres_lautre() {
    let e = extraire();
    let textes: Vec<String> = page(&e, 2).blocks.iter().map(Block::texte).collect();
    assert_eq!(
        textes,
        [
            "Colonne gauche, première ligne, puis la deuxième et la fin de la gauche.",
            "Colonne droite, première ligne, puis la deuxième et la fin de la droite.",
            "Un paragraphe pleine largeur suit les deux colonnes.",
        ]
    );
}

#[test]
fn le_tableau_est_une_origine_et_jamais_des_paragraphes() {
    let e = extraire();
    let p3 = page(&e, 3);
    assert!(p3.has_origin_block);
    let genres: Vec<&str> = p3
        .blocks
        .iter()
        .map(|b| match b {
            Block::Origin {
                reason: OriginReason::Table,
                ..
            } => "tableau",
            Block::Paragraph { .. } => "paragraphe",
            _ => "autre",
        })
        .collect();
    assert_eq!(genres, ["paragraphe", "tableau", "paragraphe"]);
    assert!(
        p3.blocks[1].texte().contains("Cent indicateurs adoptés"),
        "le texte du tableau reste cherchable"
    );
    assert!(
        !p3.blocks[0].texte().contains("Adaptation")
            && !p3.blocks[2].texte().contains("Adaptation")
    );
}

#[test]
fn la_liste_et_le_titre_de_section() {
    let e = extraire();
    let p4 = page(&e, 4);
    assert!(matches!(&p4.blocks[0], Block::Heading { level: 2, .. }));
    let elements: Vec<(String, String)> = p4
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::ListItem {
                marker, depth: 0, ..
            } => Some((marker.clone(), b.texte())),
            _ => None,
        })
        .collect();
    assert_eq!(
        elements,
        [
            ("•".to_owned(), "Finaliser les indicateurs.".to_owned()),
            ("•".to_owned(), "Renforcer les plans nationaux.".to_owned()),
        ]
    );
}

fn rendre(octets: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let mut images = Vec::new();
    let nombre = lecteur().rendre(octets, |jpeg| {
        images.push(jpeg);
        Ok(())
    })?;
    assert_eq!(nombre, images.len());
    Ok(images)
}

#[test]
fn chaque_page_se_rend_en_jpeg() {
    let images = rendre(PETIT).expect("rendu");
    assert_eq!(images.len(), 4);
    assert!(images.iter().all(|i| i.starts_with(&[0xFF, 0xD8])));
}

#[test]
fn un_sommaire_qui_boucle_se_lit_une_fois_et_sarrete() {
    let brut = lecteur()
        .lire(&fabrique::sommaire_qui_boucle())
        .expect("lecture");
    let titres: Vec<(&str, u8)> = brut
        .signets
        .iter()
        .map(|s| (s.titre.as_str(), s.niveau))
        .collect();
    assert_eq!(titres, [("A", 1), ("B", 1)]);
}

#[test]
fn une_page_etroite_et_tres_haute_saffine_au_lieu_de_grandir() {
    let images = rendre(&fabrique::pages(1, 20, 1300)).expect("rendu");
    let image = image::load_from_memory(&images[0]).expect("JPEG");
    assert!(
        image.height() <= HAUTEUR_MAX_IMAGE as u32,
        "{}",
        image.height()
    );
    assert!(image.width() < LARGEUR_IMAGE as u32);
}

#[test]
fn un_pdf_sans_page_est_refuse_avec_un_motif() {
    let motif = lecteur()
        .lire(&fabrique::pages(0, 200, 200))
        .expect_err("refus");
    assert!(motif.contains("aucune page"), "{motif}");
}

#[test]
fn un_pdf_demesure_est_refuse_avant_tout_rendu() {
    let mut rendues = 0;
    let motif = lecteur()
        .rendre(&fabrique::pages(PAGES_MAX + 1, 10, 10), |_| {
            rendues += 1;
            Ok(())
        })
        .expect_err("refus");
    assert!(motif.contains(&PAGES_MAX.to_string()), "{motif}");
    assert_eq!(rendues, 0);
}

#[test]
fn un_fichier_qui_nest_pas_un_pdf_est_refuse_avec_un_motif() {
    let motif = lecteur().lire(b"ceci n'est pas un PDF").expect_err("refus");
    assert!(motif.contains("PDF"), "{motif}");
}

#[test]
fn une_page_qui_ne_se_charge_pas_refuse_le_document_au_lieu_de_le_tronquer() {
    for (lisibles, annoncees, illisible) in [(&[false][..], 2, 1), (&[true, false, true][..], 3, 2)]
    {
        let octets = fabrique::pages_dont_une_illisible(lisibles, annoncees);
        let attendu = format!("la page {illisible} ne se charge pas");
        let motif = lecteur().lire(&octets).expect_err("lecture refusée");
        assert!(motif.contains(&attendu), "{motif}");
        let motif = rendre(&octets).expect_err("rendu refusé");
        assert!(motif.contains(&attendu), "{motif}");
    }
}

#[test]
fn un_signet_qui_vise_une_page_hors_du_document_ne_renvoie_nulle_part() {
    let brut = lecteur()
        .lire(&fabrique::signets_hors_document())
        .expect("lecture");
    let renvois: Vec<(&str, Option<usize>)> = brut
        .signets
        .iter()
        .map(|s| (s.titre.as_str(), s.page_indice))
        .collect();
    assert_eq!(
        renvois,
        [("Loin", None), ("Negatif", None), ("Juste", Some(1))]
    );
}

#[test]
fn le_caractere_nul_ne_sort_ni_du_texte_ni_de_letiquette_ni_du_sommaire() {
    let brut = lecteur().lire(&fabrique::caractere_nul()).expect("lecture");
    let p = &brut.pages[0];
    let texte: String = p.segments.iter().map(|s| s.texte.as_str()).collect();
    assert_eq!(texte, "AB");
    assert_eq!(p.etiquette.as_deref(), Some("LM1"));
    assert!(
        !brut.signets[0].titre.contains('\0'),
        "{:?}",
        brut.signets[0].titre
    );

    let e = extraction::extraire(&brut);
    let forme = serde_json::to_string(&(&e.outline, &e.pages[0].blocks)).expect("JSON");
    assert!(!forme.contains("\\u0000"), "{forme}");
    assert!(!e.pages[0].plain_text.contains('\0') && !e.pages[0].label.contains('\0'));
}

fn lire_paysage(quarts: u8, ox: f32, oy: f32, recadree: bool) -> PageBrute {
    let mut brut = lecteur()
        .lire(&fabrique::paysage(quarts, ox, oy, recadree))
        .expect("lecture");
    brut.pages.remove(0)
}

/// Au dixième de point : un filet de 0,8 pt doit rester un filet.
fn proche(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

fn cadres_proches(a: &Cadre, b: &Cadre) -> bool {
    proche(a.x0, b.x0) && proche(a.y0, b.y0) && proche(a.x1, b.x1) && proche(a.y1, b.y1)
}

fn meme_page(natif: &PageBrute, lue: &PageBrute, cas: &str) {
    assert_eq!(
        (lue.largeur, lue.hauteur),
        (natif.largeur, natif.hauteur),
        "{cas}"
    );
    let textes =
        |p: &PageBrute| -> Vec<String> { p.segments.iter().map(|s| s.texte.clone()).collect() };
    assert_eq!(textes(lue), textes(natif), "{cas}");
    for (l, n) in lue.segments.iter().zip(&natif.segments) {
        assert_eq!(
            (l.taille, l.fin_de_ligne),
            (n.taille, n.fin_de_ligne),
            "{cas} : {}",
            l.texte
        );
        assert!(
            proche(l.base, n.base),
            "{cas} : {} base {} ≠ {}",
            l.texte,
            l.base,
            n.base
        );
        assert!(
            cadres_proches(&l.cadre, &n.cadre),
            "{cas} : {} {:?} ≠ {:?}",
            l.texte,
            l.cadre,
            n.cadre
        );
    }
    assert_eq!(lue.objets.len(), natif.objets.len(), "{cas}");
    for (l, n) in lue.objets.iter().zip(&natif.objets) {
        assert_eq!(l.genre, n.genre, "{cas}");
        assert!(
            cadres_proches(&l.cadre, &n.cadre),
            "{cas} : {:?} ≠ {:?}",
            l.cadre,
            n.cadre
        );
    }
    let forme = |p: &PageBrute| {
        let e = extraction::extraire(&DocumentBrut {
            pages: vec![p.clone()],
            signets: vec![],
        });
        let p = &e.pages[0];
        (
            p.plain_text.clone(),
            p.blocks.iter().map(Block::texte).collect::<Vec<_>>(),
        )
    };
    assert_eq!(forme(lue), forme(natif), "{cas}");
}

#[test]
fn la_page_paysage_native_se_lit_a_sa_place() {
    let p = lire_paysage(0, 0.0, 0.0, false);
    assert_eq!((p.largeur, p.hauteur), (842.0, 595.0));
    let lues: Vec<(f32, f32, &str)> = p
        .segments
        .iter()
        .map(|s| (s.cadre.x0, s.base, s.texte.as_str()))
        .collect();
    assert_eq!(lues.len(), fabrique::LIGNES.len());
    for ((x, base, texte), (dx, dt, _, attendu)) in lues.into_iter().zip(fabrique::LIGNES) {
        assert_eq!(texte, attendu);
        assert!(proche(x, dx) && proche(base, dt), "{texte} : ({x}, {base})");
    }
    let [x0, y0, x1, y1] = fabrique::FILET;
    assert!(
        cadres_proches(&p.objets[0].cadre, &Cadre::new(x0, y0, x1, y1)),
        "{:?}",
        p.objets[0]
    );
    let e = extraction::extraire(&DocumentBrut {
        pages: vec![p],
        signets: vec![],
    });
    assert!(
        e.pages[0].plain_text.contains("RAPPORT DE LA SESSION"),
        "{}",
        e.pages[0].plain_text
    );
}

#[test]
fn une_page_tournee_se_lit_comme_la_meme_page_en_paysage_natif() {
    let natif = lire_paysage(0, 0.0, 0.0, false);
    for quarts in 1..4 {
        meme_page(
            &natif,
            &lire_paysage(quarts, 0.0, 0.0, false),
            &format!("/Rotate {}", 90 * u32::from(quarts)),
        );
    }
}

#[test]
fn une_boite_qui_ne_part_pas_de_lorigine_se_lit_comme_celle_qui_en_part() {
    let natif = lire_paysage(0, 0.0, 0.0, false);
    for (quarts, ox, oy, recadree) in [
        (0, 0.0, 842.0, false),
        (0, -300.0, 125.5, false),
        (0, 30.0, 40.0, true),
        (1, 30.0, 40.0, true),
        (2, 0.0, 842.0, false),
        (3, -20.5, 60.0, true),
    ] {
        meme_page(
            &natif,
            &lire_paysage(quarts, ox, oy, recadree),
            &format!(
                "/Rotate {} boîte en ({ox}, {oy}) recadrée {recadree}",
                90 * u32::from(quarts)
            ),
        );
    }
}
