/**
 * `GET /sessions/{id}/streams` — les flux en cours d'une séance publiée.
 *
 * Même règle que l'API : une séance inconnue ou non publiée rend 404, une
 * séance publiée sans direct à l'instant rend une liste vide.
 */

import type { PublicSessionStream } from '~/types/live'
import { ApiRequestError } from '~/utils/api-error'
import { STREAM } from './ids'
import { allSessions } from './sessions'

export function publicSessionStreams(sessionId: string, at: number = Date.now()): PublicSessionStream[] {
  const session = allSessions.find((s) => s.id === sessionId && s.published_at !== null)
  if (!session) {
    throw new ApiRequestError({ code: 'NOT_FOUND', message: 'La ressource demandée est introuvable.' }, 404)
  }

  const enDirect =
    session.is_streamed &&
    session.status !== 'cancelled' &&
    session.status !== 'postponed' &&
    Date.parse(session.starts_at) <= at &&
    at <= Date.parse(session.ends_at)
  if (!enDirect) return []

  return [
    {
      id: STREAM.original,
      locale: 'fr',
      provider: 'youtube',
      embed_url: 'https://www.youtube.com/embed/ifdd-direct-fr',
      watch_url: 'https://www.youtube.com/watch?v=ifdd-direct-fr',
      is_primary: true,
      started_at: session.starts_at,
    },
    {
      id: STREAM.interpretationEn,
      locale: 'en',
      provider: 'youtube',
      embed_url: 'https://www.youtube.com/embed/ifdd-direct-en',
      watch_url: 'https://www.youtube.com/watch?v=ifdd-direct-en',
      is_primary: false,
      started_at: session.starts_at,
    },
  ]
}
