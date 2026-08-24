//! Export du document OpenAPI — `cargo run -p api --bin openapi`.
//!
//! Écrit le document sur la sortie standard, **sans base ni serveur**. La route
//! `/api/docs` sert le même document, mais elle ne décrit que les modules
//! réellement montés et exige une application vivante : la génération du client
//! TypeScript du site ne peut pas en dépendre — elle tournerait sur l'état
//! d'une base de développement, et un module éteint ce jour-là retirerait ses
//! chemins du client.

use api::modules::ModuleRegistry;
use std::collections::HashMap;

fn main() {
    let document = api::openapi::document(&ModuleRegistry::complet());

    if let Err(collision) = identifiants_uniques(&document) {
        eprintln!("Document refusé : {collision}");
        std::process::exit(1);
    }

    match document.to_pretty_json() {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("document illisible : {e}");
            std::process::exit(1);
        }
    }
}

/// **Deux opérations ne peuvent pas porter le même identifiant.**
///
/// OpenAPI l'exige, et le générateur TypeScript en fait un nom de type : deux
/// `operation_id` égaux produisent un fichier qui ne compile pas, avec un
/// message qui ne nomme ni la route ni le module. C'est arrivé — `fiche`, porté
/// à la fois par la fiche d'une personne et par celle d'un utilisateur du
/// back-office —, et rien ne le signalait avant le `npm run typecheck` du site.
fn identifiants_uniques(document: &utoipa::openapi::OpenApi) -> Result<(), String> {
    let mut vus: HashMap<&str, String> = HashMap::new();

    for (chemin, item) in document.paths.paths.iter() {
        let operations = [
            ("GET", &item.get),
            ("PUT", &item.put),
            ("POST", &item.post),
            ("DELETE", &item.delete),
            ("PATCH", &item.patch),
        ];
        for (methode, operation) in operations {
            let Some(id) = operation.as_ref().and_then(|o| o.operation_id.as_deref()) else {
                continue;
            };
            let ou = format!("{methode} {chemin}");
            if let Some(premier) = vus.insert(id, ou.clone()) {
                return Err(format!(
                    "l'identifiant d'opération « {id} » est porté deux fois — {premier} et {ou}. \
                     Préfixez-le par son domaine, comme le reste du back-office."
                ));
            }
        }
    }
    Ok(())
}
