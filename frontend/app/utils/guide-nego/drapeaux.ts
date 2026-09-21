/**
 * Ouvrir ou fermer Guide Négo — la règle, écrite une fois.
 *
 * SEULE UNE RÉPONSE RÉUSSIE QUI DIT « ÉTEINT » FERME L'APPLICATION. Une API
 * injoignable, trop lente, en erreur, ou une réponse où le drapeau manque valent
 * « sans réseau » : le dernier état lu tient. Une panne en pleine COP ne doit pas
 * fermer l'application ; et ce qui n'a jamais été lu ouvert reste fermé.
 */
export const DRAPEAU_APPLICATION = 'guide_nego.enabled'
export const DRAPEAU_ECHANGES = 'negotiation.channels'
export const DRAPEAUX_GARDES = [DRAPEAU_APPLICATION, DRAPEAU_ECHANGES] as const

export type CleDrapeau = (typeof DRAPEAUX_GARDES)[number]
export type DrapeauxGardes = Partial<Record<CleDrapeau, boolean>>

interface DrapeauResolu {
  key: string
  is_enabled: boolean
}

/** Ne garde de la réponse que ce qui concerne Guide Négo ; rend `null` si elle est illisible. */
export function extraireDrapeaux(reponse: unknown): DrapeauxGardes | null {
  if (!Array.isArray(reponse)) return null
  const gardes: DrapeauxGardes = {}
  for (const entree of reponse as Partial<DrapeauResolu>[]) {
    const cle = DRAPEAUX_GARDES.find((attendue) => attendue === entree?.key)
    if (cle && typeof entree.is_enabled === 'boolean') gardes[cle] = entree.is_enabled
  }
  return gardes
}

/**
 * @param reponse ce que l'API vient de dire, ou `null` si elle s'est tue
 * @param garde   le dernier état lu, ou `null` si rien n'a jamais été lu
 */
export function resoudreDrapeau(
  reponse: DrapeauxGardes | null,
  garde: DrapeauxGardes | null,
  cle: CleDrapeau,
): boolean {
  return reponse?.[cle] ?? garde?.[cle] ?? false
}

/** Ce qui se garde après une lecture : la réponse, complétée de ce qu'elle ne dit pas. */
export function fusionner(reponse: DrapeauxGardes, garde: DrapeauxGardes | null): DrapeauxGardes {
  return { ...garde, ...reponse }
}

/** Le cinquième onglet ne se voit jamais dans une application fermée. */
export function echangesOuverts(reponse: DrapeauxGardes | null, garde: DrapeauxGardes | null): boolean {
  return (
    resoudreDrapeau(reponse, garde, DRAPEAU_APPLICATION) && resoudreDrapeau(reponse, garde, DRAPEAU_ECHANGES)
  )
}
