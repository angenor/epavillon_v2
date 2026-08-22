//! **La liste blanche du courriel** — et le piège du lien qui porte une variable.
//!
//! # Pourquoi elle diffère de celle de B4
//!
//! L'éditeur de présentation d'un dossier refuse toute mise en forme : la
//! charte décide de l'apparence, pas le déposant. Un gabarit de courriel n'a pas
//! ce luxe — **les clients de messagerie ignorent les feuilles de style**, si
//! bien qu'une mise en page de courriel passe par des tableaux et des attributs
//! `style` en ligne. Reprendre la liste de B4 rendrait tout gabarit illisible.
//!
//! # LE PIÈGE, et il est réel
//!
//! Un gabarit contient `<a href="{{lien_participation}}">`. Pour un analyseur
//! d'URL, `{{lien_participation}}` est une adresse **relative** — et la
//! politique par défaut d'un assainisseur est souvent de la réécrire contre une
//! base, ou de la refuser. Dans les deux cas **la variable est détruite**, et le
//! lien du courriel est mort.
//!
//! Ce défaut ne se voit **qu'à la réception**, c'est-à-dire après l'envoi à tous
//! les destinataires. La politique est donc réglée sur le **laisser-passer**, et
//! un test le prouve — sans lui, la décision resterait une intention.
//!
//! # Ce qui reste hors de portée, et qui est dit
//!
//! Un administrateur détenant `engagement.template.manage` peut écrire du CSS
//! malveillant dans un `style`. L'assainissement vise le HTML **collé** depuis
//! ailleurs, pas un compte de confiance. Le dire vaut mieux que de laisser
//! croire à une garantie plus forte.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Les balises admises. Bien plus larges que celles de B4 : un courriel se met
/// en page avec des tableaux, faute de feuille de style.
const BALISES: &[&str] = &[
    "p",
    "br",
    "strong",
    "b",
    "em",
    "i",
    "ul",
    "ol",
    "li",
    "blockquote",
    "h1",
    "h2",
    "h3",
    "h4",
    "hr",
    "a",
    "img",
    "table",
    "thead",
    "tbody",
    "tr",
    "td",
    "th",
    "div",
    "span",
];

/// `style` sur les balises de mise en page — la seule façon de mettre un
/// courriel en forme. Il n'est PAS admis sur `a` ni sur `img` : un lien peint
/// aux couleurs d'une page de connexion est un hameçonnage tout fait.
const BALISES_STYLEES: &[&str] = &[
    "p", "div", "span", "table", "thead", "tbody", "tr", "td", "th", "h1", "h2", "h3", "h4",
];

/// **Deux schémas, et `mailto:` en est.** Contrairement à B4 : un pied de
/// courriel porte légitimement l'adresse de contact de la plateforme, et la
/// rendre cliquable est l'usage.
const SCHEMAS: &[&str] = &["http", "https", "mailto"];

fn nettoyeur() -> &'static ammonia::Builder<'static> {
    static NETTOYEUR: OnceLock<ammonia::Builder<'static>> = OnceLock::new();
    NETTOYEUR.get_or_init(|| {
        let mut builder = ammonia::Builder::empty();

        let mut attributs: HashMap<&str, HashSet<&str>> = HashMap::new();
        attributs.insert("a", HashSet::from_iter(["href", "title"]));
        attributs.insert("img", HashSet::from_iter(["src", "alt", "width", "height"]));
        attributs.insert(
            "table",
            HashSet::from_iter(["width", "cellpadding", "cellspacing", "border", "role"]),
        );
        attributs.insert(
            "td",
            HashSet::from_iter(["width", "align", "valign", "colspan"]),
        );
        attributs.insert(
            "th",
            HashSet::from_iter(["width", "align", "valign", "colspan"]),
        );
        for balise in BALISES_STYLEES {
            attributs.entry(balise).or_default().insert("style");
        }

        builder
            .tags(HashSet::from_iter(BALISES.iter().copied()))
            .tag_attributes(attributs)
            .url_schemes(HashSet::from_iter(SCHEMAS.iter().copied()))
            // **LA LIGNE QUI COMPTE.** Sans elle, `href="{{lien}}"` — une URL
            // relative aux yeux de l'analyseur — serait réécrite ou refusée, et
            // le lien du courriel serait mort à la réception.
            .url_relative(ammonia::UrlRelative::PassThrough)
            // Un courriel s'ouvre dans un client, pas dans un onglet : le
            // couple `target`/`rel` de B4 n'a pas de sens ici, et `rel` posé
            // d'office ferait diverger le HTML enregistré de celui qu'on relit.
            .link_rel(None);
        builder
    });
    NETTOYEUR.get().expect("nettoyeur initialisé")
}

/// Assainit le corps d'une révision de modèle. Appelé **à l'écriture** : un
/// contenu stocké propre se rend partout, un contenu filtré à l'affichage doit
/// l'être dans chaque écran, chaque courriel et chaque export — et le premier
/// oubli est une injection.
pub fn assainir(html: &str) -> String {
    nettoyeur().clean(html).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le test dont R26 dit qu'il fait la différence entre une décision et une
    /// intention.**
    #[test]
    fn un_href_porteur_dune_variable_survit() {
        let propre = assainir(r#"<a href="{{lien_participation}}">Rejoindre</a>"#);
        assert!(
            propre.contains("{{lien_participation}}"),
            "la variable a été détruite : {propre}"
        );
    }

    #[test]
    fn le_script_disparait() {
        let propre = assainir(r#"<p>Bonjour</p><script>alert(1)</script>"#);
        assert!(!propre.contains("script"));
        assert!(propre.contains("Bonjour"));
    }

    #[test]
    fn lattribut_devenement_disparait() {
        let propre = assainir(r#"<p onclick="voler()">Texte</p>"#);
        assert!(!propre.contains("onclick"));
        assert!(propre.contains("Texte"));
    }

    #[test]
    fn le_schema_javascript_disparait() {
        let propre = assainir(r#"<a href="javascript:alert(1)">Piège</a>"#);
        assert!(!propre.contains("javascript"));
    }

    /// La différence assumée avec B4 : un courriel se met en page.
    #[test]
    fn le_tableau_et_le_style_en_ligne_survivent() {
        let propre = assainir(
            r#"<table width="600"><tr><td style="padding:16px">Contenu</td></tr></table>"#,
        );
        assert!(propre.contains("<table"));
        assert!(propre.contains("style=\"padding:16px\""));
        assert!(propre.contains("width=\"600\""));
    }

    /// Un lien peint aux couleurs d'une page de connexion est un hameçonnage.
    #[test]
    fn le_style_nest_pas_admis_sur_un_lien() {
        let propre = assainir(r#"<a href="https://x.fr" style="color:red">Lien</a>"#);
        assert!(!propre.contains("style"));
        assert!(propre.contains("https://x.fr"));
    }

    #[test]
    fn ladresse_de_contact_reste_cliquable() {
        let propre = assainir(r#"<a href="mailto:contact@ifdd.francophonie.org">Écrire</a>"#);
        assert!(propre.contains("mailto:"));
    }

    /// Un gabarit entier, tel qu'un administrateur le collerait.
    #[test]
    fn un_gabarit_complet_traverse_sans_perdre_ses_variables() {
        let gabarit = r#"<table><tr><td style="padding:8px">
            <p>Bonjour {{prenom}},</p>
            <p><a href="{{lien_participation}}">Rejoindre « {{titre_session}} »</a></p>
            <script>voler()</script>
        </td></tr></table>"#;
        let propre = assainir(gabarit);
        for variable in ["{{prenom}}", "{{lien_participation}}", "{{titre_session}}"] {
            assert!(propre.contains(variable), "{variable} perdue : {propre}");
        }
        assert!(!propre.contains("script"));
    }
}
