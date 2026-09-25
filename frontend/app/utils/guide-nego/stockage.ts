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
