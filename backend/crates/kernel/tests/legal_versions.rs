//! Une version par texte, pas par langue — et, pour un texte publié, **sa date
//! d'entrée en vigueur est sa version**.

use kernel::legal::{self, lire, Etat, CLES, FICHIERS, LANGUES};

fn textes() -> Vec<legal::Texte> {
    FICHIERS
        .iter()
        .map(|(f, c)| lire(f, c).unwrap_or_else(|e| panic!("{e}")))
        .collect()
}

#[test]
fn les_deux_langues_dun_texte_declarent_la_meme_version() {
    let tous = textes();
    for cle in CLES {
        let versions: Vec<&str> = tous
            .iter()
            .filter(|t| t.key == cle)
            .map(|t| t.version.as_str())
            .collect();
        assert_eq!(versions.len(), LANGUES.len(), "{cle} : une langue manque");
        assert!(
            versions.windows(2).all(|p| p[0] == p[1]),
            "{cle} : deux langues, deux versions {versions:?} — ce serait deux textes"
        );
    }
}

#[test]
fn un_texte_publie_a_pour_version_sa_date_dentree_en_vigueur() {
    for texte in textes().iter().filter(|t| t.status == Etat::Published) {
        assert_eq!(
            texte.effective_date.as_deref(),
            Some(texte.version.as_str()),
            "{}.{} : la date d'entrée en vigueur devient la version",
            texte.key,
            texte.locale
        );
    }
}

/// Tant que l'IFDD n'a rien fourni, l'inscription enregistre ce qu'elle
/// enregistrait déjà : `2026-01`.
#[test]
fn un_texte_en_attente_garde_la_version_2026_01_et_na_pas_de_corps() {
    for texte in textes().iter().filter(|t| t.status == Etat::Pending) {
        assert_eq!(texte.version, "2026-01", "{}.{}", texte.key, texte.locale);
        assert!(texte.body.is_none());
        assert!(texte.effective_date.is_none());
    }
}

#[test]
fn une_langue_absente_retombe_sur_le_francais() {
    let texte = legal::texte("privacy", "es").expect("clé connue");
    assert_eq!(texte.locale, "fr");
    assert!(legal::texte("inconnu", "fr").is_none());
    assert_eq!(legal::version("privacy"), "2026-01");
}

#[test]
fn un_entete_fautif_nomme_le_fichier() {
    let erreur = lire(
        "privacy.fr.md",
        "---\netat: publie\nversion: 2026-11-01\n---\n",
    )
    .unwrap_err();
    assert!(erreur.starts_with("privacy.fr.md"), "{erreur}");
    let erreur = lire(
        "privacy.fr.md",
        "---\netat: en_attente\nversion: 2026-01\n---\nUn corps.\n",
    )
    .unwrap_err();
    assert!(erreur.contains("pas de corps"), "{erreur}");
}
