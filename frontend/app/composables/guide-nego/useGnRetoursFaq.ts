/**
 * « Cette réponse vous a-t-elle aidée ? » et « Dépassé ou faux » (R8). Réservés à la
 * personne connectée ; chaque geste part par la file de 0c, sans empreinte, et l'écran
 * montre la voix lue du serveur ou celle qui attend dans la file. Un signalement porte
 * une référence choisie ici : rejoué, il ne compte qu'une fois.
 */
import type { FaqFeedbackInput, FaqMissingReason, FaqReportInput, FaqReportReason } from '~/types/negotiation-savoir'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { magasinDesEcritures } from '~/utils/guide-nego/garde'

const RETOUR = 'savoir.retour.'
const SIGNALEMENT = 'savoir.signalement.'

interface VoixLues {
  personne: string
  voix: Record<string, FaqFeedbackInput>
}

interface SignalementEnFile {
  entry_id: string
  entree: FaqReportInput
}

export function useGnRetoursFaq() {
  const api = useApi().guideNegoSavoir
  const session = useGnSession()
  const file = useGnFile()

  const enFileParPersonne = useState<Record<string, Record<string, FaqFeedbackInput>>>('gn-faq-retours-en-file', () => ({}))
  const personne = computed(() => (session.connectee.value ? session.compte.value.id : null))

  const lecture = useGnLecture<VoixLues>('mes-retours-faq', async () => {
    const qui = personne.value
    if (!qui) throw new Error('sans compte')
    try {
      const lu = await api.mesRetoursSurLaFaq()
      return {
        personne: qui,
        voix: Object.fromEntries(
          lu.valeur.feedback.map((v) => [v.entry_id, { helpful: v.helpful, missing_reason: v.missing_reason }]),
        ),
      }
    } catch (erreur) {
      if (normalizeApiError(erreur) instanceof ApiRequestError) return { personne: qui, voix: {} }
      throw erreur
    }
  })

  async function relireLaFile(): Promise<void> {
    const qui = personne.value
    if (!qui) return
    const intentions = await magasinDesEcritures.lire().catch(() => [])
    enFileParPersonne.value = {
      ...enFileParPersonne.value,
      [qui]: Object.fromEntries(
        intentions
          .filter((i) => i.personne === qui && i.cle.startsWith(RETOUR))
          .map((i) => [i.cle.slice(RETOUR.length), i.corps as FaqFeedbackInput]),
      ),
    }
  }

  file.inscrireFamille(
    RETOUR,
    (intention) => api.voterSurUneEntree(intention.cle.slice(RETOUR.length), intention.corps as FaqFeedbackInput),
    async () => {
      await lecture.relire()
      await relireLaFile()
    },
  )
  file.inscrireFamille(SIGNALEMENT, (intention) => {
    const { entry_id, entree } = intention.corps as SignalementEnFile
    return api.signalerUneEntree(entry_id, entree)
  })

  /** La voix de la personne sur l'entrée : celle qui attend, sinon celle du serveur. */
  function voixSur(id: string): FaqFeedbackInput | null {
    const qui = personne.value
    if (!qui) return null
    const lues = lecture.etat.value.valeur
    return enFileParPersonne.value[qui]?.[id] ?? (lues?.personne === qui ? lues.voix[id] : undefined) ?? null
  }

  async function assurer(): Promise<void> {
    await session.assurer()
    if (!personne.value) return
    await relireLaFile()
    await lecture.rafraichir()
  }

  async function voter(id: string, helpful: boolean, motif: FaqMissingReason | null = null): Promise<void> {
    const qui = personne.value
    if (!qui) return
    const corps: FaqFeedbackInput = { helpful, missing_reason: helpful ? null : motif }
    enFileParPersonne.value = {
      ...enFileParPersonne.value,
      [qui]: { ...enFileParPersonne.value[qui], [id]: corps },
    }
    await file.poser(`${RETOUR}${id}`, corps, null)
    await file.partir()
    await relireLaFile()
  }

  async function signaler(id: string, reasons: FaqReportReason[], details: string): Promise<void> {
    if (!personne.value) return
    const client_ref = crypto.randomUUID()
    const precision = details.trim()
    const corps: SignalementEnFile = {
      entry_id: id,
      entree: { client_ref, reasons, details: precision || null },
    }
    await file.poser(`${SIGNALEMENT}${client_ref}`, corps, null)
    await file.partir()
  }

  return { connectee: computed(() => personne.value !== null), voixSur, assurer, voter, signaler }
}
