/**
 * COURRIELS PROGRAMMÉS (04/11) : l'onglet « Programmés » hors ligne, avec les
 * cas de la maquette (`docs/maquettes/courriels-a-venir/`) : un rappel de
 * journée à 2 jours et 1 jour, un envoi au texte propre, les quatre rappels
 * d'activité. Les heures se calent sur l'instant du chargement : le décompte
 * tourne.
 */

import type {
  ScheduledMailBatch,
  ScheduledMailBatchRow,
  ScheduledMailKind,
  ScheduledMailLocale,
  ScheduledMailPreview,
  ScheduledMailPreviewPayload,
  ScheduledMailScreen,
  ScheduledMailTexts,
  ScheduledMailTypeTextPayload,
} from '~/types/admin-scheduled-mails'
import type { EventId } from '~/types/shared'
import { groupScheduledBatches, scheduledTypeKey } from '~/utils/scheduled-mails'
import { mockUuid } from './ids'

const MINUTE = 60_000
const loadedAt = Date.now()

const DEFAULTS: Record<ScheduledMailKind, ScheduledMailTexts> = {
  day: {
    fr: {
      subject: '{{titre_journee}} commence {{delai}}',
      body: 'Bonjour {{prenom}},\n\n« {{titre_journee}} » commence {{delai}}, le {{date}}.\n\nSuivez le direct avec le lien de votre courriel de confirmation, ou depuis la page de la journée.',
    },
    en: {
      subject: '{{titre_journee}} starts {{delai}}',
      body: 'Hello {{prenom}},\n\n"{{titre_journee}}" starts {{delai}}, on {{date}}.\n\nFollow the live stream with the link in your confirmation email, or from the page of the day.',
    },
  },
  activity: {
    fr: {
      subject: '{{titre_activite}} commence {{delai}}',
      body: "Bonjour {{prenom}},\n\n« {{titre_activite}} » commence {{delai}}, le {{date}}.\n\nRetrouvez l'activité et son direct sur sa page.",
    },
    en: {
      subject: '{{titre_activite}} starts {{delai}}',
      body: 'Hello {{prenom}},\n\n"{{titre_activite}}" starts {{delai}}, on {{date}}.\n\nFind the activity and its live stream on its page.',
    },
  },
}

const VARIABLES: Record<ScheduledMailKind, string[]> = {
  day: ['prenom', 'titre_journee', 'date', 'delai', 'edition'],
  activity: ['prenom', 'titre_activite', 'date', 'delai', 'edition'],
}

interface Seed {
  kind: ScheduledMailKind
  n: number
  title: string
  offset: number
  /** Minutes avant le départ, depuis le chargement. */
  inMinutes: number
  recipients: number
}

const ATELIER = 'Atelier préparatoire francophone des négociateurs-trices'
const SEEDS: Seed[] = [
  { kind: 'day', n: 1, title: ATELIER, offset: 2880, inMinutes: 2 * 1440 + 11 * 60 + 58, recipients: 4 },
  { kind: 'day', n: 1, title: ATELIER, offset: 1440, inMinutes: 3 * 1440 + 11 * 60 + 58, recipients: 4 },
  { kind: 'day', n: 2, title: 'Journée finance durable', offset: 2880, inMinutes: 7 * 1440 + 10 * 60 + 58, recipients: 1 },
  { kind: 'day', n: 2, title: 'Journée finance durable', offset: 1440, inMinutes: 8 * 1440 + 10 * 60 + 58, recipients: 1 },
  { kind: 'day', n: 3, title: 'Journée jeunesse', offset: 2880, inMinutes: 9 * 1440 + 10 * 60 + 58, recipients: 1 },
  { kind: 'day', n: 3, title: 'Journée jeunesse', offset: 1440, inMinutes: 10 * 1440 + 10 * 60 + 58, recipients: 1 },
  ...[2880, 1440, 60, 30].flatMap((offset) => {
    const base = 3 * 1440 + 11 * 60 + 58 + (2880 - offset)
    return [
      { kind: 'activity' as const, n: 11, title: "Financer l'adaptation dans l'espace francophone", offset, inMinutes: base, recipients: 14 },
      { kind: 'activity' as const, n: 12, title: 'Océans et littoraux : retours de terrain', offset, inMinutes: base + 1440 + 90, recipients: 8 },
      { kind: 'activity' as const, n: 13, title: 'Pertes et préjudices : où en sont les négociations', offset, inMinutes: base + 2 * 1440, recipients: 4 },
    ]
  }),
]

const DAY_START_SECONDS = Math.floor(loadedAt / 1000)

function rowOf(seed: Seed): ScheduledMailBatchRow {
  const target = mockUuid(seed.kind === 'day' ? '7096' : '7097', seed.n)
  const runAt = new Date(loadedAt + seed.inMinutes * MINUTE).toISOString()
  const startsAt = seed.kind === 'day' ? DAY_START_SECONDS + seed.n * 86_400 : null
  return {
    key: seed.kind === 'day' ? `day.${target}.${seed.offset}.${startsAt}` : `activity.${target}.${seed.offset}`,
    type_key: scheduledTypeKey(seed.kind, seed.offset),
    kind: seed.kind,
    offset_minutes: seed.offset,
    target_id: target,
    target_starts_at: startsAt === null ? null : new Date(startsAt * 1000).toISOString(),
    title: seed.title,
    run_at: runAt,
    recipients: seed.recipients,
    has_own_text: false,
  }
}

const rows = SEEDS.map(rowOf)
const typeTexts = new Map<string, ScheduledMailTexts>()
const ownTexts = new Map<string, ScheduledMailTexts>()

// L'envoi « Journée finance durable, 2 jours avant » a son propre texte, comme sur la maquette.
const financeTwoDays = rows.find((r) => r.title === 'Journée finance durable' && r.offset_minutes === 2880)
if (financeTwoDays) {
  ownTexts.set(financeTwoDays.key, {
    fr: {
      subject: '{{titre_journee}} commence dans 2 jours',
      body: "Bonjour {{prenom}},\n\n« {{titre_journee}} » commence dans 2 jours, le {{date}}.\nLe programme a été mis à jour : un moment d'échange avec les négociateurs en chef s'ajoute à 11:00.\n\nSuivez le direct avec le lien de votre courriel de confirmation, ou depuis la page de la journée.",
    },
    en: DEFAULTS.day.en,
  })
}

function nowIso(): string {
  return new Date().toISOString()
}

function liveRows(): ScheduledMailBatchRow[] {
  const now = Date.now()
  return rows
    .filter((r) => Date.parse(r.run_at) > now)
    .map((r) => ({ ...r, has_own_text: ownTexts.has(r.key) }))
}

export function scheduledMails(_eventId: EventId): ScheduledMailScreen {
  const live = liveRows()
  const types = groupScheduledBatches(live, new Set(typeTexts.keys()))
  const next = [...live].sort((a, b) => Date.parse(a.run_at) - Date.parse(b.run_at))[0] ?? null
  return { observed_at: nowIso(), total: live.reduce((sum, r) => sum + r.recipients, 0), next, types }
}

export function scheduledMailBatch(key: string): ScheduledMailBatch | null {
  const live = liveRows()
  const batch = live.find((r) => r.key === key)
  if (!batch) return null
  return {
    observed_at: nowIso(),
    batch,
    type_recipients: live.filter((r) => r.type_key === batch.type_key).reduce((sum, r) => sum + r.recipients, 0),
    type_text: typeTexts.get(batch.type_key) ?? DEFAULTS[batch.kind],
    type_text_is_custom: typeTexts.has(batch.type_key),
    own_text: ownTexts.get(key) ?? null,
    variables: VARIABLES[batch.kind],
    preview_locale: 'fr',
  }
}

const SAMPLE_VALUES: Record<ScheduledMailLocale, Record<string, string>> = {
  fr: { prenom: 'Awa', date: 'dimanche 8 novembre 2026 à 09:00 (heure de Belém)', edition: 'CdP31' },
  en: { prenom: 'Awa', date: 'Sunday 8 November 2026 at 09:00 (Belém time)', edition: 'COP31' },
}

function escape(value: string): string {
  return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function delayOf(offset: number, locale: ScheduledMailLocale): string {
  const en = locale === 'en'
  if (offset === 1440) return en ? 'tomorrow' : 'demain'
  const [count, unitFr, unitEn] =
    offset % 1440 === 0 ? [offset / 1440, 'jour', 'day'] : offset % 60 === 0 ? [offset / 60, 'heure', 'hour'] : [offset, 'minute', 'minute']
  const plural = count > 1 ? 's' : ''
  return en ? `in ${count} ${unitEn}${plural}` : `dans ${count} ${unitFr}${plural}`
}

/** Un rendu approché du gabarit des confirmations : la vraie composition est celle de l'API. */
export function scheduledMailPreview(key: string, payload: ScheduledMailPreviewPayload): ScheduledMailPreview {
  const batch = rows.find((r) => r.key === key)
  const locale = payload.locale
  const values: Record<string, string> = {
    ...SAMPLE_VALUES[locale],
    delai: delayOf(batch?.offset_minutes ?? 2880, locale),
    titre_journee: batch?.title ?? '',
    titre_activite: batch?.title ?? '',
  }
  const fill = (text: string, mark: boolean) =>
    text.replace(/\{\{\s*([a-z_]+)\s*\}\}/g, (whole, name: string) => {
      const value = values[name]
      if (value === undefined) return whole
      return mark ? `<span style="background-color:#e0f3fb;border-radius:2px;padding:0 2px">${escape(value)}</span>` : value
    })
  const paragraphs = payload.body
    .split(/\n\s*\n/)
    .map((p) => p.trim())
    .filter(Boolean)
    .map((p) => `<p style="margin:0 0 16px">${fill(escape(p), true).replace(/\n/g, '<br>')}</p>`)
    .join('')
  const action = batch?.kind === 'day' ? (locale === 'en' ? 'View the day' : 'Voir la journée') : locale === 'en' ? 'View the activity' : "Voir l'activité"
  const html = `<!doctype html><html><body style="margin:0;background:#f4f5f7;font-family:Arial,sans-serif;color:#231f20">
<div style="max-width:600px;margin:16px auto;background:#fff;border:1px solid #e3e5ea">
<div style="height:56px;background:#1d1a5b;color:#fff;font-weight:bold;display:flex;align-items:center;padding:0 20px">ePavillon de la Francophonie</div>
<div style="height:4px;background:linear-gradient(90deg,#00a1e4 0 20%,#8fbf2f 20% 40%,#ffd500 40% 60%,#e63132 60% 80%,#732f85 80%)"></div>
<div style="padding:20px 24px;font-size:15px;line-height:1.6">${paragraphs}
<p><a href="#" style="display:inline-block;padding:12px 24px;border-radius:6px;background:#0c6792;color:#fff;font-weight:bold;text-decoration:none">${escape(action)}</a></p>
</div></div></body></html>`
  return { locale, subject: fill(payload.subject, false), html }
}

export function saveScheduledBatchText(key: string, texts: ScheduledMailTexts): ScheduledMailBatch | null {
  ownTexts.set(key, texts)
  return scheduledMailBatch(key)
}

export function resetScheduledBatchText(key: string): ScheduledMailBatch | null {
  ownTexts.delete(key)
  return scheduledMailBatch(key)
}

export function saveScheduledTypeText(typeKey: string, payload: ScheduledMailTypeTextPayload): ScheduledMailTexts {
  const texts: ScheduledMailTexts = { fr: payload.fr, en: payload.en }
  typeTexts.set(typeKey, texts)
  return texts
}

