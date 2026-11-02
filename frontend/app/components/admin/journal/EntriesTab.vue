<script setup lang="ts">
import type { JournalEntry, JournalEntryFilters, JournalEntryPage } from '~/types/admin-journal'
import type { SelectOption, TableColumn } from '~/types/ui'

/**
 * « Erreurs et refus » : ce qui a échoué, été refusé ou traîné.
 *
 * Tout l'état vit dans l'URL (filtres, page, ligne ouverte, code de demande) :
 * une adresse copiée dans un message montre la même ligne à la personne qui
 * la reçoit.
 */

const PER_PAGE = 50

const { t } = useI18n()
const api = useApi()
const route = useRoute()
const router = useRouter()
const apiUrl = useApiUrl()
const labels = useJournalLabels()

const text = (value: unknown): string => (typeof value === 'string' ? value : '')

const filters = computed<JournalEntryFilters>(() => ({
  period: journalParam(JOURNAL_PERIOD_PARAM, route.query.periode) ?? '24h',
  kind: journalParam(JOURNAL_KIND_PARAM, route.query.gravite),
  source: journalParam(JOURNAL_SOURCE_PARAM, route.query.origine),
  q: text(route.query.q).trim() || undefined,
  page: journalPageOf(route.query.page),
  per_page: PER_PAGE,
}))

const isFiltered = computed(() => !!(filters.value.kind || filters.value.source || filters.value.q))

const {
  data: page,
  status,
  error,
  refresh,
} = useAsyncData<JournalEntryPage | null>(
  'admin-journal-entries',
  () => api.adminJournal.entries(filters.value),
  { watch: [filters], lazy: true, default: () => null },
)

function updateQuery(patch: Record<string, string | null>): void {
  const next = { ...route.query }
  for (const [key, value] of Object.entries(patch)) {
    if (value === null || value === '') delete next[key]
    else next[key] = value
  }
  router.replace({ query: next })
}

/** Un filtre changé ramène en page 1 : la page 3 d'une autre liste n'a pas de sens. */
const setFilter = (patch: Record<string, string | null>) => updateQuery({ ...patch, page: null })

const search = ref(text(route.query.q))
watch(() => route.query.q, (value) => (search.value = text(value)))

const periodOptions = computed<SelectOption[]>(() =>
  Object.keys(JOURNAL_PERIOD_PARAM).map((value) => ({ value, label: t(`admin.journal.period.${value}`) })),
)
const kindOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.journal.kindFilter.all') },
  ...Object.entries(JOURNAL_KIND_PARAM).map(([value, kind]) => ({ value, label: t(`admin.journal.kindFilter.${kind}`) })),
])
const sourceOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.journal.source.all') },
  ...Object.entries(JOURNAL_SOURCE_PARAM).map(([value, source]) => ({ value, label: t(`admin.journal.source.${source}`) })),
])

const selectedId = computed(() => text(route.query.ligne) || null)
const requestId = computed(() => text(route.query.demande).trim() || null)
const isDetailOpen = computed(() => !!(selectedId.value || requestId.value))

const columns = computed<TableColumn[]>(() => [
  { key: 'occurred_at', label: labels.zoneTitle.value },
  { key: 'kind', label: t('admin.journal.entries.columns.kind') },
  // Le détail ouvert redit la route : elle cède sa place au message tant que l'écran est étroit.
  { key: 'route', label: t('admin.journal.entries.columns.route'), ...(isDetailOpen.value ? { hideBelow: '2xl' as const } : { hideOnMobile: true }) },
  { key: 'actor', label: t('admin.journal.entries.columns.actor'), hideBelow: 'lg' },
  { key: 'message', label: t('admin.journal.entries.columns.message') },
])

const rows = computed<JournalEntry[]>(() => page.value?.items ?? [])

const selectedEntry = computed(() => rows.value.find((row) => row.id === selectedId.value) ?? null)
function select(row: JournalEntry): void {
  updateQuery({ ligne: row.id, demande: row.request_id })
}

const closeDetail = () => updateQuery({ ligne: null, demande: null })

const csvHref = computed(() =>
  apiUrl(
    journalCsvPath({
      period: filters.value.period,
      kind: filters.value.kind,
      source: filters.value.source,
      q: filters.value.q,
    }),
  ),
)

const errorMessage = computed(() => (error.value ? apiErrorMessage(error.value, (key) => t(key)) : ''))
</script>

<template>
  <div>
    <div class="flex flex-wrap items-end gap-3" role="group" :aria-label="t('admin.journal.filters.label')">
      <UiSelect
        class="w-full sm:w-48"
        :label="t('admin.journal.period.label')"
        hide-label
        hide-optional
        :model-value="journalParamFor(JOURNAL_PERIOD_PARAM, filters.period) ?? '24h'"
        :options="periodOptions"
        @update:model-value="setFilter({ periode: $event === '24h' ? null : $event })"
      />
      <UiSelect
        class="w-full sm:w-48"
        :label="t('admin.journal.kindFilter.label')"
        hide-label
        hide-optional
        :model-value="journalParamFor(JOURNAL_KIND_PARAM, filters.kind) ?? ''"
        :options="kindOptions"
        @update:model-value="setFilter({ gravite: $event || null })"
      />
      <UiSelect
        class="w-full sm:w-48"
        :label="t('admin.journal.source.label')"
        hide-label
        hide-optional
        :model-value="journalParamFor(JOURNAL_SOURCE_PARAM, filters.source) ?? ''"
        :options="sourceOptions"
        @update:model-value="setFilter({ origine: $event || null })"
      />
      <UiSearchInput
        v-model="search"
        class="w-full sm:w-80"
        :label="t('admin.journal.search.label')"
        hide-label
        hide-optional
        :placeholder="t('admin.journal.search.entries')"
        @search="setFilter({ q: $event || null })"
      />
    </div>

    <UiErrorState
      v-if="error"
      class="mt-6"
      :description="errorMessage"
      :retry-label="t('common.actions.retry')"
      @retry="refresh()"
    />

    <div
      v-else
      class="mt-4 grid items-start gap-5"
      :class="isDetailOpen ? 'xl:grid-cols-[minmax(0,1fr)_25rem]' : ''"
    >
      <div class="min-w-0">
        <UiTable
          :columns="columns"
          :rows="rows"
          row-key="id"
          :caption="t('admin.journal.entries.caption')"
          visually-hidden-caption
          dense
          :loading="status === 'pending' && !page"
          :loading-rows="8"
          :selected="selectedId ? [selectedId] : []"
          @row-click="select"
        >
          <template #cell-occurred_at="{ row }">
            <span class="whitespace-nowrap tabular-nums text-text-secondary">{{ labels.rowTime(row.occurred_at) }}</span>
          </template>
          <template #cell-kind="{ row }">
            <AdminJournalKindBadge :kind="row.kind" />
          </template>
          <template #cell-route="{ row }">
            <code v-if="row.route" class="font-mono text-xs">{{ row.method }} {{ row.route }}</code>
            <span v-else-if="row.task" class="text-sm">{{ labels.taskLabel(row.task) }}</span>
          </template>
          <template #cell-actor="{ row }">
            <template v-if="row.actor">
              <span class="block">{{ row.actor.display_name }}</span>
              <span v-if="row.actor.organization_name" class="block text-xs text-text-muted">{{ row.actor.organization_name }}</span>
            </template>
            <span v-else class="text-text-muted">
              {{ row.source === 'worker' ? t('admin.journal.entries.worker') : t('admin.journal.entries.visitor') }}
            </span>
          </template>
          <template #cell-message="{ row }">
            <span class="line-clamp-2">{{ row.message }}</span>
          </template>

          <template #empty>
            <UiEmptyState
              v-if="isFiltered"
              compact
              filtered
              :title="t('admin.journal.entries.noResults.title')"
              :description="t('admin.journal.entries.noResults.description')"
              :action-label="t('admin.journal.entries.noResults.action')"
              @action="setFilter({ gravite: null, origine: null, q: null })"
            />
            <UiEmptyState
              v-else
              compact
              icon="check-circle"
              :title="t('admin.journal.entries.empty.title')"
              :description="t('admin.journal.entries.empty.description')"
            />
          </template>
        </UiTable>

        <div class="mt-3 flex flex-wrap items-center justify-between gap-3 text-sm text-text-secondary">
          <span v-if="page">{{ t('admin.journal.entries.count', { shown: rows.length, total: page.total }, page.total) }}</span>
          <UiButton v-if="csvHref" variant="secondary" icon="download" :href="csvHref">
            {{ t('admin.journal.entries.export') }}
          </UiButton>
          <span v-else class="text-text-muted">{{ t('admin.journal.entries.exportOffline') }}</span>
        </div>

        <UiPagination
          v-if="page && page.total > page.per_page"
          class="mt-3"
          :page="page.page"
          :per-page="page.per_page"
          :total="page.total"
          :disabled="status === 'pending'"
          @update:page="updateQuery({ page: $event > 1 ? String($event) : null })"
        />
      </div>

      <AdminJournalEntryDetail
        v-if="isDetailOpen"
        :entry="selectedEntry"
        :request-id="requestId"
        @close="closeDetail"
      />
    </div>
  </div>
</template>
