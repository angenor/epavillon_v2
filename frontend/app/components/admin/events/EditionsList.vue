<script setup lang="ts">
import type { EditionListRow, EditionSortKey } from '~/types/admin-events'
import type { SelectOption, SortDirection } from '~/types/ui'
import type { EventStatus } from '~/types/event/edition'

/**
 * La liste des éditions, en rangées composées comme celle des propositions : le
 * titre porte la ligne, ce qui situe l'édition passe en métadonnées, ce qui sert
 * à piloter (dossiers, programmation) garde une colonne alignée à droite.
 *
 * Chaque rangée porte les dates dans le fuseau de SON édition, et le dit.
 */

interface Props {
  rows: EditionListRow[]
  label: string
  sortKey: EditionSortKey
  sortDirection: Exclude<SortDirection, null>
  loading?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  sort: [key: EditionSortKey, direction: Exclude<SortDirection, null>]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, zoneLabel } = useDateTime()
const localePath = useLocalePath()

const SORT_KEYS: EditionSortKey[] = [
  'starts_at',
  'title',
  'series',
  'edition_year',
  'location',
  'status',
  'proposal_count',
  'programme',
]

const sortOptions = computed<SelectOption[]>(() =>
  SORT_KEYS.map((key) => ({ value: key, label: t(`admin.event.list.columns.${key}`) })),
)

// « En cours » est jaune : il demande de l'attention, ce n'est pas une réussite.
const STATUS_TONE: Record<EventStatus, string> = {
  draft: 'text-neutral bg-neutral-surface',
  announced: 'text-info bg-info-surface',
  ongoing: 'text-warning bg-warning-surface',
  completed: 'text-neutral bg-neutral-surface',
  cancelled: 'text-danger bg-danger-surface',
  suspended: 'text-warning bg-warning-surface',
}

function periodOf(row: EditionListRow): string {
  return t('common.datetime.dateRange', {
    start: date(row.starts_at, row.timezone),
    end: date(row.ends_at, row.timezone),
  })
}

function placeOf(row: EditionListRow): string {
  if (!row.city) return t('admin.event.list.cell.online')
  return [row.city, tr(row.country_name)].filter(Boolean).join(', ')
}

function scheduledPercent(row: EditionListRow): number {
  return row.session_count === 0 ? 0 : Math.round((row.scheduled_session_count / row.session_count) * 100)
}
</script>

<template>
  <div class="overflow-hidden rounded-lg border border-border bg-surface-raised">
    <div class="flex flex-wrap items-center gap-3 border-b border-separator bg-surface-sunken px-4 py-3">
      <div class="flex items-center gap-1.5">
        <UiSelect
          :model-value="props.sortKey"
          :options="sortOptions"
          :label="t('admin.event.list.sort.label')"
          hide-label
          hide-optional
          size="sm"
          @update:model-value="(value: string) => emit('sort', value as EditionSortKey, props.sortDirection)"
        />
        <UiButton
          variant="ghost"
          size="sm"
          icon-only
          :icon="props.sortDirection === 'desc' ? 'sort-desc' : 'sort-asc'"
          :label="t(`admin.event.list.sort.${props.sortDirection}`)"
          @click="emit('sort', props.sortKey, props.sortDirection === 'desc' ? 'asc' : 'desc')"
        />
      </div>

      <slot name="toolbar" />
    </div>

    <ul v-if="props.loading" :aria-label="props.label" aria-busy="true">
      <li
        v-for="index in 5"
        :key="index"
        class="grid gap-4 border-b border-border-subtle px-4 py-4 last:border-0 md:grid-cols-[minmax(0,1fr)_16rem]"
      >
        <div class="flex gap-4">
          <div class="aspect-video w-24 shrink-0 sm:w-36">
            <UiSkeletonLoader height="100%" />
          </div>
          <div class="flex-1 space-y-2">
            <UiSkeletonLoader width="6rem" height="1.25rem" />
            <UiSkeletonLoader width="70%" height="1rem" />
            <UiSkeletonLoader width="45%" height="0.8rem" />
          </div>
        </div>
        <UiSkeletonLoader class="hidden md:block" width="100%" height="2.5rem" />
      </li>
    </ul>

    <div v-else-if="props.rows.length === 0" class="px-4 py-10">
      <slot name="empty" />
    </div>

    <ul v-else :aria-label="props.label">
      <li
        v-for="row in props.rows"
        :key="row.id"
        class="group relative grid grid-cols-1 gap-x-6 gap-y-3 border-b border-border-subtle px-4 py-4 transition-colors duration-(--duration-fast) last:border-0 hover:bg-surface-hover md:grid-cols-[minmax(0,1fr)_16rem]"
      >
        <div class="flex min-w-0 gap-4">
          <div class="aspect-video w-24 shrink-0 self-start overflow-hidden rounded-md border border-border-subtle bg-surface-sunken sm:w-36">
            <UiImage
              v-if="row.cover"
              :image="row.cover"
              ratio="16 / 9"
              sizes="144px"
              rounded="0"
            />
            <div v-else class="flex size-full items-center justify-center text-text-subtle">
              <UiIcon name="calendar" size="1.5rem" />
              <span class="sr-only">{{ t('admin.event.list.row.noImage') }}</span>
            </div>
          </div>

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span
                class="inline-flex items-center rounded-sm px-2 py-0.5 text-[0.6875rem] font-semibold tracking-caps uppercase"
                :class="STATUS_TONE[row.status]"
              >
                {{ t(`admin.event.list.status.${row.status}`) }}
              </span>
              <span class="text-xs text-text-subtle">
                {{ t(row.has_pavilion ? 'admin.event.list.cell.pavilion' : 'admin.event.list.cell.noPavilion') }}
              </span>
            </div>

            <h3 class="mt-1.5 text-base leading-snug font-semibold text-heading">
              <!-- Le lien s'étend à toute la rangée. -->
              <NuxtLink
                :to="localePath(`/admin/evenements/${row.id}`)"
                class="clamp-2 rounded-sm text-heading no-underline outline-none after:absolute after:inset-0 group-hover:text-accent focus-visible:ring-2 focus-visible:ring-focus"
                :title="tr(row.title)"
              >
                {{ tr(row.title) }}
              </NuxtLink>
            </h3>

            <p class="mt-1.5 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-text-muted">
              <span v-if="row.acronym || row.edition_label" class="font-mono text-text-secondary">
                {{ row.acronym ?? row.edition_label }} · {{ row.edition_year }}
              </span>
              <span v-if="row.series_name" class="inline-flex min-w-0 items-center gap-1.5">
                <UiIcon name="grid" size="0.875rem" class="shrink-0 text-text-subtle" />
                <span class="truncate">{{ tr(row.series_name) }}</span>
              </span>
              <span class="inline-flex items-start gap-1.5">
                <UiIcon name="calendar" size="0.875rem" class="mt-0.5 shrink-0 text-text-subtle" />
                <span>
                  {{ periodOf(row) }}
                  <span class="text-xs text-text-subtle">({{ zoneLabel(row.timezone, row.city ?? undefined) }})</span>
                </span>
              </span>
              <span class="inline-flex items-center gap-1.5">
                <UiIcon :name="row.city ? 'map-pin' : 'monitor'" size="0.875rem" class="text-text-subtle" />
                {{ placeOf(row) }}
              </span>
              <span class="inline-flex items-center gap-1.5 whitespace-nowrap">
                <UiIcon name="clock" size="0.875rem" class="text-text-subtle" />
                {{ t('admin.event.list.cell.days', row.day_count) }}
              </span>
            </p>
          </div>
        </div>

        <div class="grid grid-cols-[5.5rem_minmax(0,1fr)] gap-6 border-t border-border-subtle pt-3 md:border-0 md:pt-0">
          <div>
            <template v-if="row.call_status">
              <p class="font-display text-2xl leading-none font-semibold text-heading tabular-nums">
                {{ row.proposal_count }}
              </p>
              <p class="mt-1 text-xs text-text-muted">{{ t('admin.event.list.row.proposals', row.proposal_count) }}</p>
              <p class="text-xs text-text-subtle">{{ t(`admin.event.list.callStatus.${row.call_status}`) }}</p>
            </template>
            <p v-else class="text-xs text-text-subtle">{{ t('admin.event.list.cell.noCall') }}</p>
          </div>

          <div class="min-w-0">
            <p class="text-[0.6875rem] font-semibold tracking-caps text-text-subtle uppercase">
              {{ t('admin.event.list.columns.programme') }}
            </p>
            <div class="mt-1.5 h-1.5 overflow-hidden rounded-full bg-border" aria-hidden="true">
              <div
                class="h-full rounded-full"
                :class="row.programme_published_at ? 'bg-success-solid' : 'bg-accent-solid'"
                :style="{ width: `${scheduledPercent(row)}%` }"
              />
            </div>
            <p class="mt-1.5 text-xs text-text-secondary tabular-nums">
              {{ t('admin.event.list.cell.sessions', { scheduled: row.scheduled_session_count, total: row.session_count }) }}
            </p>
            <p
              class="mt-0.5 text-xs font-medium"
              :class="row.programme_published_at ? 'text-success' : 'text-text-subtle'"
            >
              {{
                row.programme_published_at
                  ? t('admin.event.list.programme.published', { date: date(row.programme_published_at, row.timezone) })
                  : t('admin.event.list.programme.unpublished')
              }}
            </p>
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
/* Les utilitaires `line-clamp-*` ne sont pas générés dans ce projet. */
.clamp-2 {
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}
</style>
