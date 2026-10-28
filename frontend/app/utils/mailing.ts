import type {
  MailingCriterion,
  MailingCriterionKind,
  MailingRecipientStatus,
  MailingStatus,
} from '~/types/admin-mailings'

/** Les familles de destinataires, dans l'ordre où l'écran les propose. */
export const MAILING_KINDS: MailingCriterionKind[] = ['submitters', 'speakers', 'registrants', 'roles']

/** `programme.proposal_status`, sans les brouillons : un brouillon n'a rien déposé. */
export const MAILING_PROPOSAL_STATUSES = [
  'submitted',
  'under_review',
  'changes_requested',
  'accepted',
  'rejected',
  'withdrawn',
  'cancelled',
] as const

/** `programme.registration_status`. */
export const MAILING_REGISTRATION_STATUSES = ['registered', 'waitlisted', 'attended', 'no_show', 'cancelled'] as const

/** Les rôles qu'on attribue sur une édition (`identity.roles.allowed_scopes`). */
export const MAILING_ROLE_CODES = ['admin', 'reviewer', 'programmer'] as const

export function emptyCriterion(kind: MailingCriterionKind): MailingCriterion {
  return { kind, proposal_statuses: [], registration_statuses: [], role_codes: [], session_id: null }
}

/** Coupe une saisie libre (virgules, points-virgules, espaces, lignes), en minuscules, sans doublon. */
export function parseAddressList(text: string): string[] {
  const seen = new Set<string>()
  for (const piece of text.split(/[\s,;]+/)) {
    const address = piece.replace(/^<|>$/g, '').trim().toLowerCase()
    if (address) seen.add(address)
  }
  return [...seen]
}

/** Même contrôle que le domaine `platform.email`, pour signaler avant d'enregistrer. */
export function isEmailAddress(value: string): boolean {
  return /^[^@\s]+@[^@\s]+\.[^@\s]{2,}$/.test(value)
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

/**
 * Le rendu d'un aperçu : même substitution que le worker (`{{nom}}` remplacé
 * littéralement), valeurs échappées dans le HTML. Une variable sans valeur est
 * vide, comme à l'envoi.
 */
export function renderMailingTemplate(template: string, values: Record<string, string>, html: boolean): string {
  return template.replace(/\{\{\s*([a-z_][a-z0-9_]*)\s*\}\}/g, (whole: string, name: string) => {
    if (!(name in values)) return whole
    const value = values[name] ?? ''
    return html ? escapeHtml(value) : value
  })
}

/** Couleur d'état d'un envoi : en cours = attention, parti = confirmé, arrêté = clos. */
export function mailingStatusIntent(status: MailingStatus): 'neutral' | 'warning' | 'success' {
  if (status === 'queued') return 'warning'
  if (status === 'sent') return 'success'
  return 'neutral'
}

export function recipientStatusIntent(
  status: MailingRecipientStatus,
): 'neutral' | 'info' | 'warning' | 'success' | 'danger' {
  switch (status) {
    case 'sent':
      return 'success'
    case 'failed':
    case 'suppressed':
      return 'danger'
    case 'sending':
      return 'warning'
    case 'queued':
      return 'info'
    default:
      return 'neutral'
  }
}

/** `{{prenom}}`, composé ici : écrit dans un gabarit Vue ou un message i18n, il serait interprété. */
export function variableToken(name: string): string {
  return `{{${name}}}`
}

/** Les variables d'un envoi, dans l'ordre de l'écran ; miroir de `domain/mailing.rs`. */
export const MAILING_VARIABLES = ['prenom', 'nom', 'organisation', 'email', 'edition']
