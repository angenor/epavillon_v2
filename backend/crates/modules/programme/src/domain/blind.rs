//! Le voile de l'évaluation en aveugle (R4).
//!
//! # Ce n'est pas un filtre : ce qui est masqué n'est pas lu
//!
//! Quand le voile est baissé, la requête qui lit les revues des pairs **n'est
//! pas exécutée**. Lire puis vider les champs sensibles laisse la donnée à
//! portée d'un champ oublié dans un type de sortie, d'une trace de débogage,
//! d'un message d'erreur enrichi. Ne pas lire supprime la classe entière de
//! défauts — et c'est ce qui rend le test possible : on inspecte la **charge
//! utile**, pas l'écran.
//!
//! Le contrat du front l'écrit en toutes lettres : « ce qui n'est pas envoyé ne
//! peut pas fuiter ».
//!
//! **Le décompte, lui, est lu.** Compter n'ancre pas ; lire, si.

/// Les trois conditions, telles que le contrat les pose.
#[derive(Debug, Clone, Copy)]
pub struct Lecteur {
    /// `event.calls_for_proposals.blind_review` — la règle de l'appel.
    pub appel_en_aveugle: bool,
    /// Une affectation existe sur ce dossier, **déport non compris** : un
    /// membre déporté ne posera plus de note, il n'y a rien à ancrer.
    pub affecte: bool,
    /// Sa propre revue est déposée — `reviews.submitted_at` non nulle.
    pub revue_deposee: bool,
}

/// Le voile est-il baissé pour ce lecteur ?
///
/// **Le cas qui décide de la forme de cette fonction** : un administrateur qui
/// tranche sans noter n'est pas affecté, donc n'est pas voilé. L'ancrage vise
/// celui qui va **poser une note** ; masquer les notes à qui doit décider
/// rendrait la décision impossible.
///
/// Et le voile **se lève à la seconde où sa propre revue part** : c'est pour
/// cela que la condition porte sur le dépôt de la revue et non sur son
/// existence — une revue en brouillon ne compte dans aucun agrégat et n'est
/// visible d'aucun pair, elle ne lève donc rien.
pub fn voile_baisse(lecteur: Lecteur) -> bool {
    lecteur.appel_en_aveugle && lecteur.affecte && !lecteur.revue_deposee
}
