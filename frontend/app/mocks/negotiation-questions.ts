/**
 * Les questions aux experts, sans API : une réserve en mémoire que le téléphone
 * (« Mes questions ») et la file des experts partagent.
 */
import type { AdminQuestion } from '~/types/admin-negotiation-queue'
import type { MyQuestion, MyQuestionInput, MyQuestionList } from '~/types/negotiation-savoir'
import { ApiRequestError } from '~/utils/api-error'
import { resolveI18nText } from '~/utils/i18n-text'
import { empreinte } from './negotiation-documents'
import { taxonomyTerms } from './reference'

interface Etiquete<T> {
  valeur: T
  empreinte: string | null
}

function themeOuRefus(code: string): string {
  const terme = taxonomyTerms.find((t) => t.taxonomy_code === 'negotiation_theme' && t.code === code)
  if (!terme) throw new ApiRequestError({ code: 'NEGOTIATION_THEME_UNKNOWN', message: "Cette thématique n'existe pas.", field: 'theme_code' }, 400)
  return resolveI18nText(terme.label, 'fr')
}

let questions: MyQuestion[] = []

export function poserUneQuestion(entree: MyQuestionInput): MyQuestion {
  const deja = questions.find((q) => q.client_ref === entree.client_ref)
  if (deja) return structuredClone(deja)
  const theme_label = themeOuRefus(entree.theme_code)
  const body = entree.body.trim()
  if (!body) throw new ApiRequestError({ code: 'VALIDATION_FAILED', message: 'Écrivez votre question.', field: 'body' }, 422)
  if ([...body].length > 600) {
    throw new ApiRequestError({ code: 'NEGOTIATION_TEXT_TOO_LONG', message: 'Ce texte dépasse 600 caractères.', field: 'body' }, 422)
  }
  const q: MyQuestion = {
    id: crypto.randomUUID(),
    client_ref: entree.client_ref,
    theme_code: entree.theme_code,
    theme_label,
    body,
    consent_to_faq: entree.consent_to_faq,
    status: 'pending',
    answer: null,
    answered_by_name: null,
    answered_at: null,
    created_at: new Date().toISOString(),
  }
  questions = [q, ...questions]
  return structuredClone(q)
}

export function mesQuestions(): Etiquete<MyQuestionList> {
  return {
    valeur: { questions: structuredClone(questions) },
    empreinte: empreinte(questions.map((q) => [q.id, q.status, q.answered_at])),
  }
}

const versLaFile = ({ client_ref: _r, answered_by_name: _n, ...q }: MyQuestion): AdminQuestion => ({
  ...structuredClone(q),
  faq_entry_id: null,
})

export function questionsDeLaFile(): AdminQuestion[] {
  const depuis = Date.now() - 30 * 86_400_000
  const attente = questions.filter((q) => q.status === 'pending').sort((a, b) => a.created_at.localeCompare(b.created_at))
  const repondues = questions
    .filter((q) => q.status === 'answered' && Date.parse(q.answered_at ?? '') > depuis)
    .sort((a, b) => (b.answered_at ?? '').localeCompare(a.answered_at ?? ''))
  return [...attente, ...repondues].map(versLaFile)
}

export const questionsEnAttente = (): number => questions.filter((q) => q.status === 'pending').length

function questionOuRefus(id: string): MyQuestion {
  const q = questions.find((x) => x.id === id)
  if (!q) throw new ApiRequestError({ code: 'NOT_FOUND', message: "Cette question n'existe pas." }, 404)
  return q
}

const traitee = () =>
  new ApiRequestError({ code: 'NEGOTIATION_QUEUE_ITEM_CLOSED', message: 'Cet élément de la file a déjà été traité.' }, 409)

export function repondreAUneQuestion(id: string, answer: string): AdminQuestion {
  const q = questionOuRefus(id)
  if (q.status !== 'pending') throw traitee()
  if (!answer.trim()) throw new ApiRequestError({ code: 'VALIDATION_FAILED', message: 'Écrivez la réponse.', field: 'answer' }, 422)
  Object.assign(q, { status: 'answered', answer: answer.trim(), answered_by_name: 'Aminata Sow', answered_at: new Date().toISOString() })
  return versLaFile(q)
}

/** La question et sa réponse, pour le brouillon ; le brouillon lui-même naît dans la FAQ d'exemple. */
export function aPromouvoir(id: string): { body: string; answer: string } {
  const q = questionOuRefus(id)
  if (q.status === 'pending') {
    throw new ApiRequestError(
      { code: 'VALIDATION_FAILED', message: "Répondez d'abord à la question : sa réponse fait celle de l'entrée.", field: 'answer' },
      422,
    )
  }
  if (q.status !== 'answered' || !q.answer) throw traitee()
  if (!q.consent_to_faq) {
    throw new ApiRequestError(
      { code: 'NEGOTIATION_QUESTION_NO_CONSENT', message: "La personne qui a posé cette question n'a pas accepté qu'elle rejoigne la FAQ." },
      422,
    )
  }
  return { body: q.body, answer: q.answer }
}

export function marquerPromue(id: string): void {
  questionOuRefus(id).status = 'added_to_faq'
}
