/**
 * La place que Guide Négo occupe sur le téléphone.
 *
 * Tout est pur ici : la mesure vient du navigateur, injectée, pour que le repli —
 * « on ne peut pas mesurer sur cet appareil », jamais un zéro faux — se teste sans lui.
 *
 * **Rien ne se libère à cette étape.** Le geste ne videra que les documents
 * téléchargés, qui arrivent à l'étape 1 — jamais les lectures, qui pèsent quelques
 * dizaines de kilo-octets et font marcher l'application sans réseau.
 */

export interface Place {
  /** Octets occupés : coquille, données lues, documents à venir. */
  utilise: number
  /** Octets encore permis par le navigateur ; nul s'il ne le dit pas. */
  libre: number | null
}

/** Ce que rend `navigator.storage.estimate()`, ou rien quand il manque. */
export interface Estimation {
  usage?: number
  quota?: number
}

/** Nul quand le navigateur ne mesure pas : l'écran le dit au lieu d'annoncer zéro. */
export function placeDe(estimation: Estimation | null): Place | null {
  if (!estimation || typeof estimation.usage !== 'number' || !Number.isFinite(estimation.usage)) return null
  const libre =
    typeof estimation.quota === 'number' && Number.isFinite(estimation.quota)
      ? Math.max(0, estimation.quota - estimation.usage)
      : null
  return { utilise: estimation.usage, libre }
}

const UNITES = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'] as const

/** « 7 Mo », « 2,1 Go » — en puissances de mille, comme l'écrit le système du téléphone. */
export function tailleLisible(octets: number, locale: string): string {
  let valeur = Math.max(0, octets)
  let rang = 0
  while (valeur >= 1000 && rang < UNITES.length - 1) {
    valeur /= 1000
    rang += 1
  }
  return new Intl.NumberFormat(locale, {
    style: 'unit',
    unit: UNITES[rang],
    unitDisplay: 'short',
    maximumFractionDigits: rang === 0 ? 0 : 1,
  }).format(valeur)
}

/** La part occupée, entre 0 et 1 ; nulle quand la place libre est inconnue. */
export function partOccupee(place: Place): number | null {
  if (place.libre === null) return null
  const total = place.utilise + place.libre
  return total > 0 ? Math.min(1, place.utilise / total) : 0
}
