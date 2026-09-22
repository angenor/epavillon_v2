/**
 * Le rendu d'un texte long — **grammaire close**, sans bibliothèque.
 *
 * Titres de niveau 2 et 3, paragraphes, listes (`- ` et `1. `), liens
 * `[texte](adresse)`, emphase `*…*` et `**…**`. Tout le reste reste du texte, et
 * s'affiche tel quel : rien n'est jamais injecté en HTML. Le contrôle côté API
 * (`kernel::legal::constructions_inconnues`) refuse, à la construction, un
 * fichier qui emploierait autre chose — c'est ce qui rend ce rendu tenable.
 */

export type Segment =
  | { type: 'texte'; texte: string }
  | { type: 'gras'; texte: string }
  | { type: 'italique'; texte: string }
  | { type: 'lien'; texte: string; adresse: string }

export type Bloc =
  | { type: 'titre2' | 'titre3' | 'paragraphe'; segments: Segment[] }
  | { type: 'liste' | 'liste-numerotee'; elements: Segment[][] }

/** Seules ces adresses deviennent des liens ; les autres restent leur texte. */
const ADRESSE_SURE = /^(https?:\/\/|mailto:)/i

const EN_LIGNE = /\*\*([^*]+)\*\*|\*([^*]+)\*|\[([^\]]+)\]\(([^)\s]+)\)/g

export function segmentsDe(ligne: string): Segment[] {
  const segments: Segment[] = []
  let depuis = 0
  for (const trouve of ligne.matchAll(EN_LIGNE)) {
    const index = trouve.index ?? 0
    if (index > depuis) segments.push({ type: 'texte', texte: ligne.slice(depuis, index) })
    const [, gras, italique, lien, adresse] = trouve
    if (gras !== undefined) segments.push({ type: 'gras', texte: gras })
    else if (italique !== undefined) segments.push({ type: 'italique', texte: italique })
    else if (lien !== undefined && adresse !== undefined && ADRESSE_SURE.test(adresse)) {
      segments.push({ type: 'lien', texte: lien, adresse })
    } else segments.push({ type: 'texte', texte: lien ?? trouve[0] })
    depuis = index + trouve[0].length
  }
  if (depuis < ligne.length) segments.push({ type: 'texte', texte: ligne.slice(depuis) })
  return segments
}

const PUCE = /^- (.*)$/
const NUMERO = /^\d+\. (.*)$/

export function blocsDe(markdown: string): Bloc[] {
  const blocs: Bloc[] = []
  let paragraphe: string[] = []

  const fermerLeParagraphe = () => {
    if (paragraphe.length) blocs.push({ type: 'paragraphe', segments: segmentsDe(paragraphe.join(' ')) })
    paragraphe = []
  }

  for (const brute of markdown.split('\n')) {
    const ligne = brute.trim()
    const puce = PUCE.exec(ligne)
    const numero = NUMERO.exec(ligne)

    if (!ligne) {
      fermerLeParagraphe()
    } else if (ligne.startsWith('### ')) {
      fermerLeParagraphe()
      blocs.push({ type: 'titre3', segments: segmentsDe(ligne.slice(4)) })
    } else if (ligne.startsWith('## ')) {
      fermerLeParagraphe()
      blocs.push({ type: 'titre2', segments: segmentsDe(ligne.slice(3)) })
    } else if (puce || numero) {
      fermerLeParagraphe()
      const type = puce ? 'liste' : 'liste-numerotee'
      const element = segmentsDe((puce ?? numero)![1]!)
      const dernier = blocs.at(-1)
      if (dernier && dernier.type === type) dernier.elements.push(element)
      else blocs.push({ type, elements: [element] })
    } else {
      paragraphe.push(ligne)
    }
  }
  fermerLeParagraphe()
  return blocs
}
