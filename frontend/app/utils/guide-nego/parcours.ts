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
 * Étape 2 franchie : l'accès est ouvert.
 *
 * Tant que le choix des thématiques n'existe pas — étape 3, qui vient avec 0c —,
 * la suite est « Ma journée » : un accès ouvert donne déjà tout ce que
 * l'application réserve. C'est aussi la sortie « Plus tard » de l'écran du code,
 * et les deux sont volontairement la même : dans les deux cas, on va lire.
 */
export const APRES_LE_CODE = '/guide-nego'
