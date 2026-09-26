/**
 * Les clés que Guide Négo pose sur le téléphone. Toutes préfixées `gn.` : le site ne
 * doit rien trouver de nous, et nous ne devons rien lire de lui.
 *
 * En navigation privée ou stockage refusé, `localStorage` LÈVE au lieu de rendre nul.
 * Tout passe donc par ces deux fonctions.
 */
export const CLE_OUVERTURE_VUE = 'gn.ouverture-vue'
export const CLE_GARDE_ANNONCEE = 'gn.garde-annoncee'
/** L'écran de premier choix des thématiques a été proposé sur cet appareil. */
export const CLE_THEMATIQUES_PROPOSEES = 'gn.thematiques-proposees'
/** Les derniers termes ouverts, cinq au plus. */
export const CLE_LEXIQUE_DERNIERS = 'gn.lexique.derniers'
/** Les termes favoris de ce téléphone : ceux posés sans compte, et le reflet du compte. */
export const CLE_LEXIQUE_FAVORIS = 'gn.lexique.favoris'
/** La personne dont les favoris sans compte ont déjà rejoint le compte (R7). */
export const CLE_LEXIQUE_FAVORIS_FUSIONNES = 'gn.lexique.favoris-fusionnes'
/** Les étapes cochées sur ce téléphone : celles cochées sans compte, et le reflet du compte. */
export const CLE_PARCOURS = 'gn.parcours'
/** La personne dont les coches sans compte ont déjà rejoint le compte (R7). */
export const CLE_PARCOURS_FUSIONNE = 'gn.parcours-fusionne'
/** Les entrées de FAQ déjà comptées aujourd'hui sur ce téléphone (R12). */
export const CLE_FAQ_LUES = 'gn.faq.lues'
/** L'écran où revenir après la connexion, quand un geste réservé au compte y a mené. */
export const CLE_RETOUR_APRES_CONNEXION = 'gn.retour-apres-connexion'
/** Le terme qu'une personne sans compte proposait, gardé le temps de se connecter. */
export const CLE_TERME_PROPOSE = 'gn.terme-propose'
/** Le rappel d'une session a été montré ; la valeur est son début, qu'un déplacement change. */
export const cleRappelVu = (sessionId: string) => `gn.rappel.vu.${sessionId}`

// Stockage refusé, une clé vit le temps de la visite : sans elle, « Continuer en
// visiteur » ramènerait à l'ouverture, en boucle.
const enMemoire = new Map<string, string>()

export function lireCle(cle: string): string | null {
  try {
    return localStorage.getItem(cle)
  } catch {
    return enMemoire.get(cle) ?? null
  }
}

export function poserCle(cle: string, valeur = '1'): void {
  try {
    localStorage.setItem(cle, valeur)
  } catch {
    enMemoire.set(cle, valeur)
  }
}
