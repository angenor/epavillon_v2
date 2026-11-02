<script setup lang="ts">
import type { JournalChange, JournalChangeFilters, JournalChangePage } from '~/types/admin-journal'
import type { SelectOption, TableColumn } from '~/types/ui'

/**
 * « Modifications » : ce qui a été écrit, par qui. Les valeurs sensibles sont
 * retirées par l'API ; un champ masqué n'affiche que son nom.
 */

const PER_PAGE = 50

const { t } = useI18n()
const api = useApi()
const route = useRoute()
const router = useRouter()
const labels = useJournalLabels()

const text = (value: unknown): string => (typeof value === 'string' ? value : '')

const filters = computed<JournalChangeFilters>(() => ({
  period: journalParam(JOURNAL_PERIOD_PARAM, route.query.periode) ?? '24h',
  entity: journalParam(JOURNAL_ENTITY_PARAM, route.query.objet),
  action: journalParam(JOURNAL_ACTION_PARAM, route.query.action),
  q: text(route.query.q).trim() || undefined,
  page: journalPageOf(route.query.page),
  per_page: PER_PAGE,
}))

const isFiltered = computed(() => !!(filters.value.entity || filters.value.action || filters.value.q))

const {
  data: page,
  status,
  error,
  refresh,
} = useAsyncData<JournalChangePage | null>(
  'admin-journal-changes',
  () => api.adminJournal.changes(filters.value),
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

const setFilter = (patch: Record<string, string | null>) => updateQuery({ ...patch, page: null })

const search = ref(text(route.query.q))
watch(() => route.query.q, (value) => (search.value = text(value)))

const periodOptions = computed<SelectOption[]>(() =>
  Object.keys(JOURNAL_PERIOD_PARAM).map((value) => ({ value, label: t(`admin.journal.period.${value}`) })),
)
const entityOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.journal.changes.entityFilter.all') },
  ...Object.keys(JOURNAL_ENTITY_PARAM).map((value) => ({ value, label: t(`admin.journal.changes.entityFilter.${value}`) })),
])
const actionOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.journal.changes.actionFilter.all') },
  ...Object.entries(JOURNAL_ACTION_PARAM).map(([value, action]) => ({ value, label: t(`admin.journal.action.${action}`) })),
])

const columns = computed<TableColumn[]>(() => [
  { key: 'occurred_at', label: labels.zoneTitle.value },
  { key: 'actor', label: t('admin.journal.changes.columns.actor'), hideOnMobile: true },
  { key: 'action', label: t('admin.journal.changes.columns.action'), width: '7rem' },
  { key: 'entity', label: t('admin.journal.changes.columns.entity') },
  { key: 'fields', label: t('admin.journal.changes.columns.fields'), hideBelow: 'lg' },
])

const rows = computed<JournalChange[]>(() => page.value?.items ?? [])
const selectedId = computed(() => text(route.query.ligne) || null)
const selected = computed(() => rows.value.find((row) => row.id === selectedId.value) ?? null)

function fieldsText(change: JournalChange): string {
  return change.fields
    .map((field) =>
      field.masked
        ? t('admin.journal.changes.maskedField', { name: labels.fieldLabel(field.name) })
        : labels.fieldLabel(field.name),
    )
    .join(', ')
}

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
        :label="t('admin.journal.changes.entityFilter.label')"
        hide-label
        hide-optional
        :model-value="text(route.query.objet)"
        :options="entityOptions"
        @update:model-value="setFilter({ objet: $event || null })"
      />
      <UiSelect
        class="w-full sm:w-56"
        :label="t('admin.journal.changes.actionFilter.label')"
        hide-label
        hide-optional
        :model-value="journalParamFor(JOURNAL_ACTION_PARAM, filters.action) ?? ''"
        :options="actionOptions"
        @update:model-value="setFilter({ action: $event || null })"
      />
      <UiSearchInput
        v-model="search"
        class="w-full sm:w-72"
        :label="t('admin.journal.search.label')"
        hide-label
        hide-optional
        :placeholder="t('admin.journal.search.changes')"
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
      :class="selected ? 'xl:grid-cols-[minmax(0,1fr)_25rem]' : ''"
    >
      <div class="min-w-0">
        <UiTable
          :columns="columns"
          :rows="rows"
          row-key="id"
          :caption="t('admin.journal.changes.caption')"
          visually-hidden-caption
          dense
          :loading="status === 'pending' && !page"
          :loading-rows="8"
          :selected="selectedId ? [selectedId] : []"
          @row-click="(row) => updateQuery({ ligne: row.id })"
        >
          <template #cell-occurred_at="{ row }">
            <span class="whitespace-nowrap tabular-nums text-text-secondary">{{ labels.rowTime(row.occurred_at) }}</span>
          </template>
          <template #cell-actor="{ row }">
            <span v-if="row.actor">{{ row.actor.display_name }}</span>
            <span v-else class="text-text-muted">{{ t('admin.journal.changes.system') }}</span>
          </template>
          <template #cell-action="{ row }">
            <UiBadge :intent="JOURNAL_ACTION_INTENT[row.action]" size="sm" solid :label="t(`admin.journal.action.${row.action}`)" />
          </template>
          <template #cell-entity="{ row }">
            <span>{{ labels.changeEntityLabel(row) }}</span>
            <span class="ml-1 font-mono text-xs whitespace-nowrap text-text-muted" :title="row.entity_id ?? undefined">{{ journalShortId(row.entity_id) }}</span>
          </template>
          <template #cell-fields="{ row }">
            <span v-if="row.fields.length" class="line-clamp-2">{{ fieldsText(row) }}</span>
            <span v-else class="text-text-muted">{{ t('admin.journal.changes.wholeRow') }}</span>
          </template>

          <template #empty>
            <UiEmptyState
              v-if="isFiltered"
              compact
              filtered
              :title="t('admin.journal.changes.noResults.title')"
              :description="t('admin.journal.changes.noResults.description')"
              :action-label="t('admin.journal.changes.noResults.action')"
              @action="setFilter({ objet: null, action: null, q: null })"
            />
            <UiEmptyState
              v-else
              compact
              icon="document"
              :title="t('admin.journal.changes.empty.title')"
              :description="t('admin.journal.changes.empty.description')"
            />
          </template>
        </UiTable>

        <p v-if="page" class="mt-3 text-sm text-text-secondary">
          {{ t('admin.journal.entries.count', { shown: rows.length, total: page.total }, page.total) }}
        </p>

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

      <AdminJournalChangeDetail v-if="selected" :change="selected" @close="updateQuery({ ligne: null })" />
    </div>
  </div>
</template>
