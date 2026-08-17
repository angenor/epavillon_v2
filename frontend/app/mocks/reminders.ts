/**
 * Données simulées des RAPPELS — `engagement.reminder_rules` et
 * `engagement.scheduled_reminders` (`110_engagement.sql` § 6).
 *
 * LES QUATRE RAPPELS SONT CUMULÉS, ce n'est pas un choix parmi quatre. Le
 * commanditaire l'a dit ainsi et le modèle l'a écrit ainsi : la colonne
 * `offsets` a pour défaut `{2 days, 1 day, 1 hour, 30 minutes}` et les quatre
 * partent. Un écran qui laisserait choisir « le rappel » au singulier
 * contredirait la règle avant même que l'API n'existe.
 *
 * LES LIGNES NE SONT PAS ÉCRITES À LA MAIN, ELLES SONT DÉRIVÉES — et c'est
 * délibéré. La matérialisation compte une ligne PAR DESTINATAIRE, PAR CANAL ET
 * PAR DÉCALAGE : les sept inscrits d'une seule séance en produisent vingt-huit,
 * et le jeu complet plusieurs centaines. Les recopier serait illisible et faux
 * au premier changement d'inscription. `sessionReminders()` rejoue donc
 * `engagement.schedule_session_reminders()` en TypeScript, exactement comme
 * `organization-search.ts` rejoue `find_similar_organizations()` :
 *
 *   1. règle applicable — celle de la séance si elle existe, sinon celle de
 *      l'édition, SANS CUMUL (`ORDER BY (session_id IS NOT NULL) DESC LIMIT 1`) ;
 *   2. destinataires — inscriptions dont le statut n'est ni `cancelled`, ni
 *      `declined`, ni `waitlisted` ; une personne en liste d'attente n'a pas de
 *      place, la prévenir deux jours avant serait lui en promettre une ;
 *   3. un rappel par (personne × canal × décalage), dont l'instant vaut
 *      `starts_at − offset` ;
 *   4. les rappels dont l'heure est DÉJÀ PASSÉE au moment de la programmation ne
 *      sont pas créés : « on ne réveille personne à 3 h du matin parce qu'un
 *      import a pris du retard », dit le SQL.
 *
 * L'ÉTAT SE DÉDUIT DE L'INSTANT, ce que fait le worker en réalité : parti si
 * l'échéance est derrière nous, en attente sinon. C'est la seule liberté prise
 * avec la base — elle porte un `status` en colonne, écrit par le worker — et
 * elle évite de figer dans le jeu de données une date d'envoi qui vieillirait
 * mal.
 */

import type {
  NotificationChannel,
  OffsetMinutes,
  ReminderRule,
  ScheduledReminder,
} from '~/types/engagement'
import type { ReminderSlot } from '~/types/organization-workspace'
import { EVENT, PERSON, REMINDER_RULE, SCHEDULED_REMINDER } from './ids'
import { registrations } from './registrations'
import { allSessions } from './sessions'

/** Les quatre décalages du modèle, en minutes. */
const DEFAULT_OFFSETS: OffsetMinutes[] = [2 * 24 * 60, 24 * 60, 60, 30]

export const reminderRules = [
  {
    id: REMINDER_RULE.cop31,
    event_id: EVENT.cop31,
    session_id: null,
    offsets: DEFAULT_OFFSETS,
    channels: ['email'],
    type_code: 'programme.session.reminder',
    template_id: null,
    is_active: true,
    created_by: PERSON.perretAdmin,
    created_at: '2026-06-02T09:00:00Z',
    updated_at: '2026-06-02T09:00:00Z',
  },
  {
    id: REMINDER_RULE.cop30,
    event_id: EVENT.cop30,
    session_id: null,
    offsets: DEFAULT_OFFSETS,
    channels: ['email'],
    type_code: 'programme.session.reminder',
    template_id: null,
    is_active: true,
    created_by: PERSON.perretAdmin,
    created_at: '2025-06-04T09:00:00Z',
    updated_at: '2025-06-04T09:00:00Z',
  },
] satisfies ReminderRule[]

/** Statuts d'inscription qui ne reçoivent aucun rappel — cf. la fonction SQL. */
const EXCLUDED_STATUSES = new Set(['cancelled', 'declined', 'waitlisted'])

/**
 * La règle applicable à une séance : celle de la séance d'abord, celle de son
 * édition ensuite. Pas de cumul, pour que l'administrateur sache ce qui part.
 */
function ruleFor(sessionId: string, eventId: string): ReminderRule | null {
  const rules: ReminderRule[] = reminderRules
  return (
    rules.find((rule) => rule.is_active && rule.session_id === sessionId) ??
    rules.find((rule) => rule.is_active && rule.event_id === eventId) ??
    null
  )
}

/**
 * Matérialisation des rappels d'une séance — `schedule_session_reminders()`.
 *
 * Rendue publique parce que le planificateur (A9) et le back-office en auront
 * besoin ligne à ligne. L'espace organisation, lui, consomme l'agrégat
 * ci-dessous : il n'a pas à savoir QUI reçoit quoi.
 */
export function sessionReminders(sessionId: string, at: number = Date.now()): ScheduledReminder[] {
  const sessionIndex = allSessions.findIndex((s) => s.id === sessionId)
  const session = allSessions[sessionIndex]
  if (!session) return []

  const rule = ruleFor(session.id, session.event_id)
  if (!rule) return []

  const recipients = registrations.filter(
    (registration) =>
      registration.session_id === sessionId && !EXCLUDED_STATUSES.has(registration.status),
  )

  const startsAt = Date.parse(session.starts_at)
  const scheduled: ScheduledReminder[] = []
  let sequence = 0

  for (const registration of recipients) {
    for (const offset of rule.offsets) {
      for (const channel of rule.channels as NotificationChannel[]) {
        const scheduledFor = startsAt - offset * 60_000
        // La séance annulée ne rappelle rien : le worker marque `skipped`, et
        // le SQL prévoit le motif (`session_cancelled`).
        const isCancelled = session.status === 'cancelled'
        sequence += 1
        scheduled.push({
          // Numérotés dans l'espace de LEUR séance : deux séances distinctes ne
          // peuvent pas produire le même identifiant, ce qu'un simple compteur
          // local aurait fait dès qu'un écran en charge deux.
          id: SCHEDULED_REMINDER(sessionIndex * 1000 + sequence),
          rule_id: rule.id,
          session_id: session.id,
          person_id: registration.person_id,
          registration_id: registration.id,
          channel,
          offset_before: offset,
          scheduled_for: new Date(scheduledFor).toISOString(),
          status: isCancelled ? 'skipped' : scheduledFor <= at ? 'sent' : 'pending',
          job_id: null,
          sent_at: !isCancelled && scheduledFor <= at ? new Date(scheduledFor).toISOString() : null,
          skip_reason: isCancelled ? 'session_cancelled' : null,
          created_at: registration.created_at,
        })
      }
    }
  }

  return scheduled
}

/**
 * LE CALENDRIER DES RAPPELS D'UNE SÉANCE, tel que son organisation le lit :
 * une ligne par décalage, avec l'état et le nombre de destinataires.
 *
 * C'est l'agrégation que le modèle ne rend pas — obligation d'API inscrite au
 * prompt B6. Elle porte un NOMBRE et jamais une liste : les inscrits d'une
 * séance ne sont pas les données de l'organisation qui l'anime.
 */
export function sessionReminderSchedule(sessionId: string, at: number = Date.now()): ReminderSlot[] {
  const lines = sessionReminders(sessionId, at)
  const groups = new Map<string, ReminderSlot>()

  for (const line of lines) {
    // Clé sur le couple (décalage, canal) : deux canaux d'un même décalage sont
    // deux envois distincts, et les fondre en cacherait un.
    const key = `${line.offset_before}:${line.channel}`
    const existing = groups.get(key)
    if (existing) {
      existing.recipient_count += 1
      continue
    }
    groups.set(key, {
      offset_before: line.offset_before,
      channel: line.channel,
      scheduled_for: line.scheduled_for,
      status: line.status,
      recipient_count: 1,
      sent_at: line.sent_at,
    })
  }

  // Du plus lointain au plus proche du début — l'ordre dans lequel ils partent.
  return [...groups.values()].sort((a, b) => b.offset_before - a.offset_before)
}
