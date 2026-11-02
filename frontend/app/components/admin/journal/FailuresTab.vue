<script setup lang="ts">
import type { JournalFailedJob, JournalFailures, JournalJobState, JournalOutboxEvent } from '~/types/admin-journal'
import type { Intent, SelectOption, TableColumn } from '~/types/ui'

/**
 * « Tâches en échec » : ce qui attend encore, lu en direct (travaux de fond et
 * événements non relayés). Relancer remet la ligne en file ; le bouton n'est
 * montré qu'avec `identity.platform.maintenance` en portée globale, et un
 * refus de l'API s'affiche tel quel.
 */

const props = defineProps<{ canRetry: boolean }>()
const emit = defineEmits<{ changed: [] }>()

const { t } = useI18n()
const api = useApi()
const toast = useToast()
const labels = useJournalLabels()

const {
  data: failures,
  status,
  error,
  refresh,
} = useAsyncData<JournalFailures | null>('admin-journal-failures', () => api.adminJournal.failures(), {
  lazy: true,
  default: () => null,
})

const stateFilter = ref<'' | JournalJobState>('')
const taskFilter = ref('')

const jobs = computed(() =>
  (failures.value?.jobs ?? [])
    .filter((job) => !stateFilter.value || job.state === stateFilter.value)
    .filter((job) => !taskFilter.value || job.task === taskFilter.value),
)
const events = computed(() =>
  (failures.value?.events ?? []).filter((event) => !taskFilter.value || event.event_type === taskFilter.value),
)

const stateOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.journal.failures.filters.stateAll') },
  ...(['dead', 'failed', 'retrying'] as const).map((value) => ({ value, label: t(`admin.journal.failures.filters.state.${value}`) })),
])

/** Les natures présentes, et elles seules : un filtre qui ne rend jamais rien n'aide pas. */
const taskOptions = computed<SelectOption[]>(() => {
  const codes = new Map<string, string>()
  for (const job of failures.value?.jobs ?? []) codes.set(job.task, labels.taskLabel(job.task))
  for (const event of failures.value?.events ?? []) codes.set(event.event_type, labels.eventLabel(event.event_type))
  return [
    { value: '', label: t('admin.journal.failures.filters.taskAll') },
    ...[...codes].map(([value, label]) => ({ value, label })).sort((a, b) => a.label.localeCompare(b.label)),
  ]
})

const isEmpty = computed(() => !!failures.value && failures.value.jobs.length + failures.value.events.length === 0)

const jobColumns = computed<TableColumn[]>(() => [
  { key: 'last_failed_at', label: t('admin.journal.failures.columns.lastAttempt', { zone: labels.zone.value }), width: '9rem' },
  { key: 'task', label: t('admin.journal.failures.columns.task') },
  { key: 'attempts', label: t('admin.journal.failures.columns.attempts'), width: '12rem' },
  { key: 'last_error', label: t('admin.journal.failures.columns.error'), hideOnMobile: true },
  { key: 'actions', label: t('admin.journal.failures.columns.actions'), align: 'end' },
])

const eventColumns = computed<TableColumn[]>(() => [
  { key: 'occurred_at', label: t('admin.journal.failures.columns.occurred', { zone: labels.zone.value }), width: '9rem' },
  { key: 'event_type', label: t('admin.journal.failures.columns.task') },
  { key: 'attempts', label: t('admin.journal.failures.columns.attempts'), width: '12rem' },
  { key: 'last_error', label: t('admin.journal.failures.columns.error'), hideOnMobile: true },
  { key: 'actions', label: t('admin.journal.failures.columns.actions'), align: 'end' },
])

const JOB_INTENT: Record<JournalJobState, Intent> = { dead: 'danger', failed: 'danger', retrying: 'warning' }

function jobStateLabel(job: JournalFailedJob): string {
  return job.state === 'retrying'
    ? t('admin.journal.failures.state.retrying', { time: labels.rowTime(job.run_at) })
    : t(`admin.journal.failures.state.${job.state}`)
}

/** Un événement non relayé est remis en file à `available_at` : à venir, c'est un nouvel essai prévu. */
const eventRetriesLater = (event: JournalOutboxEvent): boolean => Date.parse(event.available_at) > Date.now()

const retrying = ref<string | null>(null)
const retryError = ref<string | null>(null)

async function retry(kind: 'job' | 'event', id: string, label: string): Promise<void> {
  retrying.value = id
  retryError.value = null
  try {
    await (kind === 'job' ? api.adminJournal.retryJob(id) : api.adminJournal.retryEvent(id))
    toast.show({ intent: 'success', title: t('admin.journal.failures.retried'), message: t('admin.journal.failures.retriedMessage', { label }) })
    emit('changed')
  } catch (thrown) {
    retryError.value = apiErrorMessage(thrown, (key) => t(key))
  } finally {
    retrying.value = null
    // Après un 409 aussi : la ligne a changé d'état entre-temps, la liste doit le montrer.
    await refresh()
  }
}

const errorMessage = computed(() => (error.value ? apiErrorMessage(error.value, (key) => t(key)) : ''))
</script>

<template>
  <div>
    <UiErrorState
      v-if="error"
      class="mt-2"
      :description="errorMessage"
      :retry-label="t('common.actions.retry')"
      @retry="refresh()"
    />

    <UiEmptyState
      v-else-if="isEmpty"
      class="mt-6"
      icon="check-circle"
      :title="t('admin.journal.failures.empty.title')"
      :description="t('admin.journal.failures.empty.description')"
    />

    <template v-else>
      <div class="flex flex-wrap items-end gap-3" role="group" :aria-label="t('admin.journal.filters.label')">
        <UiSelect
          v-model="stateFilter"
          class="w-full sm:w-56"
          :label="t('admin.journal.failures.filters.stateLabel')"
          hide-label
          hide-optional
          :options="stateOptions"
        />
        <UiSelect
          v-model="taskFilter"
          class="w-full sm:w-72"
          :label="t('admin.journal.failures.filters.taskLabel')"
          hide-label
          hide-optional
          :options="taskOptions"
        />
      </div>

      <UiAlert v-if="retryError" class="mt-4" intent="danger" live dismissible :message="retryError" @dismiss="retryError = null" />
      <p v-if="!props.canRetry" class="mt-3 text-sm text-text-muted">{{ t('admin.journal.failures.readOnly') }}</p>

      <section class="mt-6" aria-labelledby="journal-jobs-title">
        <h2 id="journal-jobs-title" class="text-lg font-semibold">{{ t('admin.journal.failures.jobs.title') }}</h2>
        <UiTable
          class="mt-3"
          :columns="jobColumns"
          :rows="jobs"
          row-key="id"
          :caption="t('admin.journal.failures.jobs.title')"
          visually-hidden-caption
          dense
          :hoverable="false"
          :loading="status === 'pending' && !failures"
          :loading-rows="3"
        >
          <template #cell-last_failed_at="{ row }">
            <span v-if="row.last_failed_at" class="whitespace-nowrap tabular-nums text-text-secondary">{{ labels.rowTime(row.last_failed_at) }}</span>
            <span v-else class="text-text-muted">{{ t('admin.journal.failures.notLogged') }}</span>
          </template>
          <template #cell-task="{ row }">
            <span class="block">{{ labels.taskLabel(row.task) }}</span>
            <span class="block font-mono text-xs text-text-muted">{{ row.queue }}</span>
          </template>
          <template #cell-attempts="{ row }">
            <span class="flex flex-wrap items-center gap-2">
              <span class="tabular-nums">{{ t('admin.journal.failures.attemptsOf', { attempts: row.attempts, max: row.max_attempts }) }}</span>
              <UiBadge :intent="JOB_INTENT[row.state]" size="sm" solid :label="jobStateLabel(row)" />
            </span>
          </template>
          <template #cell-last_error="{ row }">
            <span class="line-clamp-3 text-sm">{{ row.last_error }}</span>
          </template>
          <template #cell-actions="{ row }">
            <UiButton
              v-if="props.canRetry"
              variant="secondary"
              icon="refresh"
              :loading="retrying === row.id"
              :disabled="retrying !== null && retrying !== row.id"
              @click="retry('job', row.id, labels.taskLabel(row.task))"
            >
              {{ row.state === 'retrying' ? t('admin.journal.failures.retryNow') : t('admin.journal.failures.retry') }}
            </UiButton>
          </template>
          <template #empty>
            <p class="text-center text-sm text-text-muted">{{ t('admin.journal.failures.jobs.empty') }}</p>
          </template>
        </UiTable>
      </section>

      <section class="mt-8" aria-labelledby="journal-events-title">
        <h2 id="journal-events-title" class="text-lg font-semibold">{{ t('admin.journal.failures.events.title') }}</h2>
        <p class="mt-1 text-sm text-text-muted">{{ t('admin.journal.failures.events.subtitle') }}</p>
        <UiTable
          class="mt-3"
          :columns="eventColumns"
          :rows="events"
          row-key="id"
          :caption="t('admin.journal.failures.events.title')"
          visually-hidden-caption
          dense
          :hoverable="false"
          :loading="status === 'pending' && !failures"
          :loading-rows="2"
        >
          <template #cell-occurred_at="{ row }">
            <span class="whitespace-nowrap tabular-nums text-text-secondary">{{ labels.rowTime(row.occurred_at) }}</span>
          </template>
          <template #cell-event_type="{ row }">
            <span class="block">{{ labels.eventLabel(row.event_type) }}</span>
            <span class="block font-mono text-xs text-text-muted" :title="row.aggregate_id">
              {{ row.aggregate_schema }}.{{ row.aggregate_type }} {{ journalShortId(row.aggregate_id) }}
            </span>
          </template>
          <template #cell-attempts="{ row }">
            <span class="flex flex-wrap items-center gap-2">
              <span class="tabular-nums">{{ t('admin.journal.failures.attempts', row.attempts) }}</span>
              <UiBadge
                :intent="eventRetriesLater(row) ? 'warning' : 'danger'"
                size="sm"
                solid
                :label="
                  eventRetriesLater(row)
                    ? t('admin.journal.failures.state.retrying', { time: labels.rowTime(row.available_at) })
                    : t('admin.journal.failures.state.failed')
                "
              />
            </span>
          </template>
          <template #cell-last_error="{ row }">
            <span class="line-clamp-3 text-sm">{{ row.last_error }}</span>
          </template>
          <template #cell-actions="{ row }">
            <UiButton
              v-if="props.canRetry"
              variant="secondary"
              icon="refresh"
              :loading="retrying === row.id"
              :disabled="retrying !== null && retrying !== row.id"
              @click="retry('event', row.id, labels.eventLabel(row.event_type))"
            >
              {{ eventRetriesLater(row) ? t('admin.journal.failures.retryNow') : t('admin.journal.failures.retry') }}
            </UiButton>
          </template>
          <template #empty>
            <p class="text-center text-sm text-text-muted">{{ t('admin.journal.failures.events.empty') }}</p>
          </template>
        </UiTable>
      </section>
    </template>
  </div>
</template>
