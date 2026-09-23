//! L'ordre de lecture est celui du flux du PDF. Seules les formes flottantes —
//! zones de texte, schémas —, que l'outil de mise en page écrit en fin de page,
//! s'en écartent : elles se repèrent ici et se replacent à leur hauteur.

use super::entetes::en_marge;
use super::lignes::Ligne;

/// Les lignes écrites **après** une ligne qu'elles surplombent, dans les mêmes
/// abscisses. Une colonne de droite remonte aussi, mais sur d'autres abscisses.
/// Les marges ne comptent pas : un pied non écarté ouvre souvent le flux.
pub fn flottantes(lignes: &[Ligne], hauteur: f32) -> Vec<usize> {
    lignes
        .iter()
        .enumerate()
        .filter(|(i, l)| {
            lignes[..*i]
                .iter()
                .filter(|e| !en_marge(e, hauteur))
                .any(|e| e.cadre.y0 > l.cadre.y1 + 2.0 && l.chevauche_en_x(&e.cadre))
        })
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::lignes::{construire, essai::*};
    use super::*;

    #[test]
    fn deux_colonnes_ne_sont_pas_flottantes() {
        let lignes = construire(&[
            fin(seg("gauche un", 72.0, 100.0)),
            fin(seg("gauche deux", 72.0, 114.0)),
            fin(seg("droite un", 320.0, 100.0)),
            fin(seg("droite deux", 320.0, 114.0)),
        ]);
        assert!(flottantes(&lignes, 842.0).is_empty());
    }

    #[test]
    fn un_schema_ecrit_en_fin_de_page_est_flottant() {
        let lignes = construire(&[
            fin(seg("paragraphe du haut", 72.0, 100.0)),
            fin(seg("paragraphe du bas", 72.0, 500.0)),
            fin(seg("légende du schéma", 80.0, 300.0)),
        ]);
        assert_eq!(flottantes(&lignes, 842.0), vec![2]);
    }

    #[test]
    fn un_pied_en_tete_du_flux_ne_rend_rien_flottant() {
        let lignes = construire(&[
            fin(seg("© GUIDE DES NÉGOCIATIONS", 72.0, 800.0)),
            fin(seg("le corps commence", 72.0, 100.0)),
        ]);
        assert!(flottantes(&lignes, 842.0).is_empty());
    }
}
