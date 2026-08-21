//! Les huit longueurs maximales des textes d'un dossier (R15, écart n° 28).
//!
//! # Pourquoi elles sont ici et pas dans la configuration
//!
//! Le contraste avec B1 est instructif. Les seuils de verrouillage de compte
//! sont allés en configuration parce que ce sont des réglages
//! **d'exploitation** : on les ajuste sans redéployer. Une longueur maximale de
//! résumé est une règle de **contrat** — la changer change ce que le front
//! affiche, ce que la carte de programmation rend, et ce que l'export produit.
//! La rendre modifiable par variable d'environnement, ce serait permettre à
//! deux déploiements de refuser des dossiers différents.
//!
//! # Pourquoi elles ne sont pas en base
//!
//! `platform.i18n_text` est un `jsonb` sans borne, et c'est justifié : la base
//! n'a pas à trancher ce qu'est un résumé lisible.
//!
//! **Les valeurs sont exactement celles de `TEXT_LIMITS`** du front
//! (`frontend/app/types/proposal-form.ts`). Deux bornes différentes des deux
//! côtés produiraient un écran qui accepte et une API qui refuse — le pire
//! ordre des deux.

/// Un champ borné, avec le nom que l'erreur rendra à l'écran.
pub struct Borne {
    pub champ: &'static str,
    pub max: usize,
}

const fn borne(champ: &'static str, max: usize) -> Borne {
    Borne { champ, max }
}

pub const TITRE: Borne = borne("title", 180);
pub const RESUME: Borne = borne("summary", 400);
pub const OBJECTIFS: Borne = borne("objectives", 1200);
pub const PRESENTATION: Borne = borne("detailed_presentation", 4000);
pub const RESULTATS: Borne = borne("expected_outcomes", 1200);
pub const PUBLIC_VISE: Borne = borne("target_audiences", 600);
pub const CONTRAINTES: Borne = borne("scheduling_constraints", 500);
pub const BIOGRAPHIE: Borne = borne("bio", 800);

/// Le texte tient-il dans sa borne ?
///
/// **Le compte est en caractères, pas en octets** : un résumé de quatre cents
/// signes accentués fait plus de quatre cents octets, et le refuser pour cela
/// serait refuser le français.
pub fn tient(texte: &str, borne: &Borne) -> bool {
    texte.chars().count() <= borne.max
}

/// La borne de la présentation détaillée se mesure sur le **texte**, pas sur le
/// HTML : le balisage de l'éditeur compte pour un tiers du volume, et le
/// compteur du front — qui affiche `getText().length` — dirait autre chose que
/// l'API. Deux compteurs divergents sur le même champ, c'est un envoi refusé
/// sans que l'écran l'ait annoncé.
pub fn longueur_du_texte(html: &str) -> usize {
    super::sanitize::texte_seul(html).chars().count()
}
