/**
 * « Mon agenda » (FR-030, FR-031, FR-034) — le patron des favoris : une clé de
 * file par session (`agenda-<id>`), la dernière intention gagne, et ce qui
 * s'affiche est l'agenda lu **plus** ce qui attend dans la file.
 *
 * La garde `mon-agenda` est réécrite aussitôt : un ajout fait dans un tunnel se
 * voit à la réouverture. Un `409` (session annulée entre-temps) abandonne
 * l'intention ; la file relit alors l'agenda vrai, et l'ajout se défait.
 *
 * Les réunions non annoncées suivent le même patron, sous leur propre préfixe
 * (`reseau-agenda-<id>`) ; retirée entre-temps, la réunion rend `404` et s'abandonne.
 */
import type { MyAgenda } from '~/types/negotiation-sessions'
import { estInchange } from '~/composables/api/etiquete'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import {
  appliquerIntention,
  appliquerIntentionReseau,
  avecLaFile,
  CLE_LECTURE_AGENDA,
  PREFIXE_FILE_AGENDA,
  PREFIXE_FILE_AGENDA_RESEAU,
  type IntentionAgenda,
} from '~/utils/guide-nego/agenda'
import { ecrireGarde, magasinDesEcritures } from '~/utils/guide-nego/garde'

interface AgendaLu {
  personne: string
  agenda: MyAgenda
  empreinte: string | null
}

const VIDE: MyAgenda = { entries: [], network_entries: [] }

export function useGnAgenda() {
  const api = useApi().negotiationSessions
  const session = useGnSession()
  const file = useGnFile()
  // Par personne : un choix de A encore dans la file ne s'affiche pas chez B.
  const enFileParPersonne = useState<Record<string, Record<string, IntentionAgenda>>>('gn-agenda-en-file', () => ({}))
  const enFile = computed(() => enFileParPersonne.value[session.compte.value.id ?? ''] ?? {})
  const reseauParPersonne = useState<Record<string, Record<string, IntentionAgenda>>>('gn-agenda-reseau-en-file', () => ({}))
  const enFileReseau = computed(() => reseauParPersonne.value[session.compte.value.id ?? ''] ?? {})

  const lecture = useGnLecture<AgendaLu>(CLE_LECTURE_AGENDA, async (garde) => {
    const personne = session.compte.value.id
    if (!personne) throw new Error('sans compte')
    const connue = garde?.personne === personne ? garde : null
    try {
      const lu = await api.monAgenda(connue?.empreinte ?? null)
      if (!estInchange(lu)) return { personne, agenda: lu.valeur, empreinte: lu.empreinte }
      if (connue) return { ...connue, empreinte: lu.empreinte }
      throw new Error('304 sans agenda gardé')
    } catch (erreur) {
      // L'API a parlé — session close ailleurs : ce n'est pas une panne de réseau.
      if (normalizeApiError(erreur) instanceof ApiRequestError) return { personne, agenda: VIDE, empreinte: null }
      throw erreur
    }
  })

  async function relireLaFile(): Promise<void> {
    const personne = session.compte.value.id
    if (!personne) return
    const intentions = (await magasinDesEcritures.lire().catch(() => [])).filter((i) => i.personne === personne)
    const par = (prefixe: string) =>
      Object.fromEntries(
        intentions.filter((i) => i.cle.startsWith(prefixe)).map((i) => [i.cle.slice(prefixe.length), i.corps as IntentionAgenda]),
      )
    enFileParPersonne.value = { ...enFileParPersonne.value, [personne]: par(PREFIXE_FILE_AGENDA) }
    reseauParPersonne.value = { ...reseauParPersonne.value, [personne]: par(PREFIXE_FILE_AGENDA_RESEAU) }
  }

  const lu = computed<AgendaLu | null>(() => {
    const valeur = lecture.etat.value.valeur
    return valeur && valeur.personne === session.compte.value.id ? valeur : null
  })

  const agenda = computed<MyAgenda>(() => avecLaFile(lu.value?.agenda ?? VIDE, enFile.value, new Date(), enFileReseau.value))

  function entree(sessionId: string) {
    return agenda.value.entries.find((e) => e.session_id === sessionId) ?? null
  }

  function entreeReseau(reunionId: string) {
    return agenda.value.network_entries.find((e) => e.network_meeting_id === reunionId) ?? null
  }

  file.inscrireFamille(
    PREFIXE_FILE_AGENDA,
    (intention) => {
      const id = intention.cle.slice(PREFIXE_FILE_AGENDA.length)
      const voulu = intention.corps as IntentionAgenda
      return voulu.garder ? api.garderUneSession(id, voulu.remind) : api.retirerUneSession(id)
    },
    async () => {
      await lecture.relire()
      await relireLaFile()
    },
  )
  file.inscrireFamille(
    PREFIXE_FILE_AGENDA_RESEAU,
    (intention) => {
      const id = intention.cle.slice(PREFIXE_FILE_AGENDA_RESEAU.length)
      const voulu = intention.corps as IntentionAgenda
      return voulu.garder ? api.garderUneReunion(id, voulu.remind) : api.retirerUneReunion(id)
    },
    async () => {
      await lecture.relire()
      await relireLaFile()
    },
  )

  // Sans compte, la lecture échouerait, et l'échec passerait pour une panne de réseau.
  function assurer(): void {
    if (!session.connectee.value) return
    void relireLaFile()
    void lecture.rafraichir()
  }

  /** Ce que la personne voit tout de suite, et ce qu'elle retrouve à la réouverture sans réseau. */
  async function reecrireLaGarde(personne: string, appliquer: (agenda: MyAgenda) => MyAgenda): Promise<void> {
    const etat = lecture.etat.value
    const avant = etat.valeur?.personne === personne ? etat.valeur : { personne, agenda: VIDE, empreinte: null }
    // Sans empreinte : l'agenda écrit ici n'est pas celui du serveur, un `304` ne doit pas le confirmer.
    const valeur: AgendaLu = { personne, agenda: appliquer(avant.agenda), empreinte: null }
    lecture.etat.value = { ...etat, valeur, pret: true }
    await ecrireGarde({ cle: CLE_LECTURE_AGENDA, valeur, lu_a: etat.luA ?? new Date().toISOString(), empreinte: null })
  }

  async function vouloir(sessionId: string, voulu: IntentionAgenda): Promise<void> {
    const personne = session.compte.value.id
    if (!personne) return
    enFileParPersonne.value = { ...enFileParPersonne.value, [personne]: { ...enFile.value, [sessionId]: voulu } }
    await reecrireLaGarde(personne, (a) => appliquerIntention(a, sessionId, voulu, new Date()))
    await file.poser(`${PREFIXE_FILE_AGENDA}${sessionId}`, voulu, null)
    await file.partir()
    await relireLaFile()
  }

  async function vouloirReunion(reunionId: string, voulu: IntentionAgenda): Promise<void> {
    const personne = session.compte.value.id
    if (!personne) return
    reseauParPersonne.value = { ...reseauParPersonne.value, [personne]: { ...enFileReseau.value, [reunionId]: voulu } }
    await reecrireLaGarde(personne, (a) => appliquerIntentionReseau(a, reunionId, voulu, new Date()))
    await file.poser(`${PREFIXE_FILE_AGENDA_RESEAU}${reunionId}`, voulu, null)
    await file.partir()
    await relireLaFile()
  }

  return {
    agenda,
    entree,
    dansLAgenda: (sessionId: string) => entree(sessionId) !== null,
    pret: computed(() => lecture.etat.value.pret),
    /** Lu au moins une fois pour cette personne : sinon, un agenda vide ne veut pas dire « rien ». */
    connu: computed(() => lu.value !== null),
    luA: computed(() => lecture.etat.value.luA),
    assurer,
    /** Ajoute, ou change le rappel ; l'ajout garde le rappel déjà choisi. */
    ajouter: (sessionId: string, remind = entree(sessionId)?.remind ?? false) =>
      vouloir(sessionId, { garder: true, remind }),
    rappeler: (sessionId: string, remind: boolean) => vouloir(sessionId, { garder: true, remind }),
    retirer: (sessionId: string) => vouloir(sessionId, { garder: false, remind: false }),
    entreeReseau,
    reunionDansLAgenda: (reunionId: string) => entreeReseau(reunionId) !== null,
    ajouterReunion: (reunionId: string, remind = entreeReseau(reunionId)?.remind ?? false) =>
      vouloirReunion(reunionId, { garder: true, remind }),
    rappelerReunion: (reunionId: string, remind: boolean) => vouloirReunion(reunionId, { garder: true, remind }),
    retirerReunion: (reunionId: string) => vouloirReunion(reunionId, { garder: false, remind: false }),
  }
}
