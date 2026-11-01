<script setup lang="ts">
import type { ManagedIncident } from '~/types/admin-incidents'
import type { IncidentSeverity } from '~/types/live'
import type { TimeZoneName } from '~/types/shared'

interface Props {
  rows: ManagedIncident[]
  timezone: TimeZoneName
  canPublish: boolean
  busy: string | null
  loading?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ unpublish: [row: ManagedIncident] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()

/** Jaune attention, rouge échec : la règle de couleur du guide. */
const DOTS: Record<IncidentSeverity, string> = {
  info: 'bg-info',
  warning: 'bg-warning-solid',
  error: 'bg-live',
  critical: 'bg-live',
}
</script>

<template>
  <section class="rounded-lg border border-border-subtle bg-surface-raised shadow-sm" aria-labelledby="regie-incidents">
    <div class="flex items-center justify-between gap-3 border-b border-border-subtle px-4 py-3.5 sm:px-5">
      <h2 id="regie-incidents" class="heading-condensed text-[1.35rem]">{{ t('admin.regie.incidents.title') }}</h2>
      <span class="text-sm text-text-muted">{{ t('admin.regie.incidents.count', { count: props.rows.length }, props.rows.length) }}</span>
    </div>
    <div v-if="props.loading" class="px-4 py-3 sm:px-5"><UiSkeletonLoader variant="text" :lines="2" /></div>
    <ul v-else-if="props.rows.length" class="m-0 list-none p-0">
      <li
        v-for="row in props.rows"
        :key="row.incident_id"
        class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-start gap-2.5 border-b border-border-subtle px-4 py-3 sm:px-5"
      >
        <span class="mt-1.5 size-2.5 rounded-full" :class="DOTS[row.severity]" aria-hidden="true" />
        <div class="min-w-0">
          <p class="text-sm">{{ tr(row.message) }}</p>
          <small class="mt-0.5 block text-xs text-text-muted">
            {{ t('admin.regie.incidents.since', { target: row.target_label ?? t(`admin.regie.incidents.scope.${row.scope}`), time: dt.time(row.published_at ?? row.display_from, props.timezone) }) }}
          </small>
        </div>
        <UiButton
          v-if="props.canPublish"
          size="sm"
          variant="ghost"
          :loading="props.busy === `unpublish:${row.incident_id}`"
          @click="emit('unpublish', row)"
        >
          {{ t('common.actions.remove') }}
        </UiButton>
        <span v-else />
      </li>
    </ul>
    <p v-else class="px-4 py-3 text-sm text-text-muted sm:px-5">{{ t('admin.regie.incidents.empty') }}</p>
    <slot />
  </section>
</template>
