<script setup lang="ts">
import type { ScheduledMailScreen } from '~/types/admin-scheduled-mails'
import type { TabItem } from '~/types/ui'

/**
 * LES ONGLETS DE COURRIELS (04/11) : Envois, Gabarits, Textes, Programmés.
 * Des liens, chaque volet ayant son adresse. La pastille de « Programmés »
 * compte les courriels en file de l'édition : la page des programmés la donne,
 * les autres la lisent (même clé de cache que cette page).
 */
type Section = 'sent' | 'templates' | 'texts' | 'scheduled'

const props = defineProps<{
  current: Section
  /** Le nombre déjà connu de la page ; sinon, il se lit. */
  scheduledCount?: number | null
}>()

const { t } = useI18n()
const api = useApi()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()

const PATHS: Record<Section, string> = {
  sent: '/admin/courriels',
  templates: '/admin/courriels/gabarits',
  texts: '/admin/courriels/textes',
  scheduled: '/admin/courriels/programmes',
}

const { data: screen } = useAsyncData<ScheduledMailScreen | null>(
  'admin-scheduled-mails-count',
  async () => {
    const eventId = adminScope.currentEventId
    if (props.scheduledCount !== undefined || !eventId) return null
    try {
      return await api.adminMailings.scheduled.list(eventId, adminScope.scope)
    } catch {
      return null
    }
  },
  { watch: [() => adminScope.currentEventId], lazy: true, server: false },
)

const count = computed<number | undefined>(() => {
  if (props.scheduledCount !== undefined) return props.scheduledCount ?? undefined
  return screen.value?.total
})

const items = computed<TabItem[]>(() =>
  (['sent', 'templates', 'texts', 'scheduled'] as const).map((value) => {
    const item: TabItem = { value, label: t(`admin-mailing-section-tabs.${value}`) }
    if (value === 'scheduled' && count.value !== undefined) item.count = count.value
    return item
  }),
)
</script>

<template>
  <UiTabs
    :model-value="props.current"
    :items="items"
    :label="t('admin-mailing-section-tabs.label')"
    :to="(item) => localePath(PATHS[item.value as Section])"
  />
</template>
