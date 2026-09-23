//! L'étiquette d'une page : celle que le PDF déclare, sinon le numéro imprimé
//! dans son pied, sinon son indice.

use super::lignes::Ligne;

pub fn etiquette(declaree: Option<&str>, ecartees: &[Ligne], indice: usize) -> String {
    if let Some(e) = declaree.map(str::trim).filter(|e| !e.is_empty()) {
        return e.to_owned();
    }
    ecartees
        .iter()
        .map(|l| l.texte().trim().to_owned())
        .find(|t| !t.is_empty() && t.len() <= 4 && t.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or_else(|| indice.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::lignes::{construire, essai::*};
    use super::*;

    #[test]
    fn letiquette_declaree_puis_le_numero_imprime_puis_lindice() {
        let pied = construire(&[
            fin(seg("© GUIDE", 72.0, 790.0)),
            fin(seg("59", 510.0, 790.0)),
        ]);
        assert_eq!(etiquette(Some("iv"), &pied, 63), "iv");
        assert_eq!(etiquette(None, &pied, 63), "59");
        assert_eq!(etiquette(None, &[], 63), "63");
    }
}
