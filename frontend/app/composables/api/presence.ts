/**
 * Présence : le signal de chaque onglet (`usePresence()` seulement) et la
 * lecture de la carte « En ce moment » du tableau de bord.
 *
 * **Le signal passe hors de `request()`**, comme le pouls du temps réel : une
 * présence perdue ne s'affiche jamais, alors que `request()` annoncerait une
 * API injoignable au bandeau de panne. Un échec rend `null`, le signal suivant
 * rattrape.
 */
import type { AdminPresence, PresenceAck, PresenceBeat } from '~/types/admin-presence'
import type { IssueDeRotation } from '~/utils/rotation'
import type { ApiHttp } from './http'
import type { ApiTransport } from './proposal-review'

export interface PresenceApi {
  isConfigured: () => boolean
  beat: (body: PresenceBeat) => Promise<PresenceAck | null>
  rotation: () => Promise<IssueDeRotation>
}

export function createPresenceApi(http: Pick<ApiHttp, 'isConfigured' | 'client' | 'rotation'>): PresenceApi {
  return {
    isConfigured: () => http.isConfigured.value,

    async beat(body) {
      try {
        const response = await http.client.raw<PresenceAck>('/presence', {
          method: 'POST',
          body,
          retry: 0,
          ignoreResponseError: true,
        })
        return response.status === 200 && response._data ? response._data : null
      } catch {
        return null
      }
    },

    rotation: () => http.rotation(),
  }
}

/** `analytics.presence.read` en portée globale ; sans elle, la carte n'est pas affichée. */
export function createAdminPresenceApi({ call }: Pick<ApiTransport, 'call'>) {
  return {
    read: (): Promise<AdminPresence> => call('/admin/presence', (m) => m.adminPresence()),
  }
}
