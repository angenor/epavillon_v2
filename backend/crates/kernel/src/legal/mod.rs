//! Les textes qui engagent — politique de confidentialité et conditions
//! d'utilisation —, **embarqués dans le binaire**, une source pour le site et
//! pour l'application.
//!
//! Ils vivent dans `kernel` et non dans un module : `programme` lit la version à
//! l'inscription, et ne peut dépendre d'aucun autre crate de module (principe
//! II). La version servie est celle que `identity.consents` enregistre ; elle
//! remplace le réglage `PRIVACY_POLICY_VERSION`, qui pouvait nommer une version
//! dont personne ne produisait le texte.
//!
//! **Tant que l'IFDD n'a pas fourni un texte, il est « en attente »** : aucun
//! corps, et la version reste `2026-01`, celle que l'inscription enregistrait
//! déjà. Rien ici n'est écrit par un outil — voir `LISEZMOI.md`.

use std::sync::OnceLock;

use serde::Serialize;

/// Les deux textes. Les licences n'en sont pas : elles n'appellent aucun accord
/// et vivent avec l'écran « À propos ».
pub const CLES: [&str; 2] = ["privacy", "terms"];
pub const LANGUES: [&str; 2] = ["fr", "en"];
const LANGUE_DE_REPLI: &str = "fr";

/// `(fichier, contenu)`, dans l'ordre de `CLES` × `LANGUES`.
pub const FICHIERS: [(&str, &str); 4] = [
    ("privacy.fr.md", include_str!("privacy.fr.md")),
    ("privacy.en.md", include_str!("privacy.en.md")),
    ("terms.fr.md", include_str!("terms.fr.md")),
    ("terms.en.md", include_str!("terms.en.md")),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Etat {
    /// Le texte de l'IFDD n'est pas encore fourni : aucun corps.
    Pending,
    Published,
}

/// `LegalText` — ce que `GET /legal/{cle}` rend.
#[derive(Debug, Clone, Serialize)]
pub struct Texte {
    pub key: &'static str,
    /// La langue **servie**, qui peut différer de la demandée : repli sur le français.
    pub locale: &'static str,
    pub status: Etat,
    pub version: String,
    /// Date d'entrée en vigueur, `AAAA-MM-JJ` ; nulle tant que le texte est en attente.
    pub effective_date: Option<String>,
    /// Markdown, dans la grammaire close ; nul tant que le texte est en attente.
    pub body: Option<String>,
}

/// Lit un fichier. L'erreur nomme le fichier et ce qui manque : ces textes
/// seront écrits par quelqu'un qui n'a aucune raison de connaître le format.
pub fn lire(fichier: &'static str, contenu: &str) -> Result<Texte, String> {
    let (cle, langue) = fichier
        .strip_suffix(".md")
        .and_then(|nom| nom.split_once('.'))
        .ok_or_else(|| format!("{fichier} : nom attendu « cle.langue.md »"))?;
    let key = CLES
        .into_iter()
        .find(|c| *c == cle)
        .ok_or_else(|| format!("{fichier} : clé inconnue « {cle} »"))?;
    let locale = LANGUES
        .into_iter()
        .find(|l| *l == langue)
        .ok_or_else(|| format!("{fichier} : langue inconnue « {langue} »"))?;

    let reste = contenu
        .strip_prefix("---\n")
        .ok_or_else(|| format!("{fichier} : l'en-tête doit ouvrir le fichier par « --- »"))?;
    let (entete, corps) = reste
        .split_once("\n---\n")
        .or_else(|| reste.strip_suffix("\n---").map(|e| (e, "")))
        .ok_or_else(|| format!("{fichier} : l'en-tête doit se fermer par « --- »"))?;

    let champ = |nom: &str| {
        entete
            .lines()
            .find_map(|l| l.strip_prefix(nom)?.strip_prefix(':'))
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    };

    let status = match champ("etat").as_deref() {
        Some("en_attente") => Etat::Pending,
        Some("publie") => Etat::Published,
        autre => {
            return Err(format!(
                "{fichier} : « etat » vaut en_attente ou publie, pas {autre:?}"
            ))
        }
    };
    let version = champ("version").ok_or_else(|| format!("{fichier} : « version » manque"))?;
    let effective_date = champ("en_vigueur");
    let corps = corps.trim();

    match status {
        Etat::Pending if !corps.is_empty() => {
            return Err(format!("{fichier} : un texte en attente n'a pas de corps"));
        }
        Etat::Published if corps.is_empty() => {
            return Err(format!("{fichier} : un texte publié a un corps"));
        }
        Etat::Published if effective_date.is_none() => {
            return Err(format!(
                "{fichier} : un texte publié porte sa date « en_vigueur »"
            ));
        }
        _ => {}
    }

    Ok(Texte {
        key,
        locale,
        status,
        version,
        effective_date,
        body: (!corps.is_empty()).then(|| corps.to_owned()),
    })
}

fn textes() -> &'static [Texte] {
    static TEXTES: OnceLock<Vec<Texte>> = OnceLock::new();
    TEXTES.get_or_init(|| {
        FICHIERS
            .iter()
            .map(|(fichier, contenu)| lire(fichier, contenu).unwrap_or_else(|e| panic!("{e}")))
            .collect()
    })
}

/// Le texte dans la langue demandée, **en français à défaut**. `None` : clé inconnue.
pub fn texte(cle: &str, langue: &str) -> Option<&'static Texte> {
    let de = |l: &str| textes().iter().find(|t| t.key == cle && t.locale == l);
    de(langue).or_else(|| de(LANGUE_DE_REPLI))
}

/// La version qu'un accord enregistre. Une par texte, pas par langue — un test
/// le tient.
pub fn version(cle: &str) -> &'static str {
    texte(cle, LANGUE_DE_REPLI)
        .map(|t| t.version.as_str())
        .unwrap_or_else(|| panic!("texte inconnu : {cle}"))
}

/// Les constructions que le rendu de l'application ne connaît pas, avec leur
/// ligne (numérotée depuis 1, dans le corps). **Vide, c'est rendable.**
///
/// La grammaire est close : titres `##` et `###`, paragraphes, listes `- ` et
/// `1. `, liens `[texte](adresse)`, emphase. Tout le reste s'afficherait en
/// charabia, sans prévenir personne ; le test échoue à la place.
pub fn constructions_inconnues(corps: &str) -> Vec<(usize, &'static str)> {
    let mut trouvees = Vec::new();
    for (index, ligne) in corps.lines().enumerate() {
        let numero = index + 1;
        let nette = ligne.trim();
        let construction = if ligne.starts_with("    ") || ligne.starts_with('\t') {
            Some("bloc de code indenté")
        } else if nette.starts_with("```") || nette.starts_with("~~~") {
            Some("bloc de code")
        } else if nette.starts_with('#') && !nette.starts_with("## ") && !nette.starts_with("### ")
        {
            Some("titre d'un niveau autre que 2 ou 3")
        } else if nette.starts_with('>') {
            Some("citation")
        } else if nette.starts_with('|') || (nette.contains('|') && nette.contains("---")) {
            Some("tableau")
        } else if nette.contains("![") {
            Some("image")
        } else if nette.contains("[^") {
            Some("note de bas de page")
        } else if ["---", "***", "___"].contains(&nette) {
            Some("filet")
        } else if nette.starts_with("* ") || nette.starts_with("+ ") {
            Some("puce autre que « - »")
        } else if nette.starts_with('[') && nette.contains("]:") {
            Some("lien par référence")
        } else if balise_html(nette) {
            Some("HTML")
        } else {
            None
        };
        if let Some(construction) = construction {
            trouvees.push((numero, construction));
        }
    }
    trouvees
}

fn balise_html(ligne: &str) -> bool {
    ligne.match_indices('<').any(|(i, _)| {
        ligne[i + 1..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '/' || c == '!')
    })
}
