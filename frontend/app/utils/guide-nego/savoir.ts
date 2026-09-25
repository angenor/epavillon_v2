/**
 * Le savoir gardé sur le téléphone : une différence (`?since=`) s'applique sur le
 * corpus gardé, et le corpus se lit avec ses liens filtrés.
 *
 * LE CORPUS GARDÉ PORTE TOUS LES LIENS. Une différence ne renvoie pas le parent
 * quand une entrée liée change d'état : filtrer `related_ids` avant de garder
 * perdrait le lien le jour où sa cible est publiée. Le filtre se fait à la
 * lecture, par `lisible`.
 */
import type { FaqEntry, GlossaryEntry, KnowledgeBundle } from '../../types/negotiation-savoir.ts'
import { normaliserTerme } from './lexique.ts'

const VIDE = { faq: [], glossary: [] }

function appliquer<T extends { id: string }>(gardees: T[], changees: T[], retirees: string[]): T[] {
  const sorties = new Set(retirees)
  const nouvelles = new Map(changees.map((e) => [e.id, e]))
  const suite = gardees.filter((e) => !sorties.has(e.id)).map((e) => nouvelles.get(e.id) ?? e)
  const presentes = new Set(suite.map((e) => e.id))
  return [...suite, ...changees.filter((e) => !presentes.has(e.id))]
}

// L'ordre du serveur : la FAQ par rubrique (l'ordre d'arrivée départage), le lexique par terme normalisé.
function ordonner(corpus: KnowledgeBundle): KnowledgeBundle {
  const rang = new Map(corpus.faq_sections.map((s) => [s.code, s.sort_order]))
  const faq = corpus.faq
    .map((entree, i) => ({ entree, i }))
    .sort((a, b) => (rang.get(a.entree.section_code) ?? 0) - (rang.get(b.entree.section_code) ?? 0) || a.i - b.i)
    .map(({ entree }) => entree)
  const glossary = corpus.glossary
    .map((entree) => ({ entree, cle: normaliserTerme(entree.term) }))
    .sort((a, b) => (a.cle < b.cle ? -1 : a.cle > b.cle ? 1 : a.entree.id < b.entree.id ? -1 : 1))
    .map(({ entree }) => entree)
  return { ...corpus, faq, glossary }
}

/**
 * Le corpus après une lecture : une lecture entière remplace, une différence se
 * fusionne — entrées remplacées ou ajoutées, `removed` retirées ; rubriques,
 * familles, parcours et « les plus lues » remplacés. Idempotente : le chevauchement
 * de cinq minutes du serveur ne coûte rien.
 */
export function fusionner(garde: KnowledgeBundle | null, difference: KnowledgeBundle): KnowledgeBundle {
  if (difference.complete || !garde) return ordonner({ ...difference, complete: true, removed: VIDE })
  return ordonner({
    served_at: difference.served_at,
    complete: true,
    faq_sections: difference.faq_sections,
    glossary_families: difference.glossary_families,
    faq: appliquer(garde.faq, difference.faq, difference.removed.faq),
    glossary: appliquer(garde.glossary, difference.glossary, difference.removed.glossary),
    pathway: difference.pathway,
    most_read: difference.most_read,
    removed: VIDE,
  })
}

/** Le corpus tel qu'il se lit : les liens vers une entrée absente du téléphone tombent. */
export function lisible(corpus: KnowledgeBundle): KnowledgeBundle {
  const faq = new Set(corpus.faq.map((e) => e.id))
  const lexique = new Set(corpus.glossary.map((e) => e.id))
  const filtrer = <T extends FaqEntry | GlossaryEntry>(e: T, presentes: Set<string>): T =>
    e.related_ids.every((id) => presentes.has(id)) ? e : { ...e, related_ids: e.related_ids.filter((id) => presentes.has(id)) }
  return {
    ...corpus,
    faq: corpus.faq.map((e) => filtrer(e, faq)),
    glossary: corpus.glossary.map((e) => filtrer(e, lexique)),
    most_read: corpus.most_read.filter((id) => faq.has(id)),
  }
}
