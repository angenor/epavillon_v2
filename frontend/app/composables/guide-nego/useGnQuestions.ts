/**
 * Les questions aux experts (récit 6). Réservées à l'accès négociateur : l'écran pose
 * le verrou, l'API refuse de toute façon.
 *
 * Une question part par la file de 0c, sous une référence choisie ici : posée sans
 * réseau, elle part au retour, une fois (R8). « Mes questions » est la lecture
 * gardée, **plus celles qui attendent dans la file** ; elle quitte le téléphone à la
 * déconnexion (`useGnSession`), et la garde porte la personne pour qu'un téléphone
 * partagé ne montre pas les questions d'une autre.
 */
import type { MyQuestion, MyQuestionInput } from '~/types/negotiation-savoir'
import { ApiRequestError, ForbiddenError, normalizeApiError } from '~/utils/api-error'
import { CLE_LECTURE_QUESTIONS, magasinDesEcritures } from '~/utils/guide-nego/garde'

const PREFIXE = 'savoir.question.'

interface QuestionsLues {
  personne: string
  questions: MyQuestion[]
}

/** Une question encore dans la file : ce que la personne a écrit, et quand. */
export interface QuestionEnFile {
  entree: MyQuestionInput
  prise_a: string
}

export function useGnQuestions() {
  const api = useApi().guideNegoSavoir
  const session = useGnSession()
  const file = useGnFile()

  const enFileParPersonne = useState<Record<string, QuestionEnFile[]>>('gn-questions-en-file', () => ({}))
  const personne = computed(() => (session.connectee.value ? session.compte.value.id : null))

  const lecture = useGnLecture<QuestionsLues>(CLE_LECTURE_QUESTIONS, async () => {
    const qui = personne.value
    if (!qui) throw new Error('sans compte')
    try {
      return { personne: qui, questions: (await api.mesQuestions()).valeur.questions }
    } catch (erreur) {
      // Sans l'accès, l'API a parlé : rien à montrer, et ce n'est pas une panne.
      if (erreur instanceof ForbiddenError || normalizeApiError(erreur) instanceof ApiRequestError) {
        return { personne: qui, questions: [] }
      }
      throw erreur
    }
  })

  async function relireLaFile(): Promise<void> {
    const qui = personne.value
    if (!qui) return
    const intentions = await magasinDesEcritures.lire().catch(() => [])
    enFileParPersonne.value = {
      ...enFileParPersonne.value,
      [qui]: intentions
        .filter((i) => i.personne === qui && i.cle.startsWith(PREFIXE))
        .map((i) => ({ entree: i.corps as MyQuestionInput, prise_a: i.prise_a })),
    }
  }

  file.inscrireFamille(
    PREFIXE,
    (intention) => api.poserUneQuestion(intention.corps as MyQuestionInput),
    async () => {
      await lecture.relire()
      await relireLaFile()
    },
  )

  const lues = computed(() => {
    const l = lecture.etat.value.valeur
    return l && l.personne === personne.value ? l.questions : null
  })

  /** Celles qui attendent le réseau, sauf si la lecture les porte déjà. */
  const enFile = computed<QuestionEnFile[]>(() => {
    const qui = personne.value
    if (!qui) return []
    const parties = new Set((lues.value ?? []).map((q) => q.client_ref))
    return (enFileParPersonne.value[qui] ?? []).filter((q) => !parties.has(q.entree.client_ref))
  })

  function assurer(): void {
    if (!personne.value) return
    void relireLaFile()
    void lecture.rafraichir()
  }

  /** Rend la référence de la question : l'écran « Envoyé » la retrouve par elle. */
  async function poser(theme_code: string, body: string, consent_to_faq: boolean): Promise<QuestionEnFile | null> {
    if (!personne.value) return null
    const entree: MyQuestionInput = { client_ref: crypto.randomUUID(), theme_code, body: body.trim(), consent_to_faq }
    const enAttente: QuestionEnFile = { entree, prise_a: new Date().toISOString() }
    await file.poser(`${PREFIXE}${entree.client_ref}`, entree, null)
    await relireLaFile()
    await file.partir()
    await relireLaFile()
    return enAttente
  }

  /** Partie ? Alors la lecture la porte ; sinon elle attend encore dans la file. */
  function partie(client_ref: string): MyQuestion | null {
    return lues.value?.find((q) => q.client_ref === client_ref) ?? null
  }

  return {
    questions: computed(() => lues.value ?? []),
    enFile,
    /** Lues au moins une fois pour cette personne : sinon, une liste vide ne veut pas dire « aucune ». */
    connues: computed(() => lues.value !== null),
    pret: computed(() => lecture.etat.value.pret),
    luA: computed(() => lecture.etat.value.luA),
    assurer,
    poser,
    partie,
    relire: lecture.relire,
  }
}
