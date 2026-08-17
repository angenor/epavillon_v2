import type { IsoDateTime, TimeZoneName } from '~/types/shared'

/**
 * Dates et heures — briques de formatage pures.
 *
 * RÈGLE DE PROJET : toute date affichée porte son fuseau. Le modèle stocke les
 * instants en `timestamptz` et le fuseau de référence à côté
 * (`event.events.timezone`, `programme.sessions.timezone`) : « Les horaires sont
 * stockés en timestamptz et convertis à l'affichage — jamais l'inverse »
 * (docs/database/060_events.sql). Un formatage sans `timeZone` explicite
 * afficherait l'heure de la machine du visiteur, ce qui est faux pour un
 * événement qui se tient à Belém ou à Riyad.
 *
 * Ces fonctions ne connaissent pas i18n : elles produisent des morceaux
 * (« 14:30 », « Belém », « GMT-3 »). L'assemblage passe par les gabarits de
 * `_common.json`, via le composable `useDateTime()` — aucune chaîne en dur ici.
 */

export type DateInput = IsoDateTime | Date | number

export interface ZonedFormatOptions {
  /** Fuseau IANA de référence, ex. `America/Belem`. Obligatoire, par principe. */
  timeZone: TimeZoneName
  /** Locale BCP 47 complète, ex. `fr-FR`. */
  locale: string
}

/** Convertit une entrée en `Date`, ou `null` si la valeur est inexploitable. */
export function toDate(value: DateInput | null | undefined): Date | null {
  if (value === null || value === undefined || value === '') return null
  const date = value instanceof Date ? value : new Date(value)
  return Number.isNaN(date.getTime()) ? null : date
}

/** « 14 novembre 2027 » — la date seule, dans le fuseau de l'événement. */
export function formatDate(
  value: DateInput | null | undefined,
  options: ZonedFormatOptions & { dateStyle?: 'full' | 'long' | 'medium' | 'short' },
): string {
  const date = toDate(value)
  if (!date) return ''
  return new Intl.DateTimeFormat(options.locale, {
    dateStyle: options.dateStyle ?? 'long',
    timeZone: options.timeZone,
  }).format(date)
}

/** « 14:30 » — l'heure seule, dans le fuseau de l'événement. */
export function formatTime(
  value: DateInput | null | undefined,
  options: ZonedFormatOptions & { withSeconds?: boolean },
): string {
  const date = toDate(value)
  if (!date) return ''
  return new Intl.DateTimeFormat(options.locale, {
    hour: '2-digit',
    minute: '2-digit',
    second: options.withSeconds ? '2-digit' : undefined,
    hourCycle: 'h23',
    timeZone: options.timeZone,
  }).format(date)
}

/** « 14 novembre 2027, 14:30 » — date et heure d'un même instant. */
export function formatDateTime(
  value: DateInput | null | undefined,
  options: ZonedFormatOptions & { dateStyle?: 'full' | 'long' | 'medium' | 'short' },
): string {
  const date = toDate(value)
  if (!date) return ''
  return new Intl.DateTimeFormat(options.locale, {
    dateStyle: options.dateStyle ?? 'long',
    timeStyle: 'short',
    hourCycle: 'h23',
    timeZone: options.timeZone,
  }).format(date)
}

/** « jeu. 14 nov. » — repère court de colonne ou d'onglet de journée. */
export function formatWeekdayShort(
  value: DateInput | null | undefined,
  options: ZonedFormatOptions,
): string {
  const date = toDate(value)
  if (!date) return ''
  return new Intl.DateTimeFormat(options.locale, {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    timeZone: options.timeZone,
  }).format(date)
}

/** Nom court du fuseau tel que l'affiche le navigateur : « GMT-3 », « UTC+1 ». */
export function timeZoneShortName(
  options: ZonedFormatOptions & { at?: DateInput },
): string {
  const date = toDate(options.at) ?? new Date()
  return extractPart(date, options.locale, options.timeZone, 'shortOffset')
}

/** Décalage normalisé : « -03:00 », « +01:00 ». Utile pour un `<time>` ou un export. */
export function timeZoneOffset(
  options: ZonedFormatOptions & { at?: DateInput },
): string {
  const date = toDate(options.at) ?? new Date()
  const longOffset = extractPart(date, 'en-GB', options.timeZone, 'longOffset')
  // Intl renvoie « GMT-03:00 », et « GMT » tout court pour UTC.
  const normalized = longOffset.replace(/^(GMT|UTC)/, '')
  return normalized === '' ? '+00:00' : normalized
}

/**
 * Décalage COMPACT, à la façon dont on l'écrit dans une programmation :
 * « −3 », « +1 », « +5:30 ». Les minutes ne sont conservées que lorsqu'elles ne
 * sont pas nulles — la moitié des fuseaux du monde ne tombe pas sur l'heure
 * pleine, et « UTC+5 » serait faux pour l'Inde.
 *
 * Le signe est le SIGNE MOINS typographique (U+2212), pas un trait d'union :
 * « UTC−3 » et non « UTC-3 ». C'est la forme attendue dans les libellés de
 * créneau, où le trait d'union sert déjà à séparer les heures.
 */
export function timeZoneOffsetShort(
  options: ZonedFormatOptions & { at?: DateInput },
): string {
  const offset = timeZoneOffset(options) // « -03:00 », « +05:30 »
  const match = offset.match(/^([+-])(\d{2}):(\d{2})$/)
  if (!match) return offset

  const [, sign, hours, minutes] = match as unknown as [string, string, string, string]
  const signGlyph = sign === '-' ? '−' : '+'
  const hourValue = Number.parseInt(hours, 10)
  return minutes === '00'
    ? `${signGlyph}${hourValue}`
    : `${signGlyph}${hourValue}:${minutes}`
}

/**
 * Libellé de fuseau lisible, dérivé de l'identifiant IANA : `America/Belem`
 * devient « Belem ». C'est un repli : quand le lieu est connu (le nom de la
 * ville hôte de l'édition), lui préférer ce nom-là.
 */
export function timeZoneCityLabel(timeZone: TimeZoneName): string {
  const segment = timeZone.split('/').pop() ?? timeZone
  return segment.replace(/_/g, ' ')
}

/** Deux instants tombent-ils le même jour civil dans ce fuseau ? */
export function isSameDayInZone(
  a: DateInput | null | undefined,
  b: DateInput | null | undefined,
  timeZone: TimeZoneName,
): boolean {
  const dateA = toDate(a)
  const dateB = toDate(b)
  if (!dateA || !dateB) return false
  return dayKeyInZone(dateA, timeZone) === dayKeyInZone(dateB, timeZone)
}

/** Clé de journée civile dans un fuseau donné, au format `AAAA-MM-JJ`. */
export function dayKeyInZone(value: DateInput, timeZone: TimeZoneName): string {
  const date = toDate(value)
  if (!date) return ''
  // `en-CA` produit nativement AAAA-MM-JJ.
  return new Intl.DateTimeFormat('en-CA', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    timeZone,
  }).format(date)
}

/**
 * Heure MURALE d'un instant dans un fuseau donné, au format `AAAA-MM-JJ HH:MM`.
 *
 * Elle sert aux bibliothèques de calendrier, qui raisonnent en heure locale sans
 * fuseau : leur passer l'instant brut placerait le bloc à l'heure de la machine
 * du visiteur — 14 h à Belém deviendrait 18 h pour qui consulte depuis Dakar,
 * et la grille horaire du pavillon serait fausse. On convertit donc une fois,
 * ici, vers le fuseau de l'ÉDITION, et le calendrier n'a plus qu'à afficher.
 */
export function wallClockInZone(value: DateInput | null | undefined, timeZone: TimeZoneName): string {
  const date = toDate(value)
  if (!date) return ''
  const parts = new Intl.DateTimeFormat('en-CA', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    timeZone,
  }).formatToParts(date)

  const value_of = (type: Intl.DateTimeFormatPartTypes): string =>
    parts.find((part) => part.type === type)?.value ?? ''

  return `${value_of('year')}-${value_of('month')}-${value_of('day')} ${value_of('hour')}:${value_of('minute')}`
}

/**
 * L'INVERSE de `wallClockInZone()` : une heure murale saisie dans un fuseau
 * donné, rendue en instant ISO.
 *
 * Le formulaire de soumission (A4) saisit « le 12 novembre à 14:30 » et cette
 * heure vaut À BELÉM, pas sur la machine du déposant : un agent qui remplit son
 * dossier depuis Dakar propose bien 14:30 heure locale du pavillon. Sans cette
 * conversion, `new Date('2027-11-12T14:30')` produirait un instant lu dans le
 * fuseau du navigateur, et le créneau se déplacerait de trois heures entre la
 * saisie et l'affichage — sans qu'aucune erreur ne soit levée.
 *
 * DEUX PASSES, et la seconde n'est pas une précaution de style : le décalage
 * d'un fuseau dépend de l'instant qu'on y cherche, et l'instant dépend du
 * décalage. On approche d'abord en lisant l'heure murale comme si elle était
 * UTC, puis on corrige avec le décalage réellement en vigueur à cet instant-là.
 * C'est ce qui rend juste une saisie faite la nuit d'un changement d'heure.
 */
export function instantFromWallClock(
  wallClock: string | null | undefined,
  timeZone: TimeZoneName,
): IsoDateTime | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})/.exec(wallClock ?? '')
  if (!match) return null

  const [, year, month, day, hour, minute] = match as unknown as [string, string, string, string, string, string]
  const asIfUtc = Date.UTC(
    Number(year),
    Number(month) - 1,
    Number(day),
    Number(hour),
    Number(minute),
  )

  const approximate = asIfUtc - zoneOffsetMs(asIfUtc, timeZone)
  const corrected = asIfUtc - zoneOffsetMs(approximate, timeZone)
  return new Date(corrected).toISOString()
}

/** Décalage du fuseau, en millisecondes, à l'instant donné. */
function zoneOffsetMs(instant: number, timeZone: TimeZoneName): number {
  const wall = wallClockInZone(instant, timeZone)
  const match = /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})$/.exec(wall)
  if (!match) return 0

  const [, year, month, day, hour, minute] = match as unknown as [string, string, string, string, string, string]
  const asUtc = Date.UTC(Number(year), Number(month) - 1, Number(day), Number(hour), Number(minute))
  // L'instant est tronqué à la minute par `wallClockInZone` : on compare donc
  // deux valeurs tronquées, sans quoi les secondes fausseraient le décalage.
  return asUtc - Math.floor(instant / 60_000) * 60_000
}

/** Morceaux d'une plage horaire, à assembler avec un gabarit i18n. */
export interface TimeRangeParts {
  /** Heure de début, ex. « 14:30 ». */
  start: string
  /** Heure de fin, ex. « 16:00 ». */
  end: string
  /** Date du début, ex. « 14 novembre 2027 ». */
  startDate: string
  /** Date de fin, renseignée seulement si la plage franchit minuit. */
  endDate: string | null
  /** La plage tient-elle dans une seule journée civile du fuseau ? */
  sameDay: boolean
  /** Libellé de fuseau à afficher : celui fourni, sinon la ville de l'identifiant IANA. */
  zone: string
  /** Décalage normalisé, ex. « -03:00 ». */
  offset: string
  /** Durée en minutes, négative si les bornes sont inversées. */
  durationMinutes: number | null
}

/**
 * Décompose une plage horaire. Le libellé de fuseau attendu à l'affichage est le
 * nom du lieu (« heure de Belém ») : le passer par `zoneLabel` dès qu'il est
 * connu, la ville de l'identifiant IANA n'étant qu'un repli.
 */
export function timeRangeParts(
  start: DateInput | null | undefined,
  end: DateInput | null | undefined,
  options: ZonedFormatOptions & {
    zoneLabel?: string
    dateStyle?: 'full' | 'long' | 'medium' | 'short'
  },
): TimeRangeParts | null {
  const startDate = toDate(start)
  if (!startDate) return null
  const endDate = toDate(end)

  const sameDay = endDate ? isSameDayInZone(startDate, endDate, options.timeZone) : true

  return {
    start: formatTime(startDate, options),
    end: endDate ? formatTime(endDate, options) : '',
    startDate: formatDate(startDate, options),
    endDate: endDate && !sameDay ? formatDate(endDate, options) : null,
    sameDay,
    zone: options.zoneLabel?.trim() || timeZoneCityLabel(options.timeZone),
    offset: timeZoneOffset({ ...options, at: startDate }),
    durationMinutes: endDate
      ? Math.round((endDate.getTime() - startDate.getTime()) / 60000)
      : null,
  }
}

/** « 2 h 30 », « 45 min » — durée d'une session, à partir de minutes. */
export function durationParts(minutes: number | null): { hours: number; minutes: number } | null {
  if (minutes === null || !Number.isFinite(minutes) || minutes < 0) return null
  return { hours: Math.floor(minutes / 60), minutes: minutes % 60 }
}

function extractPart(
  date: Date,
  locale: string,
  timeZone: TimeZoneName,
  timeZoneName: 'shortOffset' | 'longOffset' | 'short' | 'long',
): string {
  const parts = new Intl.DateTimeFormat(locale, { timeZone, timeZoneName }).formatToParts(date)
  return parts.find((part) => part.type === 'timeZoneName')?.value ?? ''
}
