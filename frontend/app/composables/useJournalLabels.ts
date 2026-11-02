import type { JournalChange } from '~/types/admin-journal'

/**
 * Libellés et heures du journal du back-office.
 *
 * Un code de tâche, d'événement, de table ou de colonne est un nom technique :
 * on le traduit quand la clé existe, et on rend le code brut sinon. Un travail
 * ajouté au worker avant d'être nommé ici reste lisible, il n'est pas masqué.
 *
 * Les heures se lisent dans le fuseau de la personne connectée, à défaut en
 * UTC : une ligne de journal n'appartient à aucune édition.
 */
export function useJournalLabels() {
  const { t, te } = useI18n()
  const auth = useAuthStore()
  const dates = useDateTime()

  const timezone = computed(() => auth.person?.timezone ?? 'UTC')
  // « heure d'UTC » ne se dit pas : le repli sur UTC a son propre libellé.
  const zone = computed(() =>
    /^(Etc\/)?UTC$/.test(timezone.value) ? t('admin.journal.zoneUtc') : dates.zoneLabel(timezone.value),
  )

  /** « Heure de Montréal » : l'en-tête d'une colonne d'heures porte son fuseau. */
  const zoneTitle = computed(() => zone.value.charAt(0).toLocaleUpperCase() + zone.value.slice(1))

  function labelOr(prefix: string, code: string): string {
    const key = `admin.journal.${prefix}.${journalCodeKey(code)}`
    return te(key) ? t(key) : code
  }

  const taskLabel = (task: string): string => labelOr('tasks', task)
  const eventLabel = (eventType: string): string => labelOr('events', eventType)
  const fieldLabel = (name: string): string => labelOr('fields', name)
  const entityLabel = (schema: string, table: string): string => labelOr('entities', `${schema}.${table}`)
  const changeEntityLabel = (change: JournalChange): string =>
    entityLabel(change.entity_schema, change.entity_table)

  /** « 12:34:46 » aujourd'hui, « 2 nov. 12:34:46 » sinon. Le fuseau est dans l'en-tête de colonne. */
  function rowTime(value: string): string {
    const clock = journalClock(value, timezone.value, dates.intlLocale.value)
    const today = dayKeyInZone(new Date(), timezone.value)
    if (dayKeyInZone(value, timezone.value) === today) return clock
    return `${journalShortDay(value, timezone.value, dates.intlLocale.value)} ${clock}`
  }

  /** « 12:34:46,012 » : le fil d'une demande se lit à la milliseconde. */
  const preciseTime = (value: string): string =>
    journalClock(value, timezone.value, dates.intlLocale.value, true)

  /** « lundi 2 novembre, 12:34:46 (heure de Paris) » */
  const fullTime = (value: string): string =>
    t('admin.journal.detail.whenValue', {
      day: dates.dayLong(value, timezone.value),
      time: journalClock(value, timezone.value, dates.intlLocale.value),
      zone: zone.value,
    })

  return {
    timezone,
    zone,
    zoneTitle,
    taskLabel,
    eventLabel,
    fieldLabel,
    entityLabel,
    changeEntityLabel,
    rowTime,
    preciseTime,
    fullTime,
  }
}
