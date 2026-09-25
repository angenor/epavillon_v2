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

/**
 * Après la connexion, le code n'est demandé qu'à qui n'est pas admis. Admise
 * depuis un autre appareil, la personne va lire ; « Ma journée » lui propose ses
 * thématiques si elle n'en suit aucune.
 */
export function apresLaConnexion(accesOuvert: boolean): string {
  return accesOuvert ? PLUS_TARD : APRES_LE_COMPTE
}

/** Au-delà d'une heure, la personne ne se souvient plus d'où elle venait. */
const RETOUR_VALIDE_MS = 60 * 60 * 1000

/**
 * Un geste réservé au compte — retour sur la FAQ, signalement — mène à la connexion,
 * parfois par la création du compte et la confirmation de l'adresse : l'écran d'origine
 * est gardé sur le téléphone, pour y revenir ensuite.
 */
export function retourAGarder(chemin: string, maintenant: number): string {
  return JSON.stringify({ chemin, a: maintenant })
}

/** Seul un chemin de Guide Négo, récent, ramène : jamais une adresse extérieure. */
export function retourALire(brut: string | null, maintenant: number): string | null {
  if (!brut) return null
  try {
    const { chemin, a } = JSON.parse(brut) as { chemin?: unknown; a?: unknown }
    if (typeof chemin !== 'string' || typeof a !== 'number') return null
    if (!chemin.startsWith('/guide-nego/') || chemin.includes('//') || chemin.includes('\\')) return null
    return maintenant - a >= 0 && maintenant - a < RETOUR_VALIDE_MS ? chemin : null
  } catch {
    return null
  }
}
