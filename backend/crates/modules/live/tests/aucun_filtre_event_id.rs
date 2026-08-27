//! **Deux greps qui valent des tests.**
//!
//! 1. Aucun fichier de `src/` ne compare `incidents.event_id` à un paramètre
//!    d'édition. `live.incidents` n'a aucune colonne d'édition pour trois portées
//!    sur cinq : un tel filtre laisserait fuir la moitié de l'écran, sans que rien
//!    ne le dise — la liste paraîtrait simplement plus courte.
//!
//! 2. Aucun fichier n'appelle `kernel::events::emit`. Les deux fonctions de
//!    publication émettent déjà ; un second appel doublerait chaque ligne
//!    d'outbox.

use std::fs;
use std::path::Path;

#[test]
fn aucun_filtre_ecrit_a_la_main_sur_lédition_dun_incident() {
    for (chemin, contenu) in fichiers_source() {
        for (n, ligne) in contenu.lines().enumerate() {
            let sans_espaces: String = ligne.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                !sans_espaces.contains("i.event_id = $")
                    && !sans_espaces.contains("incidents.event_id = $"),
                "{}:{} filtre l'édition à la main — passer par live.event_incidents() : {}",
                chemin,
                n + 1,
                ligne.trim()
            );
        }
    }
}

#[test]
fn aucun_appel_a_emit() {
    for (chemin, contenu) in fichiers_source() {
        for (n, ligne) in contenu.lines().enumerate() {
            // **Les commentaires sont écartés, et c'est nécessaire** : l'en-tête
            // du crate NOMME l'appel interdit pour dire qu'on ne le fait pas.
            // Un contrôle qui les compterait interdirait d'expliquer la règle.
            let code = ligne.trim_start();
            if code.starts_with("//") {
                continue;
            }
            assert!(
                !code.contains("events::emit") && !code.contains("emit_event("),
                "{}:{} émet un événement : les deux fonctions du modèle le font déjà",
                chemin,
                n + 1
            );
        }
    }
}

fn fichiers_source() -> Vec<(String, String)> {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut fichiers = Vec::new();
    collecter(&racine, &mut fichiers);
    fichiers
}

fn collecter(dossier: &Path, fichiers: &mut Vec<(String, String)>) {
    for entree in fs::read_dir(dossier).expect("lecture du dossier") {
        let chemin = entree.expect("entrée").path();
        if chemin.is_dir() {
            collecter(&chemin, fichiers);
        } else if chemin.extension().is_some_and(|e| e == "rs") {
            let contenu = fs::read_to_string(&chemin).expect("lecture du fichier");
            fichiers.push((chemin.display().to_string(), contenu));
        }
    }
}
