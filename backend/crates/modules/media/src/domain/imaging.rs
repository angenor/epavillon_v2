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
    let source = image::load_from_memory(octets).map_err(|e| e.to_string())?;
    let (largeur, hauteur) = source.dimensions();
    if largeur == 0 || hauteur == 0 {
        return Err("image de dimension nulle".to_owned());
    }

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
        RenditionFormat::Jpeg => DynamicImage::ImageRgb8(reduite.to_rgb8())
            .write_to(&mut sortie, image::ImageFormat::Jpeg)
            .map_err(|e| e.to_string())?,
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
