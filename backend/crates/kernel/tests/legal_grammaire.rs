//! **Les fichiers réels se rendent avec la grammaire close**, et toute
//! construction qu'elle ne couvre pas fait échouer ce test, avec sa ligne.
//!
//! Ces textes seront écrits par quelqu'un qui n'a aucune raison de connaître les
//! bornes du rendu de l'application : un tableau s'y afficherait en charabia,
//! sans prévenir personne.

use kernel::legal::{constructions_inconnues, lire, FICHIERS};

#[test]
fn les_fichiers_reels_ne_portent_que_la_grammaire_close() {
    for (fichier, contenu) in FICHIERS {
        let texte = lire(fichier, contenu).unwrap_or_else(|e| panic!("{e}"));
        let inconnues = constructions_inconnues(texte.body.as_deref().unwrap_or(""));
        assert!(
            inconnues.is_empty(),
            "{fichier} : {}",
            inconnues
                .iter()
                .map(|(ligne, c)| format!("ligne {ligne} du corps, {c}"))
                .collect::<Vec<_>>()
                .join(" ; ")
        );
    }
}

/// Le contrôle mord : chaque construction refusée est nommée, à sa ligne.
#[test]
fn chaque_construction_hors_grammaire_est_nommee() {
    let corps = "## Données\n\nUn paragraphe *souligné* avec un [lien](https://ifdd.francophonie.org).\n\n- une puce\n1. un numéro\n\n| a | b |\n|---|---|\nVoir la note[^1].\n![logo](x.png)\n```\ncode\n```\n# Titre\n> citation\n<b>gras</b>\n";
    let trouvees = constructions_inconnues(corps);
    let noms: Vec<&str> = trouvees.iter().map(|(_, c)| *c).collect();
    for attendue in [
        "tableau",
        "note de bas de page",
        "image",
        "bloc de code",
        "titre d'un niveau autre que 2 ou 3",
        "citation",
        "HTML",
    ] {
        assert!(
            noms.contains(&attendue),
            "{attendue} n'est pas repéré : {trouvees:?}"
        );
    }
    assert_eq!(trouvees[0], (8, "tableau"), "la ligne est celle du corps");
    assert!(
        !trouvees.iter().any(|(l, _)| *l <= 6),
        "titres, paragraphes, listes, liens et emphase passent : {trouvees:?}"
    );
}
