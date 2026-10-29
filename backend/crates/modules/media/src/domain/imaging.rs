//! Décoder, mesurer, redimensionner — **sans base, sans réseau, sans état**.
//!
//! Tout ce que le traitement différé fait de coûteux vit ici, en fonctions
//! synchrones : c'est ce qui permet à [`crate::jobs::process`] de les confier à
//! une **tâche bloquante dédiée**, comme B1 le fait du hachage de mot de passe.
//! Quelques centaines de millisecondes de redimensionnement sur le fil d'exécution
//! asynchrone bloqueraient tout ce qui le partage.
//!
//! # La transparence se mesure sur les pixels, pas sur l'en-tête
//!
//! Un PNG peut porter un canal alpha entièrement opaque — c'est le cas de la
//! plupart des exports d'outils de dessin. Se fier au type de couleur
//! produirait alors un PNG là où un JPEG serait deux à cinq fois plus léger,
//! sur des images de conférence qui n'ont aucune transparence. On regarde donc
//! les pixels ; le décodage a déjà eu lieu, et le parcours est négligeable
//! devant lui.

use image::{DynamicImage, GenericImageView};

use crate::domain::asset::RenditionFormat;

/// Ce qu'un relevé rend d'une image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    /// Au moins un pixel n'est pas entièrement opaque.
    pub porte_transparence: bool,
}

/// Décode une image et rend ses dimensions. `None` quand le contenu n'est pas
/// une image décodable — **et ce n'est pas une erreur** : `tg_validate_attachment`
/// laisse passer un objet image sans dimensions relevées, parce que c'est le
/// relevé qui a échoué et non le fichier qui est mal cadré.
pub fn mesurer(octets: &[u8]) -> Option<Dimensions> {
    let image = image::load_from_memory(octets).ok()?;
    let (width, height) = image.dimensions();

    Some(Dimensions {
        width,
        height,
        porte_transparence: porte_transparence(&image),
    })
}

fn porte_transparence(image: &DynamicImage) -> bool {
    if !image.color().has_alpha() {
        return false;
    }
    image.to_rgba8().pixels().any(|pixel| pixel.0[3] < u8::MAX)
}

/// Une déclinaison fabriquée : les octets encodés et la taille obtenue.
#[derive(Debug, Clone)]
pub struct Declinaison {
    pub octets: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Redimensionne à la largeur demandée, hauteur proportionnelle, et encode dans
/// le format donné.
///
/// **Synchrone et coûteuse** : l'appelant l'exécute sur une tâche bloquante.
///
/// Le JPEG n'a pas de canal alpha : l'image y est aplatie en RVB. C'est
/// précisément pourquoi [`crate::domain::variants::format_pour`] ne le choisit
/// que pour une image opaque — aplatir un logo transparent sur du blanc est un
/// défaut visible sur fond sombre.
pub fn redimensionner(
    octets: &[u8],
    largeur_cible: u32,
    format: RenditionFormat,
) -> Result<Declinaison, String> {
    decliner(octets, largeur_cible, None, None, format)
}

/// Comme [`decliner`], la qualité JPEG baissant (jusqu'à 30) tant que le
/// fichier dépasse `poids_max`, puis la largeur (jusqu'à 600 px, le minimum
/// d'un grand aperçu de lien). Sans plafond, ou en PNG, c'est [`decliner`].
pub fn decliner_sous(
    octets: &[u8],
    largeur_cible: u32,
    rapport: Option<(u32, u32)>,
    qualite: Option<u8>,
    poids_max: Option<usize>,
    format: RenditionFormat,
) -> Result<Declinaison, String> {
    let premiere = decliner(octets, largeur_cible, rapport, qualite, format)?;
    let Some(plafond) = poids_max else {
        return Ok(premiere);
    };
    if format != RenditionFormat::Jpeg || premiere.octets.len() <= plafond {
        return Ok(premiere);
    }
    // La plus haute qualité qui tient, par dichotomie : six encodages au plus.
    let (mut basse, mut haute) = (30u8, qualite.unwrap_or(75).saturating_sub(1));
    let mut retenue = None;
    while basse <= haute {
        let milieu = basse + (haute - basse) / 2;
        let essai = decliner(octets, largeur_cible, rapport, Some(milieu), format)?;
        if essai.octets.len() <= plafond {
            retenue = Some(essai);
            basse = milieu + 1;
        } else {
            if milieu == 0 {
                break;
            }
            haute = milieu - 1;
        }
    }
    if let Some(faite) = retenue {
        return Ok(faite);
    }
    let mut largeur = largeur_cible;
    loop {
        largeur = (largeur * 4 / 5).max(600);
        let essai = decliner(octets, largeur, rapport, Some(30), format)?;
        if essai.octets.len() <= plafond || largeur == 600 {
            return Ok(essai);
        }
    }
}

/// Comme [`redimensionner`], avec un recadrage au centre au rapport donné
/// (largeur, hauteur) avant la réduction, et une qualité JPEG.
pub fn decliner(
    octets: &[u8],
    largeur_cible: u32,
    rapport: Option<(u32, u32)>,
    qualite: Option<u8>,
    format: RenditionFormat,
) -> Result<Declinaison, String> {
    let source = image::load_from_memory(octets).map_err(|e| e.to_string())?;
    let (largeur, hauteur) = source.dimensions();
    if largeur == 0 || hauteur == 0 {
        return Err("image de dimension nulle".to_owned());
    }
    let source = match rapport {
        Some((rl, rh)) if rl > 0 && rh > 0 => {
            let (l, h) = (u64::from(largeur), u64::from(hauteur));
            let (rl, rh) = (u64::from(rl), u64::from(rh));
            // La plus grande fenêtre au rapport voulu, centrée.
            let (cl, ch) = if l * rh > h * rl {
                (h * rl / rh, h)
            } else {
                (l, l * rh / rl)
            };
            let (cl, ch) = (cl.max(1) as u32, ch.max(1) as u32);
            source.crop_imm((largeur - cl) / 2, (hauteur - ch) / 2, cl, ch)
        }
        _ => source,
    };
    let (largeur, hauteur) = source.dimensions();
    // Un recadrage plus étroit que la cible n'est pas agrandi.
    let largeur_cible = largeur_cible.min(largeur);

    // La hauteur est calculée plutôt que déduite d'un cadre : `resize` bornerait
    // aussi la hauteur, et une image très haute ressortirait plus étroite que la
    // largeur visée — la déclinaison ne ferait alors plus la taille annoncée.
    let hauteur_cible = ((u64::from(hauteur) * u64::from(largeur_cible)) as f64
        / f64::from(largeur))
    .round() as u32;
    let hauteur_cible = hauteur_cible.max(1);

    let reduite = source.resize_exact(
        largeur_cible,
        hauteur_cible,
        image::imageops::FilterType::Lanczos3,
    );

    let mut sortie = std::io::Cursor::new(Vec::new());
    match format {
        RenditionFormat::Jpeg => match qualite {
            Some(q) => {
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut sortie, q.clamp(1, 100))
                    .encode_image(&DynamicImage::ImageRgb8(reduite.to_rgb8()))
                    .map_err(|e| e.to_string())?
            }
            None => DynamicImage::ImageRgb8(reduite.to_rgb8())
                .write_to(&mut sortie, image::ImageFormat::Jpeg)
                .map_err(|e| e.to_string())?,
        },
        RenditionFormat::Png => DynamicImage::ImageRgba8(reduite.to_rgba8())
            .write_to(&mut sortie, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?,
        autre => {
            return Err(format!(
                "format de déclinaison non produit : {}",
                autre.as_str()
            ))
        }
    }

    Ok(Declinaison {
        octets: sortie.into_inner(),
        width: largeur_cible,
        height: hauteur_cible,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_recadrage_courriel_rend_un_6_1_leger() {
        let source = png(1920, 1080, 255);
        let faite = decliner(&source, 1200, Some((6, 1)), Some(62), RenditionFormat::Jpeg).unwrap();
        assert_eq!((faite.width, faite.height), (1200, 200));
        let mesure = mesurer(&faite.octets).unwrap();
        assert_eq!((mesure.width, mesure.height), (1200, 200));
    }

    /// Du bruit : le pire cas pour un JPEG, que le plafond doit tenir quand même.
    fn bruit(largeur: u32, hauteur: u32) -> Vec<u8> {
        let mut graine: u32 = 7;
        let mut tampon = image::RgbImage::new(largeur, hauteur);
        for pixel in tampon.pixels_mut() {
            graine = graine.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let [a, b, c, _] = graine.to_le_bytes();
            *pixel = image::Rgb([a, b, c]);
        }
        let mut octets = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(tampon)
            .write_to(&mut octets, image::ImageFormat::Png)
            .unwrap();
        octets.into_inner()
    }

    #[test]
    fn la_declinaison_de_partage_tient_sous_son_plafond() {
        let plafond = 100 * 1024;
        let faite = decliner_sous(
            &bruit(1600, 900),
            1200,
            None,
            Some(85),
            Some(plafond),
            RenditionFormat::Jpeg,
        )
        .unwrap();
        assert!(faite.width <= 1200 && faite.width >= 600);
        assert_eq!(
            faite.height,
            (faite.width * 9 + 8) / 16,
            "la forme est gardée"
        );
        assert!(
            faite.octets.len() <= plafond,
            "{} octets",
            faite.octets.len()
        );

        let mut unie = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            1600,
            900,
            image::Rgb([30, 60, 120]),
        ))
        .write_to(&mut unie, image::ImageFormat::Png)
        .unwrap();
        let unie = unie.into_inner();
        let legere = decliner_sous(
            &unie,
            1200,
            None,
            Some(85),
            Some(plafond),
            RenditionFormat::Jpeg,
        )
        .unwrap();
        let directe = decliner(&unie, 1200, None, Some(85), RenditionFormat::Jpeg).unwrap();
        assert_eq!(
            legere.octets.len(),
            directe.octets.len(),
            "sous le plafond, la qualité demandée est gardée"
        );
    }

    fn png(largeur: u32, hauteur: u32, opacite: u8) -> Vec<u8> {
        let mut tampon = image::RgbaImage::new(largeur, hauteur);
        for (x, y, pixel) in tampon.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x % 251) as u8, (y % 241) as u8, 90, opacite]);
        }
        let mut octets = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(tampon)
            .write_to(&mut octets, image::ImageFormat::Png)
            .unwrap();
        octets.into_inner()
    }

    #[test]
    fn les_dimensions_relevees_sont_celles_de_limage() {
        let mesure = mesurer(&png(320, 180, 255)).expect("image décodable");
        assert_eq!((mesure.width, mesure.height), (320, 180));
    }

    /// Un canal alpha entièrement opaque n'est **pas** de la transparence : s'y
    /// fier produirait un PNG là où un JPEG serait bien plus léger.
    #[test]
    fn un_canal_alpha_opaque_ne_compte_pas_comme_transparence() {
        assert!(!mesurer(&png(64, 64, 255)).unwrap().porte_transparence);
        assert!(mesurer(&png(64, 64, 200)).unwrap().porte_transparence);
    }

    #[test]
    fn un_contenu_qui_nest_pas_une_image_ne_se_mesure_pas() {
        assert!(mesurer(b"%PDF-1.4 ceci n'est pas une image").is_none());
    }

    /// La hauteur suit la largeur : une déclinaison qui ne ferait pas la taille
    /// annoncée rendrait le `<picture>` faux.
    #[test]
    fn la_hauteur_reste_proportionnelle() {
        let faite = redimensionner(&png(1600, 400, 255), 800, RenditionFormat::Jpeg).unwrap();
        assert_eq!((faite.width, faite.height), (800, 200));

        let mesure = mesurer(&faite.octets).expect("la déclinaison se relit");
        assert_eq!((mesure.width, mesure.height), (800, 200));
    }

    #[test]
    fn le_png_conserve_la_transparence_que_le_jpeg_aplatit() {
        let source = png(400, 400, 128);

        let en_png = redimensionner(&source, 200, RenditionFormat::Png).unwrap();
        assert!(mesurer(&en_png.octets).unwrap().porte_transparence);

        let en_jpeg = redimensionner(&source, 200, RenditionFormat::Jpeg).unwrap();
        assert!(!mesurer(&en_jpeg.octets).unwrap().porte_transparence);
    }

    /// Les deux formats déclarés par le modèle et **non produits** par ce jalon
    /// (R12) ressortent en refus explicite, jamais en image vide.
    #[test]
    fn les_formats_non_produits_sont_refuses_en_le_disant() {
        let erreur = redimensionner(&png(64, 64, 255), 32, RenditionFormat::Webp).unwrap_err();
        assert!(erreur.contains("webp"), "{erreur}");
    }
}
