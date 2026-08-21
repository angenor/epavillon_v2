//! La liste blanche du HTML restreint (R14, écart n° 32).
//!
//! # Pourquoi une bibliothèque, et pas un filtre écrit ici
//!
//! Le filtrage de HTML écrit à la main est le cas d'école du contrôle qu'on
//! croit avoir. Un analyseur conforme à la spécification HTML est le seul moyen
//! de refuser ce qu'un **navigateur** accepterait : attribut d'événement,
//! `javascript:` encodé, balise mal fermée qui rouvre un contexte. `ammonia`
//! est fondée sur `html5ever`, l'analyseur de Servo.
//!
//! # Pourquoi à l'écriture
//!
//! Un contenu stocké propre se rend partout. Un contenu filtré à l'affichage
//! doit l'être dans chaque écran, chaque courriel et chaque export — et le
//! premier oubli est une injection. C'est l'écart n° 32, et le modèle l'écrit
//! lui-même : « assainie par l'API à l'écriture ».
//!
//! # La liste blanche est celle de la barre d'outils, exactement
//!
//! Elle est relevée sur `frontend/app/components/ui/RichText.vue` : gras,
//! italique, listes à puces et numérotées, citation, titres de niveau 3 et 4.
//! S'y ajoutent le paragraphe, le saut de ligne, le séparateur et le lien, que
//! l'éditeur produit sans bouton dédié.
//!
//! **Ni police, ni taille, ni couleur, ni alignement**, et ce n'est pas une
//! omission : la mise en forme appartient à la charte, pas au déposant. Un
//! texte peint en bleu foncé devient illisible au premier thème sombre.

use std::collections::HashSet;
use std::sync::OnceLock;

/// Les balises admises. `strong`/`b` et `em`/`i` cohabitent : l'éditeur produit
/// les premières, un dossier repris de la v1 porte les secondes.
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
    "h3",
    "h4",
    "hr",
    "a",
];

/// Le seul attribut admis, sur la seule balise qui en prenne un.
const ATTRIBUT_LIEN: &str = "href";

/// **Les deux seuls schémas d'URL.** `mailto:` en est absent volontairement :
/// une adresse écrite dans une présentation se lit, elle n'a pas à devenir
/// cliquable, et l'admettre ouvrirait la porte au reste.
const SCHEMAS: &[&str] = &["http", "https"];

fn nettoyeur() -> &'static ammonia::Builder<'static> {
    static NETTOYEUR: OnceLock<ammonia::Builder<'static>> = OnceLock::new();
    NETTOYEUR.get_or_init(|| {
        let mut builder = ammonia::Builder::empty();
        builder
            .tags(HashSet::from_iter(BALISES.iter().copied()))
            .tag_attributes(std::collections::HashMap::from([(
                "a",
                HashSet::from_iter([ATTRIBUT_LIEN]),
            )]))
            .url_schemes(HashSet::from_iter(SCHEMAS.iter().copied()))
            // Un lien d'un dossier mène hors de la plateforme : sans ces deux
            // valeurs, la page ouverte accède à `window.opener`.
            .link_rel(Some("noopener noreferrer"));
        builder
    })
}

/// Assainit un fragment. Ce qui n'est pas dans la liste blanche disparaît ; le
/// **texte** qu'une balise refusée contenait est conservé — supprimer le
/// contenu ferait perdre un paragraphe entier pour un `<span>` de trop.
pub fn assainir(html: &str) -> String {
    nettoyeur().clean(html).to_string()
}

/// Un fragment vide, tel que l'éditeur le rend une fois le champ effacé.
///
/// Il n'envoie pas toujours la chaîne vide mais `<p></p>` : sans cette
/// reconnaissance, tout champ « vide » compterait comme rempli à la validation,
/// et l'exigence de français d'`i18n_text` s'appliquerait à un paragraphe creux.
pub fn est_vide(html: &str) -> bool {
    texte_seul(html).trim().is_empty()
}

/// Le texte débarrassé de son balisage. Sert à mesurer et à décider du vide,
/// jamais à stocker : c'est le HTML assaini qui est écrit.
pub fn texte_seul(html: &str) -> String {
    let mut dans_balise = false;
    let mut texte = String::with_capacity(html.len());
    for c in html.chars() {
        match c {
            '<' => dans_balise = true,
            '>' => dans_balise = false,
            _ if !dans_balise => texte.push(c),
            _ => {}
        }
    }
    texte
}
