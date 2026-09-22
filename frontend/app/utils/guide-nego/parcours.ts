/**
 * Où mène « la suite », à chaque marche du parcours d'entrée.
 *
 * **Un seul endroit, et c'est le point.** Cinq écrans y renvoient — la création
 * de compte, la connexion, le retour depuis le courriel de vérification, la
 * saisie du code, et le verrou d'un module réservé. Écrire l'adresse dans
 * chacun obligerait à se souvenir des cinq, et l'un resterait sur l'ancienne
 * sans que rien ne le dise.
 */

/**
 * Étape 1 franchie : le compte est en règle, le code d'invitation suit.
 *
 * L'étape 2 est livrée : un compte en règle mène à la saisie du code.
 */
export const APRES_LE_COMPTE = '/guide-nego/code'

/**
 * Étape 2 franchie : l'accès est ouvert. La suite est le choix des thématiques,
 * dernière marche de l'entrée (FR-003).
 *
 * **La sortie « Plus tard » n'est plus la même** : qui n'a pas saisi de code va
 * lire, et se verra proposer ses thématiques depuis « Ma journée », une seule
 * fois. Les envoyer là depuis un écran qu'on a choisi de quitter serait la même
 * marche imposée deux fois.
 */
export const APRES_LE_CODE = '/guide-nego/thematiques'

/** « Plus tard », à toutes les marches du parcours : on va lire. */
export const PLUS_TARD = '/guide-nego'
