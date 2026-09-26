/**
 * Les signalements du réseau et les réglages de notification de Guide Négo — leur part
 * de `useApi()`. Contrat : `specs/015-guide-nego-signalements/contracts/api-signalements.md`.
 *
 * La file de validation vit sous `/admin` comme tout le back-office de Guide Négo, même
 * si l'écran est dans l'application ; elle n'entre jamais dans la file d'écritures (R4).
 */
import type { MyThemes, NotificationSettings } from '~/types/negotiation'
import type { MyReport, MyReports, ReportPayload, ReportQueue, ReportQueueItem, RejectPayload } from '~/types/negotiation-reports'
import type { Uuid } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'
import type { Primitives } from './guide-nego'

type Deps = Pick<Primitives, 'call' | 'send'>

const exemples = () => import('~/mocks/negotiation-reports')

const edition = (slug: string) => `?edition=${encodeURIComponent(slug)}`

export function createNegotiationReportsApi({ call, send }: Deps) {
  return {
    /** Rejoué avec le même `client_ref` : `200` et le même signalement. Doublon : `409 NEGOTIATION_REPORT_DUPLICATE`. */
    signaler: (payload: ReportPayload): Promise<MyReport> =>
      send('/negotiation/reports', payload, async () => (await exemples()).signaler(payload)),

    mesSignalements: (slug: string): Promise<MyReports> =>
      call('/negotiation/me/reports' + edition(slug), async () => (await exemples()).mesSignalements()),

    fileDeValidation: (slug: string): Promise<ReportQueue> =>
      call('/admin/negotiation/reports' + edition(slug), async () => (await exemples()).fileDeValidation()),

    /** Rien ne paraît avant la publication, trente secondes plus tard. */
    valider: (id: Uuid): Promise<ReportQueueItem> =>
      send(`/admin/negotiation/reports/${id}/validate`, {}, async () => (await exemples()).valider(id)),

    /** Tant que rien n'est publié ; sinon `409 NEGOTIATION_REPORT_UNDO_EXPIRED`. */
    annuler: (id: Uuid): Promise<ReportQueueItem> =>
      send(`/admin/negotiation/reports/${id}/undo`, {}, async () => (await exemples()).annuler(id)),

    refuser: (id: Uuid, corps: RejectPayload): Promise<ReportQueueItem> =>
      send(`/admin/negotiation/reports/${id}/reject`, corps, async () => (await exemples()).refuser(id, corps)),

    retirer: (id: Uuid): Promise<ReportQueueItem> =>
      send(`/admin/negotiation/reports/${id}/withdraw`, {}, async () => (await exemples()).retirer(id)),

    /** Sans accord enregistré : `email: true`. */
    reglageDesNotifications: (): Promise<NotificationSettings> =>
      call('/negotiation/me/notifications', async () => (await exemples()).reglageDesNotifications()),

    reglerLesNotifications: (email: boolean): Promise<NotificationSettings> =>
      send('/negotiation/me/notifications', { email }, async () => (await exemples()).reglerLesNotifications(email), 'PUT'),

    /** La liste entière, parmi les thématiques suivies ; sinon `400 NEGOTIATION_THEME_UNKNOWN`. */
    notifierDesThematiques: (codes: string[]): Promise<MyThemes> =>
      send('/negotiation/me/themes/notifications', { codes }, async () => {
        const lu = (await import('~/mocks/negotiation-themes')).notifierDesThematiques(codes)
        if (!lu) throw new ApiRequestError({ code: 'NEGOTIATION_THEME_UNKNOWN', message: "Cette thématique n'existe pas." }, 400)
        return lu
      }, 'PUT'),
  }
}
