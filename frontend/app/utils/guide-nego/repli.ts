/**
 * Le texte replié, sans accents ni casse : la recherche du lecteur et l'ancrage des
 * notes de correction comparent ainsi, et renvoient au texte d'origine.
 */
// « CO₂ » se tape « co2 », « Paris–Nairobi » « paris-nairobi » : NFKD ramène exposants
// et indices, la table le reste.
const REMPLACEMENTS: Record<string, string> = { œ: 'oe', æ: 'ae', '’': "'", '–': '-', '—': '-', '‑': '-', '‐': '-' }

/**
 * Le texte replié, et pour chacune de ses lettres la position de la lettre d'origine.
 * Les blancs se réduisent à une espace : une double espace d'extraction ne cache rien.
 */
export function replier(texte: string): { replie: string; origine: number[] } {
  let replie = ''
  const origine: number[] = []
  for (let i = 0; i < texte.length; i += 1) {
    const lettre = texte[i] as string
    if (/\s/u.test(lettre)) {
      if (!replie.endsWith(' ')) {
        replie += ' '
        origine.push(i)
      }
      continue
    }
    const basse = lettre.toLowerCase()
    const pliee = REMPLACEMENTS[basse] ?? basse.normalize('NFKD').replace(/\p{M}/gu, '')
    for (const c of pliee) {
      replie += c
      origine.push(i)
    }
  }
  return { replie, origine }
}
