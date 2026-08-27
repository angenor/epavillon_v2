/**
 * Texte brut tiré d'un fragment HTML — le pendant de `UiRichContent` pour les
 * endroits qui ne peuvent pas rendre de structure : un aperçu de carte, une
 * balise `<title>`, un extrait de résultat de recherche. Sans lui, les balises
 * de l'éditeur s'y affichent telles quelles.
 *
 * Les fins de bloc deviennent une espace : sinon deux paragraphes se recollent
 * en un seul mot.
 */
export function richTextToPlain(html: string | null | undefined): string {
  if (!html) return ''

  return decodeEntities(
    html
      .replace(/<\/(p|h[1-6]|li|blockquote|div|tr)>|<br\s*\/?>/gi, ' ')
      .replace(/<[^>]*>/g, ''),
  )
    .replace(/\s+/g, ' ')
    .trim()
}

/**
 * L'éditeur n'échappe que ce qui casserait le balisage, plus l'espace
 * insécable ; le reste sort en UTF-8. On s'en tient donc à cette liste et aux
 * formes numériques.
 */
const NAMED_ENTITIES: Record<string, string> = {
  amp: '&',
  lt: '<',
  gt: '>',
  quot: '"',
  apos: "'",
  nbsp: ' ',
}

function decodeEntities(text: string): string {
  return text.replace(/&(#x?[0-9a-f]+|[a-z]+);/gi, (match, entity: string) => {
    if (entity.startsWith('#')) {
      const code = entity[1]?.toLowerCase() === 'x'
        ? Number.parseInt(entity.slice(2), 16)
        : Number.parseInt(entity.slice(1), 10)
      return Number.isFinite(code) && code > 0 ? String.fromCodePoint(code) : match
    }
    return NAMED_ENTITIES[entity.toLowerCase()] ?? match
  })
}
