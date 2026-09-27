//! Lire un fichier de routes comme un texte : ce qu'il monte, quelles gardes
//! ses gestionnaires déclarent. Partagé par les tests de périmètre.

/// Le code seul, commentaires retirés.
///
/// Les fichiers de ce module **expliquent le piège dans leur en-tête**, et y
/// nomment donc `RequiresAnyScope` et `Perimeter` pour dire de ne pas s'en
/// servir. Compter sur le texte brut ferait échouer le contrôle sur
/// l'explication qui le justifie.
pub fn sans_commentaires(source: &str) -> String {
    source
        .lines()
        .filter(|ligne| !ligne.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Le code sans commentaires ni blancs : `rustfmt` coupe les appels longs sur
/// plusieurs lignes.
pub fn compact(source: &str) -> String {
    sans_commentaires(source)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Les `.route("chemin", web::verbe().to(gestionnaire))` du fichier, en
/// `(verbe, chemin, gestionnaire)`.
pub fn routes_montees(source: &str) -> Vec<(String, String, String)> {
    compact(source)
        .split(".route(")
        .skip(1)
        .map(|appel| {
            let (chemin, reste) = appel
                .strip_prefix('"')
                .and_then(|a| a.split_once("\",web::"))
                .unwrap_or_else(|| panic!("route illisible : {appel:.80}"));
            let (verbe, reste) = reste
                .split_once("().to(")
                .unwrap_or_else(|| panic!("verbe illisible : {reste:.80}"));
            let gestionnaire = reste.split(')').next().unwrap_or_default();
            (verbe.to_owned(), chemin.to_owned(), gestionnaire.to_owned())
        })
        .collect()
}

/// Chaque `pub(crate) async fn`, avec sa signature — jusqu'à l'accolade du
/// corps, là où les extracteurs se déclarent.
pub fn signatures(source: &str) -> Vec<(String, String)> {
    sans_commentaires(source)
        .split("pub(crate) async fn ")
        .skip(1)
        .map(|reste| {
            let nom = reste.split('(').next().unwrap_or_default().trim();
            let signature = reste.split('{').next().unwrap_or_default();
            (nom.to_owned(), signature.to_owned())
        })
        .collect()
}

/// Le corps compacté du bloc qui suit `entete`. Les accolades des chaînes ne
/// comptent pas : les chemins en portent, `{id}`.
pub fn bloc(source: &str, entete: &str) -> String {
    let code = compact(source);
    let debut = code
        .find(entete)
        .unwrap_or_else(|| panic!("{entete} absent du fichier"))
        + entete.len();
    let reste = &code[debut..];
    let ouverture = reste.find('{').expect("accolade ouvrante");
    let (mut profondeur, mut dans_une_chaine, mut precedent) = (0, false, ' ');
    for (i, c) in reste[ouverture..].char_indices() {
        match c {
            '"' if precedent != '\\' => dans_une_chaine = !dans_une_chaine,
            '{' if !dans_une_chaine => profondeur += 1,
            '}' if !dans_une_chaine => {
                profondeur -= 1;
                if profondeur == 0 {
                    return reste[ouverture + 1..ouverture + i].to_owned();
                }
            }
            _ => {}
        }
        precedent = c;
    }
    panic!("le bloc de {entete} ne se ferme pas")
}

pub const CONFIGURER: &str = "pubfnconfigurer(cfg:&mutweb::ServiceConfig)";
/// Ce qui monte une porte sans passer par `.route(…)`.
const AUTRES_MONTAGES: [&str; 7] = [
    ".service(",
    "web::resource(",
    "web::scope(",
    "default_service(",
    ".configure(",
    "web::to(",
    "external_resource(",
];
const MACROS_DE_ROUTE: [&str; 11] = [
    "get", "post", "put", "patch", "delete", "head", "options", "trace", "connect", "route",
    "routes",
];

/// Tout ce qui, dans un fichier de routes, échapperait au compte des
/// `.route(…)`. Vide : le fichier ne monte que ses routes.
pub fn ecarts_de_montage(source: &str) -> Vec<String> {
    let code = compact(source);
    let mut ecarts: Vec<String> = AUTRES_MONTAGES
        .iter()
        .filter(|m| code.contains(**m))
        .map(|m| (*m).to_owned())
        .collect();
    for attribut in code.split("#[").skip(1) {
        let nom = attribut.split(['(', ']']).next().unwrap_or_default();
        if MACROS_DE_ROUTE.contains(&nom.trim_start_matches("actix_web::")) {
            ecarts.push(format!("#[{nom}]"));
        }
    }
    let points_de_montage = code.matches("ServiceConfig").count();
    if points_de_montage != 1 {
        ecarts.push(format!("{points_de_montage} ServiceConfig"));
    }
    let routes: String = routes_montees(source)
        .iter()
        .map(|(v, c, g)| format!(".route(\"{c}\",web::{v}().to({g}))"))
        .collect();
    // `rustfmt` laisse une virgule finale aux appels qu'il coupe.
    let configurer = bloc(source, CONFIGURER).replace(",)", ")");
    if configurer != format!("cfg{routes};") {
        ecarts.push(format!("configurer() : {configurer}"));
    }
    ecarts
}
