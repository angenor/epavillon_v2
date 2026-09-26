/**
 * Le centre de notifications et la cloche (FR-027) — les 50 dernières de Guide Négo,
 * gardées sous `notifications` avec l'heure de lecture.
 *
 * **Marquer lu passe par la file, une intention par notification** (`lu:<id>`) : la
 * file garde une entrée par clé, et un lot écraserait le précédent. Ce qui attend
 * compte déjà comme lu, pour que la cloche ne remonte pas sous les doigts.
 */
import type { NotificationFeed } from '~/types/engagement'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { ecrireGarde, magasinDesEcritures } from '~/utils/guide-nego/garde'
import {
  CLE_LECTURE_NOTIFICATIONS,
  PREFIXE_FILE_LU,
  avecLesLectures,
  idsNonLues,
  notificationsParJour,
} from '~/utils/guide-nego/signalements'

interface NotificationsLues {
  personne: string
  feed: NotificationFeed
}

const VIDE: NotificationFeed = { items: [], unread_count: 0 }

export function useGnNotifications() {
  const api = useApi().notifications
  const session = useGnSession()
  const edition = useGnEdition()
  const file = useGnFile()
  const luesParPersonne = useState<Record<string, string[]>>('gn-notifications-lues', () => ({}))

  const lecture = useGnLecture<NotificationsLues>(CLE_LECTURE_NOTIFICATIONS, async () => {
    const personne = session.compte.value.id
    if (!personne) throw new Error('sans compte')
    try {
      return { personne, feed: await api.filGuideNego() }
    } catch (erreur) {
      if (normalizeApiError(erreur) instanceof ApiRequestError) return { personne, feed: VIDE }
      throw erreur
    }
  })

  const lu = computed<NotificationsLues | null>(() => {
    const valeur = lecture.etat.value.valeur
    return valeur && valeur.personne === session.compte.value.id ? valeur : null
  })

  const feed = computed<NotificationFeed>(() =>
    avecLesLectures(lu.value?.feed ?? VIDE, new Set(luesParPersonne.value[session.compte.value.id ?? ''] ?? []), new Date()),
  )
  const fuseau = computed(() => edition.edition.value?.timezone ?? 'UTC')

  async function relireLaFile(): Promise<void> {
    const personne = session.compte.value.id
    if (!personne) return
    const intentions = await magasinDesEcritures.lire().catch(() => [])
    luesParPersonne.value = {
      ...luesParPersonne.value,
      [personne]: intentions
        .filter((i) => i.personne === personne && i.cle.startsWith(PREFIXE_FILE_LU))
        .map((i) => i.cle.slice(PREFIXE_FILE_LU.length)),
    }
  }

  /** Posé dans la garde : l'intention partie, la lecture reste vraie jusqu'à la relecture suivante. */
  async function appliquer(ids: string[]): Promise<void> {
    const etat = lecture.etat.value
    const personne = session.compte.value.id
    if (!personne || etat.valeur?.personne !== personne) return
    const valeur: NotificationsLues = { personne, feed: avecLesLectures(etat.valeur.feed, new Set(ids), new Date()) }
    lecture.etat.value = { ...etat, valeur }
    await ecrireGarde({ cle: CLE_LECTURE_NOTIFICATIONS, valeur, lu_a: etat.luA ?? new Date().toISOString(), empreinte: null })
  }

  file.inscrireFamille(
    PREFIXE_FILE_LU,
    async (intention) => {
      const { ids } = intention.corps as { ids: string[] }
      await api.marquerLues(ids)
      await appliquer(ids)
    },
    relireLaFile,
  )

  function assurer(): void {
    if (!session.connectee.value) return
    void relireLaFile()
    void lecture.rafraichir()
  }

  async function marquer(ids: string[]): Promise<void> {
    const personne = session.compte.value.id
    if (!personne || ids.length === 0) return
    luesParPersonne.value = {
      ...luesParPersonne.value,
      [personne]: [...new Set([...(luesParPersonne.value[personne] ?? []), ...ids])],
    }
    await appliquer(ids)
    for (const id of ids) await file.poser(`${PREFIXE_FILE_LU}${id}`, { ids: [id] }, null)
    await file.partir()
    await relireLaFile()
  }

  return {
    notifications: computed(() => feed.value.items),
    parJour: computed(() => notificationsParJour(feed.value.items, fuseau.value)),
    /** Le compteur jaune de la cloche. */
    nonLues: computed(() => feed.value.unread_count),
    fuseau,
    pret: computed(() => lecture.etat.value.pret),
    connu: computed(() => lu.value !== null),
    luA: computed(() => lecture.etat.value.luA),
    assurer,
    rafraichir: lecture.rafraichir,
    marquerLue: (id: string) => marquer(feed.value.items.some((n) => n.id === id && !n.read_at) ? [id] : []),
    /** Une intention par notification non lue de la liste — jamais un marquage sans identifiants. */
    toutMarquer: () => marquer(idsNonLues(feed.value.items)),
  }
}
