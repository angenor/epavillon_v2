/**
 * ENVOIS DE COURRIELS DU BACK-OFFICE (28/10) : sa part de `useApi()`.
 *
 * Le périmètre est vérifié avant l'appel quand l'édition est connue : une
 * édition hors périmètre refuse l'accès plutôt que de rendre une liste vide.
 * Un envoi visé par son identifiant, lui, répond 404 hors droit (l'API ne dit
 * pas qu'il existe) : `callOrNull`.
 *
 * Les écritures passent par `send`, jamais rejouée : un envoi rejoué après une
 * coupure partirait deux fois.
 */

import type {
  CreateMailingPayload,
  Mailing,
  MailingAudience,
  MailingListScreen,
  MailingPayload,
  MailingPreview,
  MailingTestResult,
  MailTemplate,
  MailTemplatePayload,
  MailTemplatePreview,
} from '~/types/admin-mailings'
import type { AdministeredEvents } from '~/types/identity'
import type { EventId, Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

interface MailingsApiDeps extends Pick<ApiTransport, 'call' | 'callOrNull' | 'send'> {
  assertEventInScope: (eventId: Uuid, scope: AdministeredEvents) => void
}

export function createAdminMailingsApi({ call, callOrNull, send, assertEventInScope }: MailingsApiDeps) {
  return {
    /** L'historique de l'édition, la cadence et la copie de l'équipe. */
    list: (eventId: EventId, scope: AdministeredEvents): Promise<MailingListScreen> => {
      assertEventInScope(eventId, scope)
      return call('/admin/mailings', (m) => m.mailingListScreen(eventId), { event_id: eventId })
    },

    /** Un envoi. `null` : inexistant ou hors droit. */
    byId: (mailingId: Uuid): Promise<Mailing | null> =>
      callOrNull(`/admin/mailings/${mailingId}`, (m) => m.mailingById(mailingId)),

    /** La liste telle qu'elle partirait maintenant, avec le décompte. */
    audience: (mailingId: Uuid): Promise<MailingAudience | null> =>
      callOrNull(`/admin/mailings/${mailingId}/audience`, (m) => m.mailingAudience(mailingId)),

    /** Le courriel tel que ce destinataire le recevra, gabarit compris. */
    preview: (mailingId: Uuid, email?: string): Promise<MailingPreview | null> =>
      callOrNull(`/admin/mailings/${mailingId}/preview`, (m) => m.mailingPreview(mailingId, email), {
        email,
      }),

    /** Un brouillon ; sans copie précisée, l'API propose celle de l'équipe. */
    create: (payload: CreateMailingPayload): Promise<Mailing> =>
      send('/admin/mailings', payload, (m) => m.createMailing(payload), 'POST'),

    update: (mailingId: Uuid, payload: MailingPayload): Promise<Mailing> =>
      send(`/admin/mailings/${mailingId}`, payload, (m) => m.updateMailing(mailingId, payload), 'PUT'),

    remove: (mailingId: Uuid): Promise<{ deleted: boolean }> =>
      send(`/admin/mailings/${mailingId}`, {}, (m) => m.deleteMailing(mailingId), 'DELETE'),

    /** L'essai part à la personne connectée, par la file. */
    test: (mailingId: Uuid): Promise<MailingTestResult> =>
      send(`/admin/mailings/${mailingId}/test`, {}, (m) => m.testMailing(mailingId), 'POST'),

    /** Fige les destinataires et met l'envoi en file. */
    dispatch: (mailingId: Uuid): Promise<Mailing> =>
      send(`/admin/mailings/${mailingId}/send`, {}, (m) => m.dispatchMailing(mailingId), 'POST'),

    cancel: (mailingId: Uuid): Promise<Mailing> =>
      send(`/admin/mailings/${mailingId}/cancel`, {}, (m) => m.cancelMailing(mailingId), 'POST'),

    duplicate: (mailingId: Uuid): Promise<Mailing> =>
      send(`/admin/mailings/${mailingId}/duplicate`, {}, (m) => m.duplicateMailing(mailingId), 'POST'),

    /**
     * GABARITS PERSONNALISÉS (29/10). Ceux de l'édition et les communs ; leurs
     * images se posent par `PUT /media/attachments`, comme celles d'une édition.
     */
    templates: {
      list: (eventId: EventId, scope: AdministeredEvents): Promise<MailTemplate[]> => {
        assertEventInScope(eventId, scope)
        return call('/admin/mail-templates', (m) => m.mailTemplates(eventId), { event_id: eventId })
      },

      /** `null` : inexistant ou hors droit. */
      byId: (templateId: Uuid): Promise<MailTemplate | null> =>
        callOrNull(`/admin/mail-templates/${templateId}`, (m) => m.mailTemplateById(templateId)),

      create: (payload: MailTemplatePayload): Promise<MailTemplate> =>
        send('/admin/mail-templates', payload, (m) => m.createMailTemplate(payload), 'POST'),

      update: (templateId: Uuid, payload: MailTemplatePayload): Promise<MailTemplate> =>
        send(`/admin/mail-templates/${templateId}`, payload, (m) => m.updateMailTemplate(templateId, payload), 'PUT'),

      remove: (templateId: Uuid): Promise<{ deleted: boolean }> =>
        send(`/admin/mail-templates/${templateId}`, {}, (m) => m.deleteMailTemplate(templateId), 'DELETE'),

      /** Rendu par la fonction du worker ; `eventId` sert un gabarit commun. */
      preview: (templateId: Uuid, eventId?: EventId): Promise<MailTemplatePreview | null> =>
        callOrNull(`/admin/mail-templates/${templateId}/preview`, (m) => m.mailTemplatePreview(templateId), {
          event_id: eventId,
        }),
    },
  }
}
