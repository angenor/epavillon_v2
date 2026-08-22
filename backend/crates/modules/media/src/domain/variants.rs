//! Le jeu de déclinaisons, **depuis la configuration**.
//!
//! Le modèle le prescrit lui-même : « la liste vit dans la configuration du
//! worker, pas dans le schéma — ajouter l'AVIF ne migre rien ». Ce fichier est
//! donc la liste, et il est le seul endroit du dépôt qui la porte.
//!
//! # Trois tailles, et un format par objet
//!
//! `lg` 1600 px, `md` 800 px, `thumb` 320 px, à la **largeur**, hauteur
//! proportionnelle. Le format suit la matière : **JPEG si l'image est opaque,
//! PNG si elle porte de la transparence** — un logo aplati sur du blanc est un
//! défaut visible sur fond sombre, et c'est le cas le plus fréquent de la
//! plateforme.
//!
//! **Ni WebP ni AVIF** (B6, R12) : l'encodeur WebP disponible est sans perte,
//! si bien qu'un WebP de photographie de conférence pèse **plus lourd** que son
//! JPEG — l'objectif pris à l'envers. Les deux formats sont déjà déclarés par
//! `media.rendition_format` : le jour venu, les ajouter est une insertion.
//!
//! # Une image plus petite que la taille visée n'est pas agrandie
//!
//! Agrandir n'ajoute aucune information et fait grossir le fichier. La
//! déclinaison est alors simplement absente, et l'écran se replie sur
//! l'original — ce que `AttachedImage.url` porte toujours.

use crate::domain::asset::RenditionFormat;

/// Une déclinaison à produire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variante {
    /// `variant_code` en base. La contrainte impose des minuscules et des
    /// chiffres séparés par des soulignés.
    pub code: &'static str,
    pub largeur_max: u32,
}

pub const VARIANTES: &[Variante] = &[
    Variante {
        code: "lg",
        largeur_max: 1600,
    },
    Variante {
        code: "md",
        largeur_max: 800,
    },
    Variante {
        code: "thumb",
        largeur_max: 320,
    },
];

/// Le format d'encodage d'un objet, choisi une fois pour ses trois tailles.
pub fn format_pour(porte_transparence: bool) -> RenditionFormat {
    if porte_transparence {
        RenditionFormat::Png
    } else {
        RenditionFormat::Jpeg
    }
}

/// Les déclinaisons réellement attendues pour une image d'une largeur donnée.
///
/// C'est ce nombre que l'écran d'avancement compare aux déclinaisons prêtes :
/// annoncer trois attendues pour une image de 200 px laisserait un avancement
/// bloqué à un tiers pour toujours.
pub fn attendues(largeur_source: u32) -> Vec<Variante> {
    VARIANTES
        .iter()
        .copied()
        .filter(|v| largeur_source > v.largeur_max)
        .collect()
}

/// La clé d'objet d'une déclinaison. Préfixée par l'identifiant de l'objet, ce
/// qui la rend unique tous buckets confondus — l'invariant que
/// `ux_renditions_object_key` exige.
pub fn cle_declinaison(asset_id: uuid::Uuid, code: &str, format: RenditionFormat) -> String {
    format!(
        "_renditions/{}/{}.{}",
        asset_id.simple(),
        code,
        format.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_image_plus_petite_que_la_cible_nest_pas_agrandie() {
        assert!(attendues(200).is_empty());
        assert_eq!(attendues(500).len(), 1);
        assert_eq!(attendues(1000).len(), 2);
        assert_eq!(attendues(4000).len(), 3);
    }

    #[test]
    fn la_transparence_choisit_le_format() {
        assert_eq!(format_pour(true), RenditionFormat::Png);
        assert_eq!(format_pour(false), RenditionFormat::Jpeg);
    }

    /// La contrainte `ck` sur `variant_code` : minuscules, chiffres, soulignés.
    #[test]
    fn les_codes_respectent_la_contrainte_du_modele() {
        for v in VARIANTES {
            assert!(
                v.code
                    .bytes()
                    .all(|o| o.is_ascii_lowercase() || o.is_ascii_digit() || o == b'_'),
                "{} ne respecte pas ck_renditions",
                v.code
            );
        }
    }
}
