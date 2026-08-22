//! Un moteur **déclaré**, pas une absence.

use async_trait::async_trait;

use super::{Scanner, Verdict};

pub struct AucunMoteur;

#[async_trait]
impl Scanner for AucunMoteur {
    async fn analyser(&self, _contenu: &[u8]) -> Verdict {
        Verdict::non_pris_en_charge(
            "none",
            "aucun moteur d'analyse n'est configuré (MEDIA_SCANNER=none)",
        )
    }

    fn engine(&self) -> &'static str {
        "none"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le test qui garde la décision : « sain » sans avoir regardé rendrait
    /// fausse la preuve d'inspection.
    #[tokio::test]
    async fn labsence_de_moteur_ne_declare_jamais_un_fichier_sain() {
        let verdict = AucunMoteur.analyser(b"peu importe").await;
        assert_eq!(verdict.verdict, "unsupported");
        assert_eq!(verdict.engine, "none");
        assert!(verdict.autorise_la_mise_en_service());
    }
}
