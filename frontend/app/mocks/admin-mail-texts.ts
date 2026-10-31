/**
 * TEXTES TYPES (01/11) : de quoi faire tourner la fenêtre de rédaction et leur
 * écran hors ligne. Trois des neuf textes semés par le modèle
 * (`engagement.mail_texts`, 110_engagement.sql), communs à toutes les éditions.
 */

import type { MailText, MailTextPayload } from '~/types/admin-mailings'
import type { EventId, Uuid } from '~/types/shared'
import { mockUuid } from './ids'

const SEEDED: MailText[] = [
  {
    id: mockUuid('7095', 1),
    name: 'Confirmation de sélection',
    category: 'Sélection et créneaux',
    sort_order: 10,
    subject: 'Confirmation de votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}',
    body_html:
      "<p>Madame, Monsieur,</p><p>Nous avons le plaisir de vous confirmer que votre proposition d'activité a été retenue pour être organisée au Pavillon de la Francophonie lors de la {{edition}}.</p><p>Titre : {{activite}}<br>Date : {{date}}<br>Horaire : {{heures}}</p><p>Nous vous remercions de bien vouloir nous confirmer ce créneau au plus tard le [date limite].</p>",
    event_id: null,
    created_by_name: null,
    created_at: '2026-11-01T08:00:00Z',
    updated_at: '2026-11-01T08:00:00Z',
    can_edit: true,
  },
  {
    id: mockUuid('7095', 2),
    name: "Liste d'attente",
    category: "Liste d'attente et annulation",
    sort_order: 60,
    subject: 'Programmation de votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}',
    body_html:
      "<p>Madame, Monsieur,</p><p>Nous avons inscrit votre activité sur notre liste d'attente.</p><p>Activité concernée :<br>Titre : {{activite}}</p>",
    event_id: null,
    created_by_name: null,
    created_at: '2026-11-01T08:00:00Z',
    updated_at: '2026-11-01T08:00:00Z',
    can_edit: true,
  },
  {
    id: mockUuid('7095', 3),
    name: 'Journée Finance climat',
    category: 'Journées spéciales',
    sort_order: 90,
    subject:
      'Précision sur votre activité au Pavillon de la Francophonie ({{edition}}) - Journée spéciale Finance climat - {{activite}}',
    body_html:
      '<p>Madame, Monsieur,</p><p>Votre activité a été intégrée à la Journée spéciale Finance climat.</p><p>Titre : {{activite}}<br>Date : {{date}}<br>Plage horaire : {{heures}}</p>',
    event_id: null,
    created_by_name: null,
    created_at: '2026-11-01T08:00:00Z',
    updated_at: '2026-11-01T08:00:00Z',
    can_edit: true,
  },
]

const texts = new Map<Uuid, MailText>(SEEDED.map((text) => [text.id, text]))
let sequence = 10

function now(): string {
  return new Date().toISOString()
}

/** Même ordre que l'API : catégorie (sans catégorie à la fin), rang, nom. */
export function mailTexts(eventId: EventId): MailText[] {
  const key = (t: MailText) => t.category?.toLowerCase() ?? '￿'
  return [...texts.values()]
    .filter((t) => t.event_id === null || t.event_id === eventId)
    .sort((a, b) => key(a).localeCompare(key(b)) || a.sort_order - b.sort_order || a.name.localeCompare(b.name))
}

export function mailTextById(id: Uuid): MailText | null {
  return texts.get(id) ?? null
}

export function createMailText(payload: MailTextPayload): MailText {
  const id = mockUuid('7095', sequence++)
  const text: MailText = { ...payload, id, created_by_name: 'Awa Sow Fall', created_at: now(), updated_at: now(), can_edit: true }
  texts.set(id, text)
  return text
}

export function updateMailText(id: Uuid, payload: MailTextPayload): MailText {
  const current = texts.get(id)
  if (!current) throw new Error('texte type introuvable')
  const next: MailText = { ...current, ...payload, updated_at: now() }
  texts.set(id, next)
  return next
}

export function deleteMailText(id: Uuid): { deleted: boolean } {
  return { deleted: texts.delete(id) }
}
