//! La durée d'un média temporel, lue **dans l'en-tête du conteneur**.
//!
//! # Pourquoi soixante lignes plutôt qu'une dépendance
//!
//! `media.assets.duration_seconds` existe, et le relevé est ce que le travail
//! différé doit poser (FR-025). Le seul décodeur du dépôt est `image`, qui ne
//! sait rien des médias temporels ; brancher `ffmpeg` ferait entrer un binaire
//! externe dans la chaîne de compilation pour une colonne.
//!
//! Or la durée d'un MP4 n'est pas dans les données : elle est en clair dans la
//! boîte `mvhd` de l'en-tête `moov`, sous forme d'un nombre de graduations et
//! d'une cadence. La lire coûte un parcours de quelques boîtes, sans jamais
//! décoder une image.
//!
//! # Ce que ce fichier NE fait PAS, et le dit
//!
//! **Seul le conteneur ISO-BMFF est lu** — `.mp4`, `.m4a`, `.mov`. Un WebM ou un
//! MP3 rend `None`, et la colonne reste nulle : c'est une absence de relevé,
//! déclarée, jamais une durée inventée. Ce sont les formats que la plateforme
//! produit — enregistrements de séance, fonds vidéo — et le jour où un autre
//! arrive, il s'ajoute ici.

/// Le préfixe MIME des médias dont une durée a un sens.
pub fn est_temporel(mime_type: &str) -> bool {
    mime_type.starts_with("video/") || mime_type.starts_with("audio/")
}

/// La durée en secondes, à la milliseconde — la précision de
/// `numeric(10,3)`. `None` quand le conteneur n'est pas lisible ou qu'il ne
/// déclare pas de durée.
///
/// **Zéro n'est jamais rendu** : `ck` sur la colonne exige une durée
/// strictement positive, et un flux dont la durée est inconnue la déclare
/// justement à zéro.
pub fn duree_secondes(octets: &[u8]) -> Option<f64> {
    let moov = boite(octets, b"moov")?;
    let mvhd = boite(moov, b"mvhd")?;

    // `mvhd` : version (1) + drapeaux (3), puis deux instants, la cadence et la
    // durée — sur 32 bits en version 0, sur 64 en version 1.
    let version = *mvhd.first()?;
    let (cadence, graduations) = match version {
        0 => (
            u32::from_be_bytes(mvhd.get(12..16)?.try_into().ok()?) as u64,
            u32::from_be_bytes(mvhd.get(16..20)?.try_into().ok()?) as u64,
        ),
        1 => (
            u32::from_be_bytes(mvhd.get(20..24)?.try_into().ok()?) as u64,
            u64::from_be_bytes(mvhd.get(24..32)?.try_into().ok()?),
        ),
        _ => return None,
    };

    if cadence == 0 || graduations == 0 {
        return None;
    }

    let secondes = graduations as f64 / cadence as f64;
    let arrondie = (secondes * 1000.0).round() / 1000.0;
    (arrondie > 0.0).then_some(arrondie)
}

/// Le contenu d'une boîte de ce type, cherchée parmi les boîtes de ce niveau.
///
/// Ne descend pas : `moov` est au premier niveau, `mvhd` au premier niveau de
/// `moov`. Un parcours récursif ouvrirait la porte à une entrée forgée qui
/// ferait boucler la lecture d'un fichier déposé par un tiers.
fn boite<'a>(contenu: &'a [u8], type_cherche: &[u8; 4]) -> Option<&'a [u8]> {
    let mut position = 0_usize;

    while position + 8 <= contenu.len() {
        let taille = u32::from_be_bytes(contenu.get(position..position + 4)?.try_into().ok()?);
        let type_boite = contenu.get(position + 4..position + 8)?;

        // Une taille inférieure à l'en-tête ferait boucler la lecture ; `0`
        // signifie « jusqu'à la fin » et `1` une taille sur 64 bits, que ce
        // lecteur ne traite pas.
        let taille = taille as usize;
        if taille < 8 || position + taille > contenu.len() {
            return None;
        }

        if type_boite == type_cherche {
            return contenu.get(position + 8..position + taille);
        }
        position += taille;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un MP4 réduit à ce que ce lecteur regarde : `ftyp`, puis `moov/mvhd`.
    fn mp4(cadence: u32, graduations: u32) -> Vec<u8> {
        let mut mvhd = Vec::new();
        mvhd.extend_from_slice(&[0, 0, 0, 0]); // version 0, aucun drapeau
        mvhd.extend_from_slice(&0_u32.to_be_bytes()); // création
        mvhd.extend_from_slice(&0_u32.to_be_bytes()); // modification
        mvhd.extend_from_slice(&cadence.to_be_bytes());
        mvhd.extend_from_slice(&graduations.to_be_bytes());
        mvhd.extend_from_slice(&[0_u8; 80]); // le reste, que ce lecteur ignore

        let mut moov = Vec::new();
        moov.extend_from_slice(&((mvhd.len() + 8) as u32).to_be_bytes());
        moov.extend_from_slice(b"mvhd");
        moov.extend_from_slice(&mvhd);

        let mut fichier = Vec::new();
        fichier.extend_from_slice(&20_u32.to_be_bytes());
        fichier.extend_from_slice(b"ftypisom");
        fichier.extend_from_slice(&[0_u8; 8]);
        fichier.extend_from_slice(&((moov.len() + 8) as u32).to_be_bytes());
        fichier.extend_from_slice(b"moov");
        fichier.extend_from_slice(&moov);
        fichier
    }

    #[test]
    fn la_duree_se_lit_dans_len_tete() {
        // 1000 graduations par seconde, 92 500 graduations : 92,5 secondes.
        assert_eq!(duree_secondes(&mp4(1000, 92_500)), Some(92.5));
    }

    #[test]
    fn une_duree_inconnue_ne_devient_pas_zero() {
        assert_eq!(duree_secondes(&mp4(1000, 0)), None);
        assert_eq!(duree_secondes(&mp4(0, 1000)), None);
    }

    #[test]
    fn un_contenu_qui_nest_pas_un_conteneur_rend_labsence() {
        assert_eq!(duree_secondes(b"%PDF-1.4 rien a voir"), None);
        assert_eq!(duree_secondes(&[]), None);
    }

    /// Une taille de boîte fautive ne doit pas faire boucler la lecture d'un
    /// fichier déposé par un tiers.
    #[test]
    fn une_boite_de_taille_fautive_arrete_la_lecture() {
        let mut forge = mp4(1000, 1000);
        forge[0..4].copy_from_slice(&0_u32.to_be_bytes());
        assert_eq!(duree_secondes(&forge), None);
    }

    #[test]
    fn seuls_les_medias_temporels_sont_sondes() {
        assert!(est_temporel("video/mp4"));
        assert!(est_temporel("audio/mpeg"));
        assert!(!est_temporel("image/png"));
        assert!(!est_temporel("application/pdf"));
    }
}
