//! **Un texte modifié sans nouvelle version fait échouer ce test.**
//!
//! Une preuve de consentement nomme une version ; si le texte bouge sous cette
//! version, la preuve n'oppose plus rien. Chaque fichier a donc ici l'empreinte
//! attendue pour sa version déclarée. Modifier un texte oblige à lever sa
//! version, puis à inscrire ici la nouvelle empreinte — c'est voulu.

use kernel::legal::{lire, FICHIERS};

/// `(fichier, version, empreinte)` — 16 premiers octets du SHA-256 du fichier.
const ATTENDUES: &[(&str, &str, &str)] = &[
    (
        "privacy.fr.md",
        "2026-01",
        "ef5adf2372a6d40b89a8a526457fd0e4",
    ),
    (
        "privacy.en.md",
        "2026-01",
        "ef5adf2372a6d40b89a8a526457fd0e4",
    ),
    ("terms.fr.md", "2026-01", "ef5adf2372a6d40b89a8a526457fd0e4"),
    ("terms.en.md", "2026-01", "ef5adf2372a6d40b89a8a526457fd0e4"),
];

fn empreinte(contenu: &str) -> String {
    kernel::crypto::token_hash(contenu)[..16]
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect()
}

#[test]
fn chaque_texte_garde_son_empreinte_pour_sa_version() {
    for (fichier, contenu) in FICHIERS {
        let texte = lire(fichier, contenu).unwrap_or_else(|e| panic!("{e}"));
        let attendue = ATTENDUES
            .iter()
            .find(|(f, v, _)| *f == fichier && *v == texte.version)
            .unwrap_or_else(|| {
                panic!(
                    "{fichier} : aucune empreinte inscrite pour la version {} — inscrire \
                     (\"{fichier}\", \"{}\", \"{}\") dans ATTENDUES",
                    texte.version,
                    texte.version,
                    empreinte(contenu)
                )
            });
        assert_eq!(
            empreinte(contenu),
            attendue.2,
            "{fichier} a changé sans que sa version {} change : lever la version, puis inscrire \
             la nouvelle empreinte",
            texte.version
        );
    }
}

/// Le contrôle mord : un mot changé sous la même version change l'empreinte.
#[test]
fn un_mot_change_change_lempreinte() {
    let (_, contenu) = FICHIERS[0];
    assert_ne!(
        empreinte(contenu),
        empreinte(&contenu.replace("2026-01", "2026-02"))
    );
}
