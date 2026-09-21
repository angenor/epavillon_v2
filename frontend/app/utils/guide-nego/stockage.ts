/**
 * Les clés que Guide Négo pose sur le téléphone. Toutes préfixées `gn.` : le site ne
 * doit rien trouver de nous, et nous ne devons rien lire de lui.
 *
 * En navigation privée ou stockage refusé, `localStorage` LÈVE au lieu de rendre nul.
 * Tout passe donc par ces deux fonctions.
 */
export const CLE_OUVERTURE_VUE = 'gn.ouverture-vue'
export const CLE_GARDE_ANNONCEE = 'gn.garde-annoncee'

export function lireCle(cle: string): string | null {
  try {
    return localStorage.getItem(cle)
  } catch {
    return null
  }
}

export function poserCle(cle: string, valeur = '1'): void {
  try {
    localStorage.setItem(cle, valeur)
  } catch {
    /* Un stockage refusé ne doit jamais empêcher l'écran de s'ouvrir. */
  }
}
