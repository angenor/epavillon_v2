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
  CreateProposalMailingPayload,
  Mailing,
  MailingAddressSuggestion,
  MailingAudience,
  MailingListScreen,
  MailingPayload,
  MailingPreview,
  MailingTestResult,
  MailTemplate,
  MailTemplatePayload,
  MailTemplatePreview,
  MailText,
  MailTextPayload,
} from '~/types/admin-mailings'
import type {
  ScheduledMailBatch,
  ScheduledMailPreview,
  ScheduledMailPreviewPayload,
  ScheduledMailScreen,
  ScheduledMailTexts,
  ScheduledMailTypeTextPayload,
} from '~/types/admin-scheduled-mails'
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

    /**
     * Les courriels écrits depuis un dossier (01/11), sur son édition. `null` :
     * dossier inexistant ou hors périmètre ; sans la permission, 403.
     */
    listForProposal: (proposalId: Uuid): Promise<MailingListScreen | null> =>
      callOrNull('/admin/mailings', (m) => m.proposalMailingListScreen(proposalId), { proposal_id: proposalId }),

    /** Le brouillon d'un dossier : au déposant, le contact en copie, objet = titre de l'activité. */
    createForProposal: (proposalId: Uuid): Promise<Mailing> => {
      const payload: CreateProposalMailingPayload = { proposal_id: proposalId }
      return send('/admin/mailings/for-proposal', payload, (m) => m.createProposalMailing(payload), 'POST')
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

    /** Brouillon, envoi terminé, arrêté ou en échec ; un envoi en cours est refusé (409). */
    remove: (mailingId: Uuid): Promise<{ deleted: boolean }> =>
      send(`/admin/mailings/${mailingId}`, {}, (m) => m.deleteMailing(mailingId), 'DELETE'),

    /** Les personnes connues dont le nom ou l'adresse répondent à la saisie, huit au plus. */
    addressSuggestions: (eventId: EventId, query: string): Promise<MailingAddressSuggestion[]> =>
      call('/admin/mailings/address-suggestions', (m) => m.mailingAddressSuggestions(query), {
        event_id: eventId,
        q: query,
      }),

    /** L'essai part à la personne connectée, par la file. */
    test: (mailingId: Uuid): Promise<MailingTestResult> =>
      send(`/admin/mailings/${mailingId}/test`, {}, (m) => m.testMailing(mailingId), 'POST'),

    /** Fige les destinataires et met l'envoi en file. */
    dispatch: (mailingId: Uuid): Promise<Mailing> =>
      send(`/admin/mailings/${mailingId}/send`, {}, (m) => m.dispatchMailing(mailingId), 'POST'),

    cancel: (mailingId: Uuid): Promise<Mailing> =>
      send(`/admin/mailings/${mailingId}/cancel`, {}, (m) => m.cancelMailing(mailingId), 'POST'),

    /** Un envoi en échec repart pour ses seuls destinataires encore en attente. */
    retry: (mailingId: Uuid): Promise<Mailing> =>
      send(`/admin/mailings/${mailingId}/retry`, {}, (m) => m.retryMailing(mailingId), 'POST'),

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

    /**
     * TEXTES TYPES (01/11). Ceux de l'édition et les communs ; choisis dans la
     * fenêtre de rédaction, ils remplacent l'objet et le corps du brouillon.
     */
    texts: {
      list: (eventId: EventId, scope: AdministeredEvents): Promise<MailText[]> => {
        assertEventInScope(eventId, scope)
        return call('/admin/mail-texts', (m) => m.mailTexts(eventId), { event_id: eventId })
      },

      /** `null` : inexistant ou hors droit. */
      byId: (textId: Uuid): Promise<MailText | null> =>
        callOrNull(`/admin/mail-texts/${textId}`, (m) => m.mailTextById(textId)),

      create: (payload: MailTextPayload): Promise<MailText> =>
        send('/admin/mail-texts', payload, (m) => m.createMailText(payload), 'POST'),

      update: (textId: Uuid, payload: MailTextPayload): Promise<MailText> =>
        send(`/admin/mail-texts/${textId}`, payload, (m) => m.updateMailText(textId, payload), 'PUT'),

      remove: (textId: Uuid): Promise<{ deleted: boolean }> =>
        send(`/admin/mail-texts/${textId}`, {}, (m) => m.deleteMailText(textId), 'DELETE'),
    },

    /**
     * COURRIELS PROGRAMMÉS (04/11) : les rappels en file de l'édition, leurs
     * envois et leurs textes. Un envoi se désigne par sa clé, rendue par la
     * liste ; `null` : parti, inexistant ou hors droit.
     */
    scheduled: {
      list: (eventId: EventId, scope: AdministeredEvents): Promise<ScheduledMailScreen> => {
        assertEventInScope(eventId, scope)
        return call('/admin/scheduled-mails', (m) => m.scheduledMails(eventId), { event_id: eventId })
      },

      batch: (key: string): Promise<ScheduledMailBatch | null> =>
        callOrNull(`/admin/scheduled-mails/batches/${encodeURIComponent(key)}`, (m) => m.scheduledMailBatch(key)),

      /** N'écrit rien : rendu par la fonction du travail d'envoi. */
      preview: (key: string, payload: ScheduledMailPreviewPayload): Promise<ScheduledMailPreview> =>
        send(
          `/admin/scheduled-mails/batches/${encodeURIComponent(key)}/preview`,
          payload,
          (m) => m.scheduledMailPreview(key, payload),
          'POST',
        ),

      saveBatchText: (key: string, texts: ScheduledMailTexts): Promise<ScheduledMailBatch | null> =>
        send(
          `/admin/scheduled-mails/batches/${encodeURIComponent(key)}/text`,
          texts,
          (m) => m.saveScheduledBatchText(key, texts),
          'PUT',
        ),

      resetBatchText: (key: string): Promise<ScheduledMailBatch | null> =>
        send(
          `/admin/scheduled-mails/batches/${encodeURIComponent(key)}/text`,
          {},
          (m) => m.resetScheduledBatchText(key),
          'DELETE',
        ),

      saveTypeText: (typeKey: string, payload: ScheduledMailTypeTextPayload): Promise<ScheduledMailTexts> =>
        send(
          `/admin/scheduled-mails/types/${encodeURIComponent(typeKey)}/text`,
          payload,
          (m) => m.saveScheduledTypeText(typeKey, payload),
          'PUT',
        ),
    },
  }
}
