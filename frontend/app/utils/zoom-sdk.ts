/**
 * Chargement du Meeting SDK de Zoom, dans le navigateur seulement.
 *
 * La vue client (`ZoomMtg`), pleine page, celle du logiciel : la vue intégrée a
 * été retirée le 17/10. Le paquet est un UMD CommonJS de plusieurs mégaoctets :
 * importé à la demande, et l'export se cherche sous `default` ou à plat selon
 * l'interopérabilité de Vite.
 */
import type { MeetingAccess } from '~/types/live'

type ClientModule = typeof import('@zoom/meetingsdk')
export type ZoomClientView = ClientModule['ZoomMtg']

export type ZoomLanguage = 'fr-FR' | 'en-US'

export const zoomLanguage = (locale: string): ZoomLanguage => (locale === 'en' ? 'en-US' : 'fr-FR')

export async function loadZoomClientView(): Promise<ZoomClientView> {
  const mod = (await import('@zoom/meetingsdk')) as Partial<ClientModule> & { default?: Partial<ClientModule> }
  const zoom = mod.ZoomMtg ?? mod.default?.ZoomMtg
  if (!zoom) throw new Error('ZoomMtg introuvable')
  return zoom
}

/** Les avis du SDK quand l'image ou le son n'ont pas pu se connecter (écran noir) : il n'émet aucun événement pour ce cas. */
const MEDIA_FAILURE_KEYS = [
  'apac.audio.wcl_video_bridge_websocket_connect_error_text',
  'apac.audio.wcl_audio_and_video_bridge_websocket_connect_error_text',
  'apac.audio.wcl_audio_bridge_websocket_connect_error_text',
]

export function zoomMediaFailureTexts(zoom: ZoomClientView, language: ZoomLanguage): string[] {
  const all = (zoom.i18n.getAll(language) ?? {}) as Record<string, unknown>
  return MEDIA_FAILURE_KEYS.map((key) => all[key]).filter((text): text is string => typeof text === 'string' && text.length > 0)
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
