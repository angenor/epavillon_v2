/**
 * Les deux routes du temps réel, pour `useRealtime()` seulement.
 *
 * **Hors de `request()`, et c'est voulu** : une mise à jour coupée ne se montre
 * jamais (décision du 31/10). `request()` signalerait une API injoignable au
 * bandeau de panne ; ici, un échec rend simplement `failed` et la tentative
 * suivante rattrape. La rotation du jeton reste celle du client (une seule par
 * onglet).
 */
import type { RealtimePulse, RealtimeTopic } from '~/types/realtime'
import type { IssueDeRotation } from '~/utils/rotation'
import type { ApiHttp } from './http'

export type PulseReading =
  | { kind: 'fresh'; pulse: RealtimePulse; etag: string | null }
  | { kind: 'unchanged' }
  | { kind: 'unauthenticated' }
  | { kind: 'failed' }

export interface RealtimeApi {
  isConfigured: () => boolean
  pulse: (topics: RealtimeTopic[], etag: string | null) => Promise<PulseReading>
  /** La réponse brute du flux, à lire en continu ; lève sur une panne réseau. */
  stream: (topics: RealtimeTopic[], signal: AbortSignal) => Promise<Response>
  rotation: () => Promise<IssueDeRotation>
}

export function createRealtimeApi(http: Pick<ApiHttp, 'baseURL' | 'isConfigured' | 'client' | 'rotation'>): RealtimeApi {
  return {
    isConfigured: () => http.isConfigured.value,

    async pulse(topics, etag) {
      try {
        const response = await http.client.raw<RealtimePulse>('/realtime/pulse', {
          query: { topics: topics.join(',') },
          headers: etag ? { 'If-None-Match': etag } : {},
          retry: 0,
          ignoreResponseError: true,
        })
        if (response.status === 304) return { kind: 'unchanged' }
        if (response.status === 401) return { kind: 'unauthenticated' }
        if (response.status !== 200 || !response._data) return { kind: 'failed' }
        return { kind: 'fresh', pulse: response._data, etag: response.headers.get('etag') }
      } catch {
        return { kind: 'failed' }
      }
    },

    stream(topics, signal) {
      const query = encodeURIComponent(topics.join(','))
      return fetch(`${http.baseURL.replace(/\/$/, '')}/realtime/stream?topics=${query}`, {
        credentials: 'include',
        cache: 'no-store',
        headers: { Accept: 'text/event-stream' },
        signal,
      })
    },

    rotation: () => http.rotation(),
  }
}
