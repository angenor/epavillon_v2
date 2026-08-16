import type { DateInput } from '~/utils/datetime'
import type { TimeZoneName } from '~/types/shared'

/**
 * Formatage des dates lié à la locale active et aux gabarits de `_common.json`.
 *
 * Les fonctions pures vivent dans `app/utils/datetime.ts` ; ce composable ne
 * fait que les assembler avec les libellés traduits — c'est ce qui permet de
 * respecter les deux règles à la fois : aucune chaîne en dur, et toute date
 * affichée porte son fuseau.
 */
export function useDateTime() {
  const { t, locale, locales } = useI18n()

  /** Locale BCP 47 complète (`fr-FR`) attendue par `Intl`, déduite de la configuration. */
  const intlLocale = computed(() => {
    const current = (unref(locales) as Array<{ code: string; language?: string }>).find(
      (entry) => entry.code === locale.value,
    )
    return current?.language ?? locale.value
  })

  const base = (timeZone: TimeZoneName) => ({ timeZone, locale: intlLocale.value })

  return {
    intlLocale,

    /** « 14 novembre 2027 » */
    date: (value: DateInput | null | undefined, timeZone: TimeZoneName) =>
      formatDate(value, base(timeZone)),

    /** « 14:30 » — sans mention de fuseau : à réserver aux contextes qui la portent déjà. */
    time: (value: DateInput | null | undefined, timeZone: TimeZoneName) =>
      formatTime(value, base(timeZone)),

    /** « 14:30, heure de Belém » */
    timeWithZone: (
      value: DateInput | null | undefined,
      timeZone: TimeZoneName,
      zoneLabel?: string,
    ) => {
      const time = formatTime(value, base(timeZone))
      if (!time) return ''
      return t('common.datetime.timeWithZone', {
        time,
        zone: zoneLabel?.trim() || timeZoneCityLabel(timeZone),
      })
    },

    /** « 14 novembre 2027 à 14:30 » */
    dateTime: (value: DateInput | null | undefined, timeZone: TimeZoneName) => {
      const day = formatDate(value, base(timeZone))
      const time = formatTime(value, base(timeZone))
      if (!day) return ''
      return t('common.datetime.dayAndTime', { date: day, time })
    },

    /**
     * « 14:30 — 16:00, heure de Belém ».
     * Si la plage franchit minuit, les dates des deux bornes sont explicitées.
     */
    timeRange: (
      start: DateInput | null | undefined,
      end: DateInput | null | undefined,
      timeZone: TimeZoneName,
      zoneLabel?: string,
    ) => {
      const parts = timeRangeParts(start, end, { ...base(timeZone), zoneLabel })
      if (!parts) return ''
      if (!parts.end) {
        return t('common.datetime.timeWithZone', { time: parts.start, zone: parts.zone })
      }
      if (parts.sameDay) {
        return t('common.datetime.timeRangeWithZone', {
          start: parts.start,
          end: parts.end,
          zone: parts.zone,
        })
      }
      return t('common.datetime.timeRangeWithZone', {
        start: t('common.datetime.dayAndTime', { date: parts.startDate, time: parts.start }),
        end: t('common.datetime.dayAndTime', { date: parts.endDate ?? '', time: parts.end }),
        zone: parts.zone,
      })
    },

    /** « du 10 novembre 2027 au 21 novembre 2027 » */
    dateRange: (
      start: DateInput | null | undefined,
      end: DateInput | null | undefined,
      timeZone: TimeZoneName,
    ) => {
      const from = formatDate(start, base(timeZone))
      const to = formatDate(end, base(timeZone))
      if (!from) return ''
      if (!to) return from
      return t('common.datetime.dateRange', { start: from, end: to })
    },

    /** « UTC-03:00 » — mention technique, pour les infobulles et les exports. */
    zoneOffset: (timeZone: TimeZoneName, at?: DateInput) =>
      t('common.datetime.zoneOffset', { offset: timeZoneOffset({ ...base(timeZone), at }) }),

    /** « heure de Belém » — le suffixe seul, quand l'heure est déjà affichée. */
    zoneLabel: (timeZone: TimeZoneName, zoneLabel?: string) =>
      t('common.datetime.zoneOf', { zone: zoneLabel?.trim() || timeZoneCityLabel(timeZone) }),
  }
}
