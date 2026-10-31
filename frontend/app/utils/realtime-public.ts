import type { RealtimeTopic } from '~/types/realtime'

/**
 * Les sujets des pages publiques d'une édition, servis par le pouls sans
 * session (`kernel::realtime::PublicEditionTopics`), et celui du poste de
 * direct. Émis par la base : `075_programme_sessions.sql` (`live`, `schedule`,
 * `questions`), `060_events.sql` (`schedule`, fils), `080_live.sql`
 * (`incidents`, `desk`).
 */
export type PublicEditionChannel = 'live' | 'schedule' | 'incidents' | 'questions'

export const publicEditionTopic = (eventId: string, channel: PublicEditionChannel): RealtimeTopic =>
  `edition:${eventId}:public:${channel}`

/** Les sujets de plusieurs éditions à la fois (l'accueil en montre plusieurs). */
export function publicEditionTopics(
  eventIds: Iterable<string | null | undefined>,
  channels: readonly PublicEditionChannel[],
): RealtimeTopic[] {
  const ids = [...new Set([...eventIds].filter((id): id is string => !!id))]
  return ids.flatMap((id) => channels.map((channel) => publicEditionTopic(id, channel)))
}

/** Le programme tel que le public le voit : séances, fils, repère « en direct ». */
export const PROGRAMME_CHANNELS = ['live', 'schedule'] as const satisfies readonly PublicEditionChannel[]

/** Poste de direct : privé, `live.meeting.manage` sur l'édition. */
export const deskTopic = (eventId: string): RealtimeTopic => `edition:${eventId}:desk`
