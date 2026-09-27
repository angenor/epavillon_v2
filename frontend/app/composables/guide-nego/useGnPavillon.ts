/**
 * Le Pavillon de la Francophonie de l'édition, lu en une réponse et gardé sous
 * `pavillon:<slug>` avec son lieu ; le détail d'une activité et son formulaire, sous
 * `pavillon-activite:<édition>:<slug>`. Tout se relit hors connexion, avec l'heure de
 * lecture (FR-010). **Jamais filtré par les thématiques suivies** (FR-003).
 */
import type { RegistrationForm, RegistrationFormField } from '~/types/programme/registration'
import type { PublicScheduleRow, PublicSessionDetail } from '~/types/views'
import { estInchange } from '~/composables/api/etiquete'
import { cleDeLActivite, cleDuPavillon, lieuDuPavillon, type PavillonGarde } from '~/utils/guide-nego/pavillon'

export interface FormulaireInscription {
  form: RegistrationForm
  fields: RegistrationFormField[]
}

interface ActiviteGardee {
  slugEdition: string
  slug: string
  /** Nul : adresse inconnue ou activité dépubliée. */
  detail: PublicSessionDetail | null
  formulaire: FormulaireInscription | null
}

export function useGnPavillon() {
  const api = useApi()
  const edition = useGnEdition()

  const lecture = useGnLecture<PavillonGarde>(
    'pavillon',
    async (garde) => {
      const lue = await edition.lue()
      if (!lue?.id) throw new Error('aucune édition')
      const connue = garde?.slug === lue.slug && garde.eventId === lue.id ? garde : null
      const [lu, lieux] = await Promise.all([
        api.pavillon.edition(lue.id, connue?.empreinte ?? null),
        api.events.venues(lue.id).catch(() => null),
      ])
      const lieu = lieux ? lieuDuPavillon(lieux) : (connue?.lieu ?? null)
      if (!estInchange(lu)) return { slug: lue.slug, eventId: lue.id, activites: lu.valeur, lieu, empreinte: lu.empreinte }
      if (connue) return { ...connue, lieu, empreinte: lu.empreinte }
      throw new Error('304 sans programme gardé')
    },
    {
      delaiMs: 15_000,
      cleDeGarde: async () => {
        const gardee = await edition.gardee()
        return gardee ? cleDuPavillon(gardee.slug) : null
      },
    },
  )

  const garde = computed(() => {
    const valeur = lecture.etat.value.valeur
    const slug = edition.edition.value?.slug ?? valeur?.slug ?? null
    return valeur && valeur.slug === slug ? valeur : null
  })

  const activites = computed<PublicScheduleRow[]>(() => garde.value?.activites ?? [])
  const fuseau = computed<string | null>(() => edition.edition.value?.timezone ?? activites.value[0]?.timezone ?? null)
  const sansEdition = computed(() => edition.etat.value.source === 'reseau' && !edition.edition.value)

  async function rafraichir(): Promise<void> {
    if (!(await edition.gardee()) && !(await edition.lue())) return
    await lecture.rafraichir()
  }

  const activite = (id: string) => activites.value.find((a) => a.id === id) ?? null
  const activiteParSlug = (slug: string) => activites.value.find((a) => a.slug === slug) ?? null

  return {
    activites,
    lieu: computed(() => garde.value?.lieu ?? null),
    fuseau,
    sansEdition,
    activite,
    activiteParSlug,
    /** L'étiquette « Se tient aussi au Pavillon » : le slug de l'activité liée, s'il est connu (FR-008). */
    slugDeLActiviteLiee: (sessionId: string | null) => (sessionId ? (activite(sessionId)?.slug ?? null) : null),
    pret: computed(() => lecture.etat.value.pret),
    luA: computed(() => lecture.etat.value.luA),
    etat: lecture.etat,
    rafraichir,
  }
}

/** Le détail d'une activité et son formulaire ; la ligne de l'édition gardée tient lieu de repli. */
export function useGnActivitePavillon(slug: string) {
  const api = useApi()
  const edition = useGnEdition()
  const pavillon = useGnPavillon()

  const lecture = useGnLecture<ActiviteGardee>(
    `pavillon-activite:${slug}`,
    async (garde) => {
      const lue = await edition.lue()
      if (!lue?.id) throw new Error('aucune édition')
      const detail = await api.pavillon.activite(lue.id, slug)
      const connu = garde?.slugEdition === lue.slug ? garde.formulaire : null
      const formulaire: FormulaireInscription | null = detail
        ? await api.registrations.form(detail.session.id).catch(() => connu)
        : null
      return { slugEdition: lue.slug, slug, detail, formulaire }
    },
    {
      cleDeGarde: async () => {
        const gardee = await edition.gardee()
        return gardee ? cleDeLActivite(gardee.slug, slug) : null
      },
    },
  )

  const garde = computed(() => {
    const valeur = lecture.etat.value.valeur
    const slugEdition = edition.edition.value?.slug ?? valeur?.slugEdition ?? null
    return valeur && valeur.slugEdition === slugEdition && valeur.slug === slug ? valeur : null
  })

  return {
    detail: computed(() => garde.value?.detail ?? null),
    /** Le détail s'il a été lu, sinon la ligne de l'édition : la fiche s'ouvre sans réseau. */
    activite: computed<PublicScheduleRow | null>(() => garde.value?.detail?.session ?? pavillon.activiteParSlug(slug)),
    formulaire: computed(() => garde.value?.formulaire ?? null),
    /** L'API a répondu que l'activité n'existe pas (ou plus) : ce n'est pas une panne. */
    introuvable: computed(() => lecture.etat.value.source === 'reseau' && garde.value !== null && garde.value.detail === null),
    pret: computed(() => lecture.etat.value.pret),
    luA: computed(() => lecture.etat.value.luA),
    etat: lecture.etat,
    rafraichir: lecture.rafraichir,
  }
}
