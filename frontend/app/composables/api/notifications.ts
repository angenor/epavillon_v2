/**
 * Le fil de notifications d'`engagement`, filtré par module d'origine : la cloche de
 * Guide Négo ne compte pas les avis du site (R12).
 */
import type { NotificationFeed } from '~/types/engagement'
import type { Uuid } from '~/types/shared'
import type { Primitives } from './guide-nego'

type Deps = Pick<Primitives, 'call' | 'send'>

const exemples = () => import('~/mocks/notifications')

export function createNotificationsApi({ call, send }: Deps) {
  return {
    /** Les 50 dernières de Guide Négo, et le compte de toutes ses non lues. */
    filGuideNego: (): Promise<NotificationFeed> =>
      call('/notifications?module=negotiation&limit=50', async () => (await exemples()).filDeNotifications(50)),

    /**
     * **Toujours des identifiants explicites** : sans liste, l'API marque lues toutes
     * les notifications de la personne, celles du site comprises.
     */
    marquerLues: async (ids: Uuid[]): Promise<void> => {
      if (ids.length === 0) return
      await send('/notifications/read', { ids }, async () => (await exemples()).marquerLues(ids))
    },
  }
}
