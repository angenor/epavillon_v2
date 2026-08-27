//! **Aucune arête vers un autre crate de module** — principe II.
//!
//! Le contrôle porte sur `cargo tree`, qui liste **aussi les dépendances de
//! développement** : c'est pour cela que les tests de ce crate ne montent pas la
//! vraie application, qui demanderait une arête vers `api`.

use std::process::Command;

#[test]
fn le_manifeste_ne_cite_aucun_crate_de_module() {
    let manifeste = include_str!("../Cargo.toml");
    for module in [
        "identity",
        "org",
        "event",
        "programme",
        "media",
        "engagement",
        "content",
        "analytics",
    ] {
        assert!(
            !manifeste.contains(&format!("{module}.workspace")),
            "le crate `live` ne dépend pas de `{module}`"
        );
    }
}

#[test]
fn cargo_tree_ne_porte_aucune_arete_vers_un_module() {
    let sortie = Command::new(env!("CARGO"))
        .args(["tree", "-p", "live", "--edges", "normal,build,dev"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree");

    let arbre = String::from_utf8_lossy(&sortie.stdout);
    // **Le nom se lit APRÈS le tiret de branche**, jamais par sous-chaîne : sans
    // cela, « api » se retrouve dans « lock_api » et le contrôle échoue sur une
    // dépendance qui n'a rien à voir.
    for ligne in arbre.lines() {
        let nom = ligne
            .rsplit_once("── ")
            .map(|(_, reste)| reste)
            .unwrap_or(ligne)
            .split_whitespace()
            .next()
            .unwrap_or_default();

        assert!(
            !matches!(
                nom,
                "identity"
                    | "org"
                    | "event"
                    | "programme"
                    | "media"
                    | "engagement"
                    | "content"
                    | "analytics"
                    | "api"
            ),
            "arête interdite vers `{nom}` :\n{ligne}"
        );
    }
}
