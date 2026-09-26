/**
 * Signaler, et « Mes signalements » (FR-006 à FR-008).
 *
 * **Un signalement passe toujours par la file**, clé `signalement:<client_ref>` : il
 * paraît aussitôt dans « Mes signalements », en attente d'envoi, et part au retour du
 * réseau. Le `client_ref` est posé une fois, à la saisie : un envoi rejoué rend `200`
 * et le même signalement. Un `409` (doublon) abandonne l'intention ; la file relit
 * alors la liste vraie, et son message passe par l'avis de file.
 */
import type { MyReport, ReportPayload } from '~/types/negotiation-reports'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { magasinDesEcritures } from '~/utils/guide-nego/garde'
import {
  CLE_LECTURE_MES_SIGNALEMENTS,
  PREFIXE_FILE_SIGNALEMENT,
  lignesDesSignalements,
  referenceClient,
  signalementEnAttente,
  signalementEnCours,
  type LigneSignalement,
  type SignalementEnFile,
} from '~/utils/guide-nego/signalements'

interface SignalementsLus {
  personne: string
  slug: string
  signalements: MyReport[]
}

export type Saisie = Omit<ReportPayload, 'client_ref' | 'edition'>

export interface IssueDuSignalement {
  client_ref: string
  /** `en-attente` : pas de réseau, il partira à son retour. */
  issue: 'envoye' | 'en-attente' | 'refuse'
  /** Le message de l'API, tel quel, quand elle a refusé. */
  message: string | null
}

export function useGnSignalements() {
  const api = useApi().negotiationReports
  const session = useGnSession()
  const edition = useGnEdition()
  const sessions = useGnSessions()
  const file = useGnFile()
  const enFileParPersonne = useState<Record<string, SignalementEnFile[]>>('gn-signalements-en-file', () => ({}))

  const lecture = useGnLecture<SignalementsLus>(CLE_LECTURE_MES_SIGNALEMENTS, async () => {
    const personne = session.compte.value.id
    if (!personne) throw new Error('sans compte')
    const lue = await edition.lue()
    if (!lue) throw new Error('aucune édition')
    try {
      return { personne, slug: lue.slug, signalements: (await api.mesSignalements(lue.slug)).reports }
    } catch (erreur) {
      // L'API a parlé — sans accès, rien à montrer : ce n'est pas une panne de réseau.
      if (normalizeApiError(erreur) instanceof ApiRequestError) return { personne, slug: lue.slug, signalements: [] }
      throw erreur
    }
  })

  const lu = computed<SignalementsLus | null>(() => {
    const valeur = lecture.etat.value.valeur
    return valeur && valeur.personne === session.compte.value.id ? valeur : null
  })

  const enAttente = computed<MyReport[]>(() =>
    (enFileParPersonne.value[session.compte.value.id ?? ''] ?? []).map((e) => {
      const s = e.corps.session_id ? sessions.session(e.corps.session_id) : null
      return signalementEnAttente(
        e,
        s ? { id: s.id, title_en: s.title_en, title_fr: s.title_fr, start_at: s.start_at, venue: s.venue } : null,
      )
    }),
  )

  const lignes = computed<LigneSignalement[]>(() => lignesDesSignalements(lu.value?.signalements ?? [], enAttente.value))

  async function relireLaFile(): Promise<void> {
    const personne = session.compte.value.id
    if (!personne) return
    const intentions = await magasinDesEcritures.lire().catch(() => [])
    enFileParPersonne.value = {
      ...enFileParPersonne.value,
      [personne]: intentions
        .filter((i) => i.personne === personne && i.cle.startsWith(PREFIXE_FILE_SIGNALEMENT))
        .map((i) => ({ corps: i.corps as ReportPayload, prise_a: i.prise_a })),
    }
  }

  file.inscrireFamille(
    PREFIXE_FILE_SIGNALEMENT,
    (intention) => api.signaler(intention.corps as ReportPayload),
    async () => {
      await lecture.relire()
      await relireLaFile()
    },
  )

  function assurer(): void {
    if (!session.connectee.value) return
    void relireLaFile()
    void lecture.rafraichir()
  }

  async function signaler(saisie: Saisie): Promise<IssueDuSignalement> {
    const personne = session.compte.value.id
    const slug = (await edition.gardee())?.slug ?? null
    const client_ref = referenceClient()
    if (!personne || !slug) return { client_ref, issue: 'refuse', message: null }

    const corps: ReportPayload = { ...saisie, client_ref, edition: slug }
    const cle = `${PREFIXE_FILE_SIGNALEMENT}${client_ref}`
    const prise_a = new Date().toISOString()
    enFileParPersonne.value = {
      ...enFileParPersonne.value,
      [personne]: [...(enFileParPersonne.value[personne] ?? []), { corps, prise_a }],
    }
    await file.poser(cle, corps, null)
    const suite = (await file.partir()).find((s) => s.cle === cle)
    await relireLaFile()
    if (suite?.sort === 'envoyee') return { client_ref, issue: 'envoye', message: null }
    if (suite?.sort === 'refusee' || suite?.sort === 'perimee') return { client_ref, issue: 'refuse', message: suite.message }
    return { client_ref, issue: 'en-attente', message: null }
  }

  return {
    lignes,
    /** FR-008 : le signalement non tranché de la personne sur cette session, s'il y en a un. */
    enCoursSur: (sessionId: string) => signalementEnCours(sessionId, lignes.value),
    pret: computed(() => lecture.etat.value.pret),
    /** Lu au moins une fois pour cette personne : sinon, une liste vide ne veut pas dire « rien ». */
    connu: computed(() => lu.value !== null),
    luA: computed(() => lecture.etat.value.luA),
    /** Le refus d'un envoi parti de la file, avec le message de l'API. */
    refus: computed(() => {
      const avis = file.avis.value
      return avis?.cle.startsWith(PREFIXE_FILE_SIGNALEMENT) ? avis : null
    }),
    assurer,
    rafraichir: lecture.rafraichir,
    signaler,
  }
}
