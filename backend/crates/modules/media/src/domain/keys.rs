//! La convention de clé d'objet, **pure**.
//!
//! `<année>/<mois>/<uuid>/<nom-normalisé>.<ext>` — c'est le modèle qui l'écrit,
//! en commentaire de `media.assets.object_key`, et c'est lui qui décide.
//!
//! # Ce que la base refuse, et que la normalisation doit donc garantir
//!
//! `ck` sur `object_key` : entre 1 et 1024 signes, **ni barre oblique
//! initiale**, **aucun caractère d'espacement**. Une clé fautive sortirait en
//! 500 sur une contrainte de vérification — un défaut du service présenté comme
//! une panne.
//!
//! # Pourquoi l'identifiant est dans le chemin
//!
//! Deux personnes déposent « logo.png » le même mois. Sans l'identifiant, la
//! seconde écraserait la première **sur le stockage** — et l'index unique
//! `ux_assets_object` refuserait l'écriture en base après que l'objet a été
//! écrasé. L'ordre des dégâts est le pire possible.

use time::OffsetDateTime;
use uuid::Uuid;

/// Longueur maximale du nom normalisé, extension comprise. Le reste du chemin
/// coûte une soixantaine de signes ; la contrainte de base en accepte 1024.
const NOM_MAX: usize = 160;

/// Nom de repli quand il ne reste rien du nom d'origine — un fichier nommé
/// « ??? » existe, et il ne doit pas produire une clé finissant par une barre.
const NOM_DEFAUT: &str = "fichier";

/// La clé définitive d'un objet.
pub fn cle_objet(depose_le: OffsetDateTime, asset_id: Uuid, nom_fichier: &str) -> String {
    format!(
        "{:04}/{:02}/{}/{}",
        depose_le.year(),
        u8::from(depose_le.month()),
        asset_id.simple(),
        normaliser_nom(nom_fichier)
    )
}

/// La clé **temporaire** d'un flux en cours de réception.
///
/// Le dépôt écrit d'abord ici, calcule l'empreinte au passage, puis renomme ou
/// supprime selon que le contenu est déjà connu (B6, R10). Le préfixe la range
/// à part : un balayage du stockage distingue ainsi un flux interrompu d'un
/// objet vivant.
pub fn cle_temporaire(jeton: Uuid) -> String {
    format!("_incoming/{}", jeton.simple())
}

/// Un nom de fichier réduit à ce qu'une clé d'objet accepte : minuscules sans
/// accent, chiffres, tiret, point de l'extension. Tout le reste devient un
/// tiret, et les tirets consécutifs se réduisent à un.
pub fn normaliser_nom(nom: &str) -> String {
    let (base, extension) = separer_extension(nom);
    let base = translitterer(base);
    let extension = translitterer(extension);

    let mut base: String = base.trim_matches('-').to_owned();
    if base.is_empty() {
        base = NOM_DEFAUT.to_owned();
    }

    let extension = extension.trim_matches('-');
    let place = NOM_MAX.saturating_sub(if extension.is_empty() {
        0
    } else {
        extension.len() + 1
    });
    // Tronquer sur les octets est sans risque : `translitterer` ne rend que de
    // l'ASCII, où un octet vaut un signe.
    base.truncate(place.max(1));
    let base = base.trim_end_matches('-');
    let base = if base.is_empty() { NOM_DEFAUT } else { base };

    if extension.is_empty() {
        base.to_owned()
    } else {
        format!("{base}.{extension}")
    }
}

/// Le dernier point sépare le nom de son extension — et seulement s'il reste
/// quelque chose des deux côtés : « .gitignore » n'a pas d'extension, il a un
/// nom qui commence par un point.
fn separer_extension(nom: &str) -> (&str, &str) {
    match nom.rsplit_once('.') {
        Some((base, ext)) if !base.is_empty() && !ext.is_empty() && ext.len() <= 12 => (base, ext),
        _ => (nom, ""),
    }
}

/// Les accents français sont **repliés**, jamais supprimés : « présentation »
/// doit rester lisible dans une clé, ce qui n'arriverait pas si l'on effaçait
/// tout signe non ASCII.
fn translitterer(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars() {
        // `to_ascii_lowercase` ne touche pas aux lettres accentuées : « É »
        // resterait majuscule, ne correspondrait à aucun repli, et deviendrait
        // un tiret — « Élévation » donnant « levation ».
        let minuscule = c.to_lowercase().next().unwrap_or(c);
        let remplacement = match minuscule {
            c @ ('a'..='z' | '0'..='9') => Some(c),
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => Some('a'),
            'ç' => Some('c'),
            'è' | 'é' | 'ê' | 'ë' => Some('e'),
            'ì' | 'í' | 'î' | 'ï' => Some('i'),
            'ñ' => Some('n'),
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => Some('o'),
            'ù' | 'ú' | 'û' | 'ü' => Some('u'),
            'ý' | 'ÿ' => Some('y'),
            'æ' => {
                sortie.push_str("ae");
                None
            }
            'œ' => {
                sortie.push_str("oe");
                None
            }
            _ => Some('-'),
        };
        match remplacement {
            Some('-') if sortie.ends_with('-') => {}
            Some(c) => sortie.push(c),
            None => {}
        }
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn la_cle_suit_la_convention_du_modele() {
        let id = Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap();
        let cle = cle_objet(datetime!(2026-08-21 10:00 UTC), id, "Bandeau COP31.PNG");
        assert_eq!(
            cle,
            "2026/08/018f0000000070008000000000000001/bandeau-cop31.png"
        );
    }

    /// Les trois refus de `ck` sur `object_key` : barre initiale, espace, vide.
    #[test]
    fn la_cle_ne_porte_ni_barre_initiale_ni_espace() {
        let cle = cle_objet(
            datetime!(2026-01-05 00:00 UTC),
            Uuid::nil(),
            "/dossiers/ma présentation finale.pdf",
        );
        assert!(!cle.starts_with('/'));
        assert!(!cle.contains(char::is_whitespace));
        assert!(cle.ends_with("/dossiers-ma-presentation-finale.pdf"));
    }

    #[test]
    fn les_accents_sont_replies_et_non_effaces() {
        assert_eq!(
            normaliser_nom("Élévation-Sénégal.JPEG"),
            "elevation-senegal.jpeg"
        );
        assert_eq!(normaliser_nom("cœur & âme.png"), "coeur-ame.png");
    }

    #[test]
    fn un_nom_sans_rien_de_lisible_garde_un_nom() {
        assert_eq!(normaliser_nom("???"), "fichier");
        assert_eq!(normaliser_nom(""), "fichier");
        assert_eq!(normaliser_nom("...."), "fichier");
    }

    /// Un point initial n'est pas une extension : « .gitignore » est un nom.
    #[test]
    fn un_nom_qui_commence_par_un_point_na_pas_dextension() {
        assert_eq!(normaliser_nom(".gitignore"), "gitignore");
    }

    #[test]
    fn un_nom_tres_long_est_tronque_sans_perdre_son_extension() {
        let nom = format!("{}.png", "a".repeat(400));
        let sortie = normaliser_nom(&nom);
        assert!(sortie.len() <= NOM_MAX, "{} signes", sortie.len());
        assert!(sortie.ends_with(".png"));
    }

    #[test]
    fn la_cle_temporaire_est_rangee_a_part() {
        assert!(cle_temporaire(Uuid::nil()).starts_with("_incoming/"));
    }
}
