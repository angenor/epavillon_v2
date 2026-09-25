/**
 * Les sessions de négociation de l'édition, lues en une réponse et gardées sous
 * `sessions:<slug>` : toute la COP se relit hors connexion dès la première lecture.
 *
 * **La coupure se montre, elle ne se tait pas** (FR-039, principe XII) : une
 * réponse coupée remplace la copie gardée, et `affichage` ne rend jamais une
 * session à côté d'un état coupé. L'écran dit « suspendu, voir le programme
 * officiel », jamais « aucune session ».
 */
import { estInchange } from '~/composables/api/etiquete'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import {
  affichageDesSessions,
  apresLecture,
  cleDesSessions,
  type AffichageDesSessions,
  type SessionsGardees,
} from '~/utils/guide-nego/sessions'

const editionInconnue = (erreur: unknown) => {
  const e = normalizeApiError(erreur)
  return e instanceof ApiRequestError && e.code === 'NEGOTIATION_EDITION_UNKNOWN'
}

export function useGnSessions() {
  const api = useApi().negotiationSessions
  const edition = useGnEdition()

  const lecture = useGnLecture<SessionsGardees>(
    'sessions',
    async (garde) => {
      const lue = await edition.lue()
      if (!lue) throw new Error('aucune édition')
      const connue = garde?.slug === lue.slug ? garde : null
      try {
        const lu = await api.sessions(lue.slug, connue?.empreinte ?? null)
        return apresLecture(connue, lue.slug, estInchange(lu) ? lu : { lues: lu.valeur, empreinte: lu.empreinte })
      } catch (erreur) {
        if (editionInconnue(erreur)) return apresLecture(connue, lue.slug, { lues: null, empreinte: null })
        throw erreur
      }
    },
    {
      delaiMs: 15_000,
      cleDeGarde: async () => {
        const gardee = await edition.gardee()
        return gardee ? cleDesSessions(gardee.slug) : null
      },
    },
  )

  // Avant que l'édition soit relue, la garde des sessions a été choisie par l'édition gardée : son slug fait foi.
  const affichage = computed<AffichageDesSessions>(() => {
    const garde = lecture.etat.value.valeur
    return affichageDesSessions(garde, edition.edition.value?.slug ?? garde?.slug ?? null)
  })

  const sessions = computed(() => (affichage.value.etat === 'sert' ? affichage.value.sessions : []))
  /** Le fuseau de la COP : celui de la réponse, sinon celui de l'édition gardée. */
  const fuseau = computed<string | null>(() =>
    affichage.value.etat === 'sert' ? affichage.value.fuseau : (edition.edition.value?.timezone ?? null),
  )
  const ville = computed<string | null>(() =>
    affichage.value.etat === 'sert' ? affichage.value.ville : (edition.edition.value?.city ?? null),
  )

  /** Aucune COP climat à venir : rien à lire, et ce n'est pas une panne de réseau. */
  const sansEdition = computed(() => edition.etat.value.source === 'reseau' && !edition.edition.value)

  async function rafraichir(): Promise<void> {
    if (!(await edition.gardee()) && !(await edition.lue())) return
    await lecture.rafraichir()
  }

  function session(id: string) {
    return sessions.value.find((s) => s.id === id) ?? null
  }

  return {
    affichage,
    sessions,
    fuseau,
    ville,
    session,
    edition: edition.edition,
    sansEdition,
    /** L'état de la lecture : `luA` (heure du téléphone), `source`, `pret`, `enCours`. */
    etat: lecture.etat,
    rafraichir,
  }
}
