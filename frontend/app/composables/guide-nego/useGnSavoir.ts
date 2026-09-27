/**
 * Le savoir — FAQ, parcours, lexique —, lu d'un bloc et gardé pour le hors connexion.
 *
 * Relu à chaque ouverture avec réseau : par différence depuis la lecture gardée,
 * `304` si rien n'a changé. Une garde d'une autre langue ne se complète pas, elle
 * se remplace par une lecture entière.
 *
 * Les index de recherche se préparent une fois par corpus, hors de l'état gardé :
 * un `useState` se sérialise, un index n'a rien à y faire.
 */
import type { FaqEntry, FaqSection, GlossaryEntry, GlossaryFamily, KnowledgeBundle, PathwayGroup } from '~/types/negotiation-savoir'
import { estInchange } from '~/composables/api/etiquete'
import { chercher, preparerIndex, type IndexFlou, type ResultatsFlous } from '~/utils/guide-nego/recherche-floue'
import { grouperParLettre, resoudreLeTerme, type GroupeDeLettre } from '~/utils/guide-nego/lexique'
import { fusionner, lisible } from '~/utils/guide-nego/savoir'

interface SavoirGarde {
  corpus: KnowledgeBundle
  empreinte: string | null
  /** Les textes arrivent résolus : une autre langue ne se complète pas. */
  langue: string
}

interface Prepare {
  lisible: KnowledgeBundle
  lexique: IndexFlou<GlossaryEntry> | null
  faq: IndexFlou<FaqEntry> | null
}

// Par corpus gardé, partagé entre les écrans : chaque index ne se prépare qu'une fois.
const prepares = new WeakMap<KnowledgeBundle, Prepare>()

function preparer(garde: KnowledgeBundle): Prepare {
  let prepare = prepares.get(garde)
  if (!prepare) prepares.set(garde, (prepare = { lisible: lisible(garde), lexique: null, faq: null }))
  return prepare
}

function indexDuLexique(prepare: Prepare): IndexFlou<GlossaryEntry> {
  prepare.lexique ??= preparerIndex(
    prepare.lisible.glossary.map((e) => ({ valeur: e, noms: [e.term, e.acronym, ...e.variants], traductions: [e.translation] })),
  )
  return prepare.lexique
}

function indexDeLaFaq(prepare: Prepare): IndexFlou<FaqEntry> {
  prepare.faq ??= preparerIndex(prepare.lisible.faq.map((e) => ({ valeur: e, noms: [e.question], corps: [e.answer] })))
  return prepare.faq
}

export function useGnSavoir() {
  const api = useApi().guideNegoSavoir
  const { locale } = useI18n()

  const lecture = useGnLecture<SavoirGarde>('savoir', async (garde) => {
    const langue = String(locale.value)
    const connue = garde?.langue === langue ? garde : null
    const lu = await api.paquet(connue ? { since: connue.corpus.served_at, empreinte: connue.empreinte } : null)
    if (estInchange(lu)) {
      if (connue) return { ...connue, empreinte: lu.empreinte }
      throw new Error('304 sans savoir gardé')
    }
    return { corpus: fusionner(connue?.corpus ?? null, lu.valeur), empreinte: lu.empreinte, langue }
  })

  /** Rend la main dès qu'un verdict existe — la garde, ou à défaut le réseau — et relit derrière. */
  async function assurer(): Promise<void> {
    if (lecture.etat.value.pret) {
      void lecture.rafraichir()
      return
    }
    await lecture.rafraichir()
  }

  watch(locale, () => void lecture.rafraichir())

  const prepare = computed<Prepare | null>(() => {
    const garde = lecture.etat.value.valeur
    return garde ? preparer(toRaw(garde.corpus)) : null
  })
  const corpus = computed<KnowledgeBundle | null>(() => prepare.value?.lisible ?? null)

  const faq = computed<FaqEntry[]>(() => corpus.value?.faq ?? [])
  const lexique = computed<GlossaryEntry[]>(() => corpus.value?.glossary ?? [])
  const rubriques = computed<FaqSection[]>(() => corpus.value?.faq_sections ?? [])
  const familles = computed<GlossaryFamily[]>(() => corpus.value?.glossary_families ?? [])
  const parcours = computed<PathwayGroup[]>(() => corpus.value?.pathway.groups ?? [])
  const plusLues = computed<FaqEntry[]>(() =>
    (corpus.value?.most_read ?? []).flatMap((id) => faq.value.find((e) => e.id === id) ?? []),
  )
  const parLettre = computed<GroupeDeLettre[]>(() => grouperParLettre(lexique.value))

  const vide = <T>(): ResultatsFlous<T> => ({ trouves: [], vousCherchiezPeutEtre: false, aussiDansLesTraductions: false })

  return {
    etat: lecture.etat,
    rafraichir: lecture.rafraichir,
    assurer,
    faq,
    lexique,
    rubriques,
    familles,
    parcours,
    plusLues,
    parLettre,
    chercherDansLeLexique: (saisie: string): ResultatsFlous<GlossaryEntry> =>
      prepare.value ? chercher(indexDuLexique(prepare.value), saisie) : vide(),
    chercherDansLaFaq: (saisie: string): ResultatsFlous<FaqEntry> =>
      prepare.value ? chercher(indexDeLaFaq(prepare.value), saisie) : vide(),
    /** Le contrat de `resolution-lexique.md` : jamais d'approché. */
    resoudre: (texte: string): GlossaryEntry | null => resoudreLeTerme(texte, lexique.value),
    termeDe: (slug: string): GlossaryEntry | null => lexique.value.find((e) => e.slug === slug) ?? null,
    entreeDeFaq: (id: string): FaqEntry | null => faq.value.find((e) => e.id === id) ?? null,
  }
}
