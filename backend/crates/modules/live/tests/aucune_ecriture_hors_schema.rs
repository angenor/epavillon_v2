//! **Le module n'écrit que dans son propre schéma** — `platform` et `reference`
//! compris.
//!
//! Un grep qui vaut un test : il parcourt `src/` et refuse toute écriture visant
//! un autre schéma. C'est plus strict que ce qui a été tenu en B3, B4 et B5, et
//! c'est ce qui rend la frontière du principe II vérifiable plutôt que promise.
//!
//! **Les appels de fonction ne comptent pas** : `live.publish_incident()` écrit,
//! mais c'est le modèle qui écrit — le module ne compose aucun `UPDATE`.

use std::fs;
use std::path::Path;

const VERBES: [&str; 3] = ["INSERT INTO ", "UPDATE ", "DELETE FROM "];

/// Le seul schéma dans lequel ce crate a le droit d'écrire.
const SIEN: &str = "live.";

#[test]
fn aucune_ecriture_ne_vise_un_autre_schema() {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut fautes = Vec::new();
    parcourir(&racine, &mut fautes);
    assert!(
        fautes.is_empty(),
        "écritures hors du schéma `live` :\n{}",
        fautes.join("\n")
    );
}

fn parcourir(dossier: &Path, fautes: &mut Vec<String>) {
    for entree in fs::read_dir(dossier).expect("lecture du dossier") {
        let chemin = entree.expect("entrée").path();
        if chemin.is_dir() {
            parcourir(&chemin, fautes);
        } else if chemin.extension().is_some_and(|e| e == "rs") {
            let contenu = fs::read_to_string(&chemin).expect("lecture du fichier");
            for (n, ligne) in contenu.lines().enumerate() {
                for verbe in VERBES {
                    if let Some(reste) = ligne.trim_start().strip_prefix(verbe) {
                        if !reste.starts_with(SIEN) {
                            fautes.push(format!(
                                "{}:{} — {}",
                                chemin.display(),
                                n + 1,
                                ligne.trim()
                            ));
                        }
                    }
                }
            }
        }
    }
}
