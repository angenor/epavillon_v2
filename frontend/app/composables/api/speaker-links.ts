/**
 * Le lien d'inscription des intervenants en ligne : sa part de `useApi()`, sans
 * compte. Le jeton part dans l'en-tête `X-Speaker-Link` (`speakerLinkHeaders`).
 */
import type { OnlineSpeakerCreated, OnlineSpeakerInput, SpeakerLinkInvite, SpeakerZoomAccess } from '~/types/programme/online-speaker'
import type { Uuid } from '~/types/shared'
import { speakerLinkHeaders } from '~/utils/speaker-links'
import type { ApiTransport } from './proposal-review'

export function createSpeakerLinksApi({ call, callOrNull, send }: Pick<ApiTransport, 'call' | 'callOrNull' | 'send'>) {
  return {
    /** Nulle : jeton inconnu, ou activité terminée. */
    invite: (token: string): Promise<SpeakerLinkInvite | null> =>
      callOrNull('/speaker-links', (m) => m.speakerLinkInvite(token), undefined, speakerLinkHeaders(token)),

    register: (token: string, input: OnlineSpeakerInput): Promise<OnlineSpeakerCreated> =>
      send('/speaker-links/speakers', input, (m) => m.speakerLinkRegister(token, input), 'POST', speakerLinkHeaders(token)),

    /** 409 `LIVE_MEETING_NOT_READY` (réessayer), 503 `LIVE_ZOOM_UNAVAILABLE`. */
    zoom: (token: string, speakerId: Uuid): Promise<SpeakerZoomAccess> =>
      send('/speaker-links/zoom', { speaker_id: speakerId }, (m) => m.speakerLinkZoom(token, speakerId), 'POST', speakerLinkHeaders(token)),
  }
}
