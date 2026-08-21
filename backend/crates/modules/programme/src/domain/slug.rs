//! L'adresse d'URL d'un dossier — dérivation, repli, suffixe (R5).
//!
//! # Pourquoi cette règle existe
//!
//! `programme.proposals.slug` est **obligatoire** et unique par édition
//! (`ux_proposals_slug`), et le contrat du formulaire ne la porte pas : le
//! client ne peut pas la calculer, il ignore les autres dossiers de l'édition
//! (écart n° 95). Sans dérivation, **le tout premier enregistrement échoue**.
//!
//! # Trois décisions, et chacune vient d'un fait
//!
//! **La normalisation se fait en base**, par `platform.slugify()` : c'est la
//! même fonction que le reste de la plateforme emploie, elle enlève les accents
//! et normalise selon les règles de PostgreSQL. La réécrire ici produirait deux
//! normalisations divergentes du même texte. Ce fichier ne porte donc **que ce
//! que la base ne fait pas** : le repli et le suffixe.
//!
//! **Le repli n'est pas un ornement.** Le premier enregistrement automatique a
//! lieu à la première frappe, quand le titre est encore vide — `slugify('')`
//! rend alors NULL, et la colonne est `NOT NULL`.
//!
//! **Le suffixe se pose sur collision, jamais par comptage préalable.** Compter
//! les homonymes avant d'insérer laisserait la course entre deux dépôts
//! simultanés faire échouer le second de toute façon. C'est le patron que le
//! noyau emploie déjà pour les empreintes de jeton.

/// Ce que porte un dossier dont le titre est encore vide. `platform.slug` exige
/// deux caractères au moins : une chaîne vide ne passerait pas le domaine.
pub const REPLI: &str = "dossier";

/// Longueur maximale de `platform.slug`.
const LONGUEUR_MAX: usize = 160;

/// Nombre de réessais sur collision. Dix homonymes dans une même édition est
/// déjà une quantité qu'aucun jeu de données réel n'atteint ; au-delà, insister
/// tiendrait la transaction ouverte sans rien régler.
pub const TENTATIVES_MAX: u8 = 10;

/// La base d'adresse, une fois `platform.slugify()` passée : sa valeur, ou le
/// repli quand elle est nulle ou vide.
///
/// La troncature laisse la place au suffixe : sans elle, un titre de cent
/// soixante signes rendrait un `slug-2` de cent soixante-deux, refusé par le
/// domaine à la deuxième tentative seulement — c'est-à-dire à la première
/// collision, longtemps après la mise en service.
pub fn base(slugifie: Option<&str>) -> String {
    let brut = slugifie.map(str::trim).filter(|s| !s.is_empty());
    match brut {
        None => REPLI.to_owned(),
        Some(valeur) => tronquer(valeur, LONGUEUR_MAX - MARGE_SUFFIXE),
    }
}

/// De quoi loger `-10`, le plus long suffixe que [`TENTATIVES_MAX`] produise.
const MARGE_SUFFIXE: usize = 3;

/// L'adresse de la n-ième tentative. La première ne porte aucun suffixe : un
/// dossier sans homonyme garde l'adresse de son titre.
pub fn tentative(base: &str, numero: u8) -> String {
    if numero == 0 {
        base.to_owned()
    } else {
        format!("{base}-{}", numero + 1)
    }
}

/// Tronque **sur une frontière de segment** quand c'est possible : couper au
/// caractère produirait `-` en fin de chaîne ou un mot amputé, l'un refusé par
/// le domaine, l'autre illisible.
fn tronquer(valeur: &str, max: usize) -> String {
    if valeur.len() <= max {
        return valeur.to_owned();
    }
    let coupe = &valeur[..max];
    match coupe.rfind('-') {
        Some(0) | None => coupe.trim_end_matches('-').to_owned(),
        Some(position) => coupe[..position].to_owned(),
    }
}
