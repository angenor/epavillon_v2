/**
 * Chargement du Meeting SDK de Zoom, dans le navigateur seulement.
 *
 * Deux vues : `ZoomMtgEmbedded` (dans la page, prévue pour l'ordinateur) et
 * `ZoomMtg` (pleine page, celle que Zoom recommande sur mobile). Les paquets
 * sont des UMD CommonJS de plusieurs mégaoctets : importés à la demande, et
 * l'export se cherche sous `default` ou à plat selon l'interopérabilité de Vite.
 */
import type { MeetingAccess } from '~/types/live'

type EmbeddedModule = typeof import('@zoom/meetingsdk/embedded')
type ClientModule = typeof import('@zoom/meetingsdk')
export type ZoomEmbedded = EmbeddedModule['default']
export type ZoomEmbeddedClient = ReturnType<ZoomEmbedded['createClient']>
export type ZoomClientView = ClientModule['ZoomMtg']

export type ZoomLanguage = 'fr-FR' | 'en-US'

export const zoomLanguage = (locale: string): ZoomLanguage => (locale === 'en' ? 'en-US' : 'fr-FR')

export async function loadZoomEmbedded(): Promise<ZoomEmbedded> {
  const mod = (await import('@zoom/meetingsdk/embedded')) as { default: Partial<ZoomEmbedded> & { default?: ZoomEmbedded } }
  const candidate = mod.default
  const zoom = typeof candidate.createClient === 'function' ? (candidate as ZoomEmbedded) : candidate.default
  if (!zoom) throw new Error('ZoomMtgEmbedded introuvable')
  return zoom
}

export async function loadZoomClientView(): Promise<ZoomClientView> {
  const mod = (await import('@zoom/meetingsdk')) as Partial<ClientModule> & { default?: Partial<ClientModule> }
  const zoom = mod.ZoomMtg ?? mod.default?.ZoomMtg
  if (!zoom) throw new Error('ZoomMtg introuvable')
  return zoom
}

/** Champs communs aux deux vues ; le jeton d'inscrit n'est passé que s'il existe. */
export function zoomJoinFields(access: MeetingAccess) {
  return {
    signature: access.signature,
    meetingNumber: access.meeting_number,
    userName: access.user_name,
    userEmail: access.user_email,
    ...(access.registrant_token ? { tk: access.registrant_token } : {}),
  }
}

/** Le SDK rejette des objets `{ reason, errorCode }` plutôt que des `Error`. */
export function zoomFailureReason(error: unknown): string | null {
  if (typeof error === 'string') return error
  if (typeof error !== 'object' || error === null) return null
  const candidate = error as { reason?: unknown; errorMessage?: unknown; message?: unknown }
  for (const value of [candidate.reason, candidate.errorMessage, candidate.message]) {
    if (typeof value === 'string' && value.trim()) return value
  }
  return null
}
