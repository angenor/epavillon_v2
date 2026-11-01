/**
 * L'ASSISTANCE DE CHAQUE ÉCRAN (02/11) : sa part de `useApi()`.
 *
 * Un sujet sur un dossier s'écrit par les routes du dossier (`workspace.reply`
 * côté organisation, `review.comment` côté équipe) ; ici, la liste de tout,
 * la lecture, et les sujets sur la plateforme.
 */
import type {
  AssistanceFeed,
  AssistanceStatus,
  MarkAssistanceReadPayload,
  OpenSupportSubjectPayload,
  ReplySupportSubjectPayload,
} from '~/types/programme/assistance'
import type { ProposalComment } from '~/types/programme/proposal'
import type { PersonId, Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

const exemples = () => import('~/mocks/assistance')

export function createAssistanceApi({ call, send }: Pick<ApiTransport, 'call' | 'send'>) {
  return {
    status: (personId: PersonId) =>
      call<AssistanceStatus>('/assistance', async () => (await exemples()).assistanceStatus(personId)),

    feed: (personId: PersonId) =>
      call<AssistanceFeed>('/assistance/subjects', async () => (await exemples()).assistanceFeed(personId)),

    open: (personId: PersonId, payload: OpenSupportSubjectPayload) =>
      send<ProposalComment>('/assistance/subjects', payload, async () =>
        (await exemples()).openSupportSubject(personId, payload),
      ),

    reply: (personId: PersonId, rootId: Uuid, payload: ReplySupportSubjectPayload) =>
      send<ProposalComment>(`/assistance/subjects/${rootId}/replies`, payload, async () =>
        (await exemples()).replySupportSubject(personId, rootId, payload),
      ),

    markRead: (personId: PersonId, commentIds: Uuid[]) => {
      const payload: MarkAssistanceReadPayload = { comment_ids: commentIds }
      return send<void>('/assistance/read', payload, async () => (await exemples()).markAssistanceRead(personId, payload))
    },
  }
}
