/**
 * Les réunions de la Francophonie de l'édition, lues en une réponse et gardées sous
 * `reunions:<slug>` : liste et fiches se relisent hors connexion, avec leur heure de
 * lecture (FR-006). Lecture publique : aucun lien de visio ici (R6).
 */
import type { FrancophoneMeeting } from '~/types/negotiation-meetings'
import { estInchange } from '~/composables/api/etiquete'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { cleDesReunions, trierLesReunions, type ReunionsGardees } from '~/utils/guide-nego/reunions'

const editionInconnue = (erreur: unknown) => {
  const e = normalizeApiError(erreur)
  return e instanceof ApiRequestError && e.code === 'NEGOTIATION_EDITION_UNKNOWN'
}

export function useGnReunions() {
  const api = useApi().negotiationMeetings
  const edition = useGnEdition()

  const lecture = useGnLecture<ReunionsGardees>(
    'reunions',
    async (garde) => {
      const lue = await edition.lue()
      if (!lue) throw new Error('aucune édition')
      const connue = garde?.slug === lue.slug ? garde : null
      try {
        const lu = await api.reunions(lue.slug, connue?.empreinte ?? null)
        if (!estInchange(lu)) return { slug: lue.slug, lues: lu.valeur, empreinte: lu.empreinte }
        if (connue) return { ...connue, empreinte: lu.empreinte }
        throw new Error('304 sans réunions gardées')
      } catch (erreur) {
        if (editionInconnue(erreur)) return { slug: lue.slug, lues: null, empreinte: null }
        throw erreur
      }
    },
    {
      delaiMs: 15_000,
      cleDeGarde: async () => {
        const gardee = await edition.gardee()
        return gardee ? cleDesReunions(gardee.slug) : null
      },
    },
  )

  // Avant que l'édition soit relue, la garde a été choisie par l'édition gardée : son slug fait foi.
  const lues = computed(() => {
    const garde = lecture.etat.value.valeur
    const slug = edition.edition.value?.slug ?? garde?.slug ?? null
    return garde && garde.slug === slug ? garde.lues : null
  })

  const reunions = computed<FrancophoneMeeting[]>(() => trierLesReunions(lues.value?.meetings ?? []))
  /** Le fuseau de la COP : celui de la réponse, sinon celui de l'édition gardée. */
  const fuseau = computed<string | null>(() => lues.value?.edition.timezone ?? edition.edition.value?.timezone ?? null)
  const ville = computed<string | null>(() => lues.value?.edition.city ?? edition.edition.value?.city ?? null)

  /** Aucune COP climat à venir : rien à lire, et ce n'est pas une panne de réseau. */
  const sansEdition = computed(() => edition.etat.value.source === 'reseau' && !edition.edition.value)

  async function rafraichir(): Promise<void> {
    if (!(await edition.gardee()) && !(await edition.lue())) return
    await lecture.rafraichir()
  }

  return {
    reunions,
    reunion: (id: string) => reunions.value.find((r) => r.id === id) ?? null,
    fuseau,
    ville,
    sansEdition,
    pret: computed(() => lecture.etat.value.pret),
    /** L'heure de la dernière lecture réussie, sur le téléphone. */
    luA: computed(() => lecture.etat.value.luA),
    /** L'état de la lecture : `luA`, `source`, `pret`, `enCours`. */
    etat: lecture.etat,
    rafraichir,
  }
}
