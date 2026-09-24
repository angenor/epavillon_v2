//! L'en-tête `Range` d'une lecture du PDF, analysé sans rien lire.
//!
//! RFC 9110 § 14 : une forme illisible s'ignore et le fichier part entier ; une
//! plage qui commence au-delà du fichier est refusée (416). De plusieurs plages,
//! seule la première est servie — ce que la RFC permet, et pdf.js n'en demande
//! jamais plus d'une.

/// Ce que la requête demande d'un fichier de `taille` octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plage {
    Entier,
    /// Bornes comprises, `fin` ramenée au dernier octet.
    Partie {
        debut: u64,
        fin: u64,
    },
    HorsDuFichier,
}

pub fn analyser(entete: Option<&str>, taille: u64) -> Plage {
    let Some(valeur) = entete else {
        return Plage::Entier;
    };
    let Some(plages) = valeur.trim().strip_prefix("bytes=") else {
        return Plage::Entier;
    };
    let premiere = plages.split(',').next().unwrap_or_default().trim();
    let Some((a, b)) = premiere.split_once('-') else {
        return Plage::Entier;
    };
    let nombre = |t: &str| -> Option<u64> {
        let t = t.trim();
        (!t.is_empty() && t.bytes().all(|o| o.is_ascii_digit()))
            .then(|| t.parse().ok())
            .flatten()
    };

    match (
        nombre(a),
        nombre(b),
        a.trim().is_empty(),
        b.trim().is_empty(),
    ) {
        // bytes=-n : les n derniers octets.
        (None, Some(n), true, false) => {
            if n == 0 || taille == 0 {
                Plage::HorsDuFichier
            } else {
                Plage::Partie {
                    debut: taille.saturating_sub(n),
                    fin: taille - 1,
                }
            }
        }
        // bytes=a- : jusqu'à la fin.
        (Some(debut), None, false, true) => borner(debut, u64::MAX, taille),
        // bytes=a-b
        (Some(debut), Some(fin), false, false) if debut <= fin => borner(debut, fin, taille),
        _ => Plage::Entier,
    }
}

fn borner(debut: u64, fin: u64, taille: u64) -> Plage {
    if debut >= taille {
        Plage::HorsDuFichier
    } else {
        Plage::Partie {
            debut,
            fin: fin.min(taille - 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{analyser, Plage};

    const TAILLE: u64 = 1000;

    fn partie(debut: u64, fin: u64) -> Plage {
        Plage::Partie { debut, fin }
    }

    #[test]
    fn sans_entete_le_fichier_part_entier() {
        assert_eq!(analyser(None, TAILLE), Plage::Entier);
    }

    #[test]
    fn les_trois_formes_d_une_plage() {
        assert_eq!(analyser(Some("bytes=0-255"), TAILLE), partie(0, 255));
        assert_eq!(analyser(Some("bytes=900-"), TAILLE), partie(900, 999));
        assert_eq!(analyser(Some("bytes=-100"), TAILLE), partie(900, 999));
        assert_eq!(analyser(Some("bytes=999-999"), TAILLE), partie(999, 999));
    }

    #[test]
    fn une_fin_au_dela_s_arrete_au_dernier_octet() {
        assert_eq!(analyser(Some("bytes=512-5000"), TAILLE), partie(512, 999));
        assert_eq!(analyser(Some("bytes=-5000"), TAILLE), partie(0, 999));
    }

    #[test]
    fn de_plusieurs_plages_la_premiere_seule() {
        assert_eq!(analyser(Some("bytes=0-99, 200-299"), TAILLE), partie(0, 99));
    }

    #[test]
    fn un_debut_au_dela_du_fichier_est_refuse() {
        assert_eq!(analyser(Some("bytes=1000-"), TAILLE), Plage::HorsDuFichier);
        assert_eq!(
            analyser(Some("bytes=2000-3000"), TAILLE),
            Plage::HorsDuFichier
        );
        assert_eq!(analyser(Some("bytes=-0"), TAILLE), Plage::HorsDuFichier);
        assert_eq!(analyser(Some("bytes=0-10"), 0), Plage::HorsDuFichier);
    }

    #[test]
    fn une_forme_illisible_s_ignore() {
        for forme in [
            "octets=0-10",
            "bytes=",
            "bytes=-",
            "bytes=abc-10",
            "bytes=10-5",
            "bytes=1e3-",
            "bytes=0-10-20",
        ] {
            assert_eq!(analyser(Some(forme), TAILLE), Plage::Entier, "{forme}");
        }
    }
}
