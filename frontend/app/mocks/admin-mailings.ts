/**
 * ENVOIS DE COURRIELS (28/10) : de quoi faire tourner l'écran hors ligne.
 *
 * Rejoue en mémoire, le temps d'une session, ce que l'API fait : brouillon,
 * liste avant envoi, envoi figé. La liste est un jeu fixe de quatre adresses ;
 * le vrai calcul vit dans `engagement.mailing_audience()`.
 */

import type {
  CreateMailingPayload,
  Mailing,
  MailingAudience,
  MailingAudienceEntry,
  MailingCounts,
  MailingListScreen,
  MailingPayload,
  MailingPreview,
  MailingRecipient,
  MailingSummary,
  MailingTestResult,
  MailTemplate,
  MailTemplatePayload,
  MailTemplatePreview,
} from '~/types/admin-mailings'
import type { EventId, Uuid } from '~/types/shared'
import { EVENT, mockUuid } from './ids'

const ATTACHMENTS_MAX = 10 * 1024 * 1024
const VARIABLES = ['prenom', 'nom', 'organisation', 'email', 'edition']
const ME = 'a.sowfall@roac-afrique.org'

const ENTRIES: MailingAudienceEntry[] = [
  { email: 'k.diallo@example.org', person_id: mockUuid('7090', 1), first_name: 'Karim', last_name: 'Diallo', organization: 'ROAC', sources: ['submitters'], is_suppressed: false },
  { email: 'f.ndiaye@example.org', person_id: mockUuid('7090', 2), first_name: 'Fatou', last_name: 'Ndiaye', organization: 'ENDA', sources: ['registrants', 'speakers'], is_suppressed: false },
  { email: 'ancienne@example.org', person_id: mockUuid('7090', 3), first_name: 'Jean', last_name: 'Martin', organization: null, sources: ['registrants'], is_suppressed: true },
  { email: 'contact@partenaire.org', person_id: null, first_name: null, last_name: null, organization: null, sources: ['addresses'], is_suppressed: false },
]

const NO_COUNTS: MailingCounts = { total: 0, queued: 0, sending: 0, sent: 0, failed: 0, suppressed: 0, cancelled: 0 }

const store = new Map<Uuid, Mailing>()
let sequence = 900

function now(): string {
  return new Date().toISOString()
}

function seed(): void {
  if (store.size) return
  const id = mockUuid('7091', 1)
  const recipients: MailingRecipient[] = ENTRIES.map((entry, index) => ({
    id: mockUuid('7092', index + 1),
    email: entry.email,
    person_id: entry.person_id,
    first_name: entry.first_name,
    last_name: entry.last_name,
    organization: entry.organization,
    sources: entry.sources,
    status: entry.is_suppressed ? 'suppressed' : 'sent',
    last_error: entry.is_suppressed ? 'adresse sur la liste de suppression' : null,
    sent_at: entry.is_suppressed ? null : '2026-10-20T09:12:00Z',
  }))
  store.set(id, {
    id,
    event_id: EVENT.cop31,
    status: 'sent',
    subject: 'Votre activité au Pavillon : informations pratiques',
    body_html: '<p>Bonjour {{prenom}},</p><p>Voici les informations pratiques pour {{edition}}.</p>',
    audience: [{ kind: 'submitters', proposal_statuses: ['accepted'], registration_statuses: [], role_codes: [], session_id: null }],
    extra_addresses: ['contact@partenaire.org'],
    cc: [ME],
    bcc: [],
    reply_to: null,
    template: 'sobre',
    variable_fallbacks: { prenom: 'Madame, Monsieur' },
    reply_button: false,
    cta_label: null,
    cta_url: null,
    mail_template_id: null,
    duplicated_from: null,
    created_by_name: 'Awa Sow Fall',
    queued_by_name: 'Awa Sow Fall',
    queued_at: '2026-10-20T09:10:00Z',
    completed_at: '2026-10-20T09:14:00Z',
    cancelled_at: null,
    created_at: '2026-10-20T08:40:00Z',
    updated_at: '2026-10-20T09:14:00Z',
    counts: countsOf(recipients),
    recipients,
    attachments: [],
    attachments_total_bytes: 0,
    attachments_max_bytes: ATTACHMENTS_MAX,
  })
}

function countsOf(recipients: MailingRecipient[]): MailingCounts {
  const counts = { ...NO_COUNTS, total: recipients.length }
  for (const r of recipients) counts[r.status] += 1
  return counts
}

function summary(m: Mailing): MailingSummary {
  return {
    id: m.id,
    event_id: m.event_id,
    status: m.status,
    subject: m.subject,
    created_by_name: m.created_by_name,
    queued_by_name: m.queued_by_name,
    queued_at: m.queued_at,
    completed_at: m.completed_at,
    created_at: m.created_at,
    updated_at: m.updated_at,
    counts: m.counts,
    attachment_count: m.attachments.length,
  }
}

function found(id: Uuid): Mailing {
  seed()
  const mailing = store.get(id)
  if (!mailing) throw new Error('envoi introuvable')
  return mailing
}

function apply(m: Mailing, p: MailingPayload): Mailing {
  return {
    ...m,
    subject: p.subject ?? m.subject,
    body_html: p.body_html ?? m.body_html,
    audience: p.audience ?? m.audience,
    extra_addresses: p.extra_addresses ?? m.extra_addresses,
    cc: p.cc ?? m.cc,
    bcc: p.bcc ?? m.bcc,
    reply_to: p.reply_to === undefined ? m.reply_to : p.reply_to || null,
    event_id: p.event_id ?? m.event_id,
    ...templateOf(m, p),
    variable_fallbacks: p.variable_fallbacks ?? m.variable_fallbacks,
    reply_button: p.reply_button ?? m.reply_button,
    cta_label: p.cta_label === undefined ? m.cta_label : p.cta_label || null,
    cta_url: p.cta_url === undefined ? m.cta_url : p.cta_url || null,
    updated_at: now(),
  }
}

/** Le gabarit personnalisé l'emporte ; un modèle par défaut seul le retire. */
function templateOf(m: Mailing, p: MailingPayload): Pick<Mailing, 'template' | 'mail_template_id'> {
  if (p.mail_template_id !== undefined) {
    const custom = p.mail_template_id ? templates.get(p.mail_template_id) : undefined
    return custom
      ? { template: custom.layout, mail_template_id: custom.id }
      : { template: p.template ?? m.template, mail_template_id: null }
  }
  if (p.template !== undefined) return { template: p.template, mail_template_id: null }
  return { template: m.template, mail_template_id: m.mail_template_id }
}

export function mailingListScreen(eventId: EventId): MailingListScreen {
  seed()
  return {
    rows: [...store.values()].filter((m) => m.event_id === eventId).map(summary),
    team_cc: [],
    variables: VARIABLES,
  }
}

export function mailingById(id: Uuid): Mailing | null {
  seed()
  return store.get(id) ?? null
}

export function mailingAudience(id: Uuid): MailingAudience | null {
  const m = mailingById(id)
  if (!m) return null
  const entries = m.audience.length || m.extra_addresses.length ? ENTRIES : []
  const suppressed = entries.filter((e) => e.is_suppressed).length
  const sendable = entries.length - suppressed
  const envelope = sendable * (1 + m.cc.length + m.bcc.length)
  return {
    total: entries.length,
    sendable,
    suppressed,
    without_first_name: entries.filter((e) => !e.is_suppressed && !e.first_name).length,
    without_account: entries.filter((e) => !e.is_suppressed && !e.person_id).length,
    fallback_count: entries.filter((e) => !e.is_suppressed && !e.first_name).length,
    envelope_count: envelope,
    suggested_mail_template_id: suggestedTemplate(m),
    entries,
  }
}

export function createMailing(payload: CreateMailingPayload): Mailing {
  seed()
  const id = mockUuid('7091', sequence++)
  const draft: Mailing = apply(
    {
      id,
      event_id: payload.event_id,
      status: 'draft',
      subject: '',
      body_html: '',
      audience: [],
      extra_addresses: [],
      cc: [ME],
      bcc: [],
      reply_to: null,
      template: 'sobre',
      variable_fallbacks: { prenom: 'Madame, Monsieur' },
      reply_button: false,
      cta_label: null,
      cta_url: null,
      mail_template_id: null,
      duplicated_from: null,
      created_by_name: 'Awa Sow Fall',
      queued_by_name: null,
      queued_at: null,
      completed_at: null,
      cancelled_at: null,
      created_at: now(),
      updated_at: now(),
      counts: NO_COUNTS,
      recipients: [],
      attachments: [],
      attachments_total_bytes: 0,
      attachments_max_bytes: ATTACHMENTS_MAX,
    },
    payload,
  )
  store.set(id, draft)
  return draft
}

export function updateMailing(id: Uuid, payload: MailingPayload): Mailing {
  const next = apply(found(id), payload)
  store.set(id, next)
  return next
}

export function deleteMailing(id: Uuid): { deleted: boolean } {
  return { deleted: store.delete(id) }
}

export function testMailing(id: Uuid): MailingTestResult {
  found(id)
  return { to: ME }
}

export function dispatchMailing(id: Uuid): Mailing {
  const m = found(id)
  const recipients: MailingRecipient[] = (mailingAudience(id)?.entries ?? []).map((entry, index) => ({
    id: mockUuid('7093', sequence * 10 + index),
    email: entry.email,
    person_id: entry.person_id,
    first_name: entry.first_name,
    last_name: entry.last_name,
    organization: entry.organization,
    sources: entry.sources,
    status: entry.is_suppressed ? 'suppressed' : 'queued',
    last_error: entry.is_suppressed ? 'adresse sur la liste de suppression' : null,
    sent_at: null,
  }))
  const next: Mailing = {
    ...m,
    status: 'queued',
    queued_at: now(),
    queued_by_name: 'Awa Sow Fall',
    recipients,
    counts: countsOf(recipients),
  }
  store.set(id, next)
  return next
}

export function cancelMailing(id: Uuid): Mailing {
  const m = found(id)
  const recipients = m.recipients.map((r) => (r.status === 'queued' ? { ...r, status: 'cancelled' as const } : r))
  const next: Mailing = { ...m, status: 'cancelled', cancelled_at: now(), recipients, counts: countsOf(recipients) }
  store.set(id, next)
  return next
}

export function duplicateMailing(id: Uuid): Mailing {
  const m = found(id)
  const copy = createMailing({ event_id: m.event_id })
  const next: Mailing = {
    ...copy,
    subject: m.subject,
    body_html: m.body_html,
    audience: m.audience,
    extra_addresses: m.extra_addresses,
    cc: m.cc,
    bcc: m.bcc,
    reply_to: m.reply_to,
    template: m.template,
    mail_template_id: m.mail_template_id,
    variable_fallbacks: m.variable_fallbacks,
    reply_button: m.reply_button,
    cta_label: m.cta_label,
    cta_url: m.cta_url,
    duplicated_from: m.id,
  }
  store.set(next.id, next)
  return next
}

/** Hors ligne, un aperçu sans gabarit : le corps, variables remplacées par leur secours. */
export function mailingPreview(id: Uuid, email?: string): MailingPreview | null {
  const m = mailingById(id)
  if (!m) return null
  const entry = ENTRIES.find((e) => !e.is_suppressed && (!email || e.email === email))
  const prenom = entry?.first_name ?? m.variable_fallbacks.prenom ?? ''
  const html = m.body_html.replace(/\{\{\s*prenom\s*\}\}/g, prenom)
  return { to: entry?.email ?? 'destinataire@exemple.org', has_account: Boolean(entry?.person_id), subject: m.subject, html }
}

// ---------------------------------------------------------------------------
// Gabarits personnalisés (29/10)
// ---------------------------------------------------------------------------

const templates = new Map<Uuid, MailTemplate>()
let templateSequence = 1

/** Comme `engagement.mailing_suggested_template()` : écrire aux déposants de l'édition. */
function suggestedTemplate(m: Mailing): Uuid | null {
  const writesToSubmitters =
    m.audience.some((c) => c.kind === 'submitters') ||
    ENTRIES.some((e) => e.sources.includes('submitters') && m.extra_addresses.includes(e.email))
  if (!writesToSubmitters) return null
  return [...templates.values()].find((t) => t.event_id === m.event_id && t.preselect_for_submitters)?.id ?? null
}

function foundTemplate(id: Uuid): MailTemplate {
  const template = templates.get(id)
  if (!template) throw new Error('gabarit introuvable')
  return template
}

export function mailTemplates(eventId: EventId): MailTemplate[] {
  return [...templates.values()]
    .filter((t) => t.event_id === null || t.event_id === eventId)
    .sort((a, b) => Number(a.event_id === null) - Number(b.event_id === null) || a.name.localeCompare(b.name))
}

export function mailTemplateById(id: Uuid): MailTemplate | null {
  return templates.get(id) ?? null
}

export function createMailTemplate(payload: MailTemplatePayload): MailTemplate {
  const id = mockUuid('7094', templateSequence++)
  const template: MailTemplate = {
    ...payload,
    id,
    created_by_name: 'Awa Sow Fall',
    created_at: now(),
    updated_at: now(),
    background: null,
    logo: null,
    can_edit: true,
  }
  templates.set(id, template)
  return template
}

export function updateMailTemplate(id: Uuid, payload: MailTemplatePayload): MailTemplate {
  const next: MailTemplate = { ...foundTemplate(id), ...payload, updated_at: now() }
  templates.set(id, next)
  return next
}

export function deleteMailTemplate(id: Uuid): { deleted: boolean } {
  return { deleted: templates.delete(id) }
}

/** Hors ligne, un aperçu sans mise en page : le nom et le titre. */
export function mailTemplatePreview(id: Uuid): MailTemplatePreview | null {
  const t = templates.get(id)
  if (!t) return null
  const title = t.title ?? t.name
  return { subject: t.name, html: `<p><strong>${title}</strong></p><p>${t.subtitle ?? ''}</p>` }
}
