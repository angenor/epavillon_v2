<script setup lang="ts">
import type { EditionListFilters, EditionSeriesOption } from '~/types/admin-events'
import type { EventStatus } from '~/types/event/edition'
import type { SelectOption } from '~/types/ui'

/**
 * Filtres de la liste des éditions.
 *
 * L'ÉTAT N'EST PAS ICI. Le composant reçoit les filtres et émet le prochain jeu ;
 * la page les tient dans l'URL. Même partage qu'en A7, et pour la même raison :
 * une liste filtrée se transmet par courriel, et le jour où le filtrage part au
 * serveur, ces paramètres deviennent ceux de la requête sans qu'un composant
 * change.
 */

interface Props {
  filters: EditionListFilters
  series: EditionSeriesOption[]
  years: number[]
  total: number
  shown: number
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:filters': [value: EditionListFilters] }>()

const { t } = useI18n()
const { tr } = useI18nText()

const STATUSES: EventStatus[] = [
  'draft',
  'announced',
  'ongoing',
  'completed',
  'suspended',
  'cancelled',
]

const seriesOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...props.series.map((entry) => ({
    value: entry.id,
    label: tr(entry.name),
    description: t('admin.event.list.seriesKind.' + entry.kind),
  })),
])

const yearOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...props.years.map((year) => ({ value: String(year), label: String(year) })),
])

const statusOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...STATUSES.map((status) => ({
    value: status,
    label: t('admin.event.list.status.' + status),
  })),
])

/** Trois états et non deux : « sans importance » n'est pas « non ». */
const TRISTATE = ['', 'yes', 'no'] as const

const pavilionOptions = computed<SelectOption[]>(() =>
  TRISTATE.map((value) => ({
    value,
    label: t(
      value === ''
        ? 'admin.event.list.filters.pavilionAny'
        : value === 'yes'
          ? 'admin.event.list.filters.pavilionYes'
          : 'admin.event.list.filters.pavilionNo',
    ),
  })),
)

const publishedOptions = computed<SelectOption[]>(() =>
  TRISTATE.map((value) => ({
    value,
    label: t(
      value === ''
        ? 'admin.event.list.filters.publishedAny'
        : value === 'yes'
          ? 'admin.event.list.filters.publishedYes'
          : 'admin.event.list.filters.publishedNo',
    ),
  })),
)

function tristate(value: boolean | null): string {
  return value === null ? '' : value ? 'yes' : 'no'
}

function patch(next: Partial<EditionListFilters>): void {
  emit('update:filters', { ...props.filters, ...next })
}
</script>

<template>
  <fieldset
    class="rounded-lg border border-border bg-surface-raised p-4"
    :disabled="props.disabled"
  >
    <legend class="sr-only">{{ t('admin.event.list.filters.legend') }}</legend>

    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-6">
      <div class="lg:col-span-2">
        <UiSearchInput
          :model-value="props.filters.search"
          :label="t('admin.event.list.filters.search')"
          :placeholder="t('admin.event.list.filters.search')"
          @update:model-value="(next: string) => patch({ search: next })"
        />
      </div>

      <UiSelect
        :model-value="props.filters.series[0] ?? ''"
        :label="t('admin.event.list.filters.series')"
        :options="seriesOptions"
        hide-optional
        @update:model-value="(next: string) => patch({ series: next ? [next] : [] })"
      />

      <UiSelect
        :model-value="props.filters.years[0] ? String(props.filters.years[0]) : ''"
        :label="t('admin.event.list.filters.years')"
        :options="yearOptions"
        hide-optional
        @update:model-value="(next: string) => patch({ years: next ? [Number(next)] : [] })"
      />

      <UiSelect
        :model-value="props.filters.statuses[0] ?? ''"
        :label="t('admin.event.list.filters.statuses')"
        :options="statusOptions"
        hide-optional
        @update:model-value="
          (next: string) => patch({ statuses: next ? [next as EventStatus] : [] })
        "
      />

      <UiSelect
        :model-value="tristate(props.filters.has_pavilion)"
        :label="t('admin.event.list.filters.pavilion')"
        :options="pavilionOptions"
        hide-optional
        @update:model-value="
          (next: string) => patch({ has_pavilion: next === '' ? null : next === 'yes' })
        "
      />

      <UiSelect
        :model-value="tristate(props.filters.published)"
        :label="t('admin.event.list.filters.published')"
        :options="publishedOptions"
        hide-optional
        @update:model-value="
          (next: string) => patch({ published: next === '' ? null : next === 'yes' })
        "
      />
    </div>

    <p class="mt-3 text-sm text-text-muted" aria-live="polite">
      {{ t('admin.event.list.filters.shown', { shown: props.shown, total: props.total }) }}
    </p>
  </fieldset>
</template>
