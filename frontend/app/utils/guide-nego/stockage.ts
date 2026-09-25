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
