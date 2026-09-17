<script setup lang="ts">
import type { ProposalDashboardRow } from '~/types/views'
import type { ProposalSortKey } from '~/types/admin-proposals'
import type { SelectOption, SortDirection } from '~/types/ui'
import type { TimeZoneName, Uuid } from '~/types/shared'

/**
 * La liste des propositions, en rangées composées plutôt qu'en tableau : le
 * titre porte la ligne, ce qui situe le dossier passe en métadonnées, ce qui
 * sert à décider (revues, note) garde une colonne alignée à droite.
 */

interface Props {
  rows: ProposalDashboardRow[]
  unreadIds: Set<Uuid>
  timezone: TimeZoneName
  requiredReviews: number | null
  label: string
  sortKey: ProposalSortKey
  sortDirection: Exclude<SortDirection, null>
  selected: string[]
  loading?: boolean
  selectable?: boolean
  /** `programme.proposal.edit` sur l'édition affichée. */
  canEdit?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  sort: [key: ProposalSortKey, direction: Exclude<SortDirection, null>]
  'update:selected': [keys: string[]]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()
const localePath = useLocalePath()

const SORT_LABEL: Partial<Record<ProposalSortKey, string>> = {
  average_score: 'score',
  event_rank: 'rank',
  title: 'title',
  organization: 'organization',
  country: 'country',
  format: 'format',
  status: 'status',
  reviews: 'reviews',
  submitted_at: 'submitted',
}

const sortOptions = computed<SelectOption[]>(() =>
  Object.entries(SORT_LABEL).map(([value, key]) => ({ value, label: t(`admin.proposals.columns.${key}`) })),
)

// « Annulé » est violet comme le report : la décision est prise, elle n'attend plus rien.
const STATUS_TONE: Record<ProposalDashboardRow['status'], string> = {
  draft: 'text-neutral bg-neutral-surface',
  submitted: 'text-info bg-info-surface',
  under_review: 'text-warning bg-warning-surface',
  changes_requested: 'text-warning bg-warning-surface',
  accepted: 'text-success bg-success-surface',
  rejected: 'text-danger bg-danger-surface',
  withdrawn: 'text-neutral bg-neutral-surface',
  cancelled: 'text-postponed bg-postponed-surface',
}

// Un brouillon reste à son organisation ; un dossier clos ne se corrige plus.
const NOT_EDITABLE: ProposalDashboardRow['status'][] = ['draft', 'rejected', 'withdrawn', 'cancelled']

function isEditable(row: ProposalDashboardRow): boolean {
  return Boolean(props.canEdit) && !NOT_EDITABLE.includes(row.status)
}

function expectedReviews(row: ProposalDashboardRow): number {
  return row.required_reviews ?? props.requiredReviews ?? row.assigned_reviewers
}

function reviewSegments(row: ProposalDashboardRow): boolean[] {
  const total = Math.min(Math.max(expectedReviews(row), row.review_count), 8)
  return Array.from({ length: total }, (_, index) => index < row.review_count)
}

function pendingReviewers(row: ProposalDashboardRow): string {
  return row.reviewers
    .filter((reviewer) => reviewer.submitted_at === null)
    .map((reviewer) => reviewer.name)
    .join(', ')
}

const rowGrid = computed(() =>
  props.selectable
    ? 'grid-cols-[auto_minmax(0,1fr)] md:grid-cols-[auto_minmax(0,1fr)_auto]'
    : 'grid-cols-1 md:grid-cols-[minmax(0,1fr)_auto]',
)

const selectedSet = computed(() => new Set(props.selected))
const allSelected = computed(() => props.rows.length > 0 && props.rows.every((row) => selectedSet.value.has(row.id)))
const someSelected = computed(() => !allSelected.value && props.rows.some((row) => selectedSet.value.has(row.id)))

function toggleRow(id: string, checked: boolean): void {
  const next = new Set(props.selected)
  if (checked) next.add(id)
  else next.delete(id)
  emit('update:selected', [...next])
}

// « Tout sélectionner » ne touche que les rangées affichées, jamais les pages suivantes.
function toggleAll(checked: boolean): void {
  const next = new Set(props.selected)
  for (const row of props.rows) {
    if (checked) next.add(row.id)
    else next.delete(row.id)
  }
  emit('update:selected', [...next])
}
</script>

<template>
  <div class="overflow-hidden rounded-lg border border-border bg-surface-raised">
    <div class="flex flex-wrap items-center gap-3 border-b border-separator bg-surface-sunken px-4 py-3">
      <div v-if="props.selectable" class="proposal-check">
        <UiCheckbox
          :model-value="allSelected"
          :indeterminate="someSelected"
          :label="t('data.table.selectAll')"
          :disabled="props.loading || props.rows.length === 0"
          @update:model-value="toggleAll"
        />
      </div>

      <div class="flex items-center gap-1.5">
        <UiSelect
          :model-value="props.sortKey"
          :options="sortOptions"
          :label="t('admin.proposals.sort.label')"
          hide-label
          hide-optional
          size="sm"
          @update:model-value="(value: string) => emit('sort', value as ProposalSortKey, props.sortDirection)"
        />
        <UiButton
          variant="ghost"
          size="sm"
          icon-only
          :icon="props.sortDirection === 'desc' ? 'sort-desc' : 'sort-asc'"
          :label="t(`admin.proposals.sort.${props.sortDirection}`)"
          @click="emit('sort', props.sortKey, props.sortDirection === 'desc' ? 'asc' : 'desc')"
        />
      </div>

      <slot name="toolbar" />
    </div>

    <ul v-if="props.loading" :aria-label="t('admin.proposals.loading')" aria-busy="true">
      <li
        v-for="index in 8"
        :key="index"
        class="grid gap-4 border-b border-border-subtle px-4 py-4 last:border-0 md:grid-cols-[minmax(0,1fr)_10rem_5rem]"
      >
        <div class="flex gap-4">
          <div class="aspect-4/3 w-16 shrink-0 self-start sm:w-28">
            <UiSkeletonLoader height="100%" />
          </div>
          <div class="flex-1 space-y-2">
            <UiSkeletonLoader width="6rem" height="1.25rem" />
            <UiSkeletonLoader width="75%" height="1rem" />
            <UiSkeletonLoader width="45%" height="0.8rem" />
          </div>
        </div>
        <UiSkeletonLoader class="hidden md:block" width="100%" height="2rem" />
        <UiSkeletonLoader class="hidden md:block" width="3rem" height="2rem" />
      </li>
    </ul>

    <div v-else-if="props.rows.length === 0" class="px-4 py-10">
      <slot name="empty" />
    </div>

    <ul v-else :aria-label="props.label">
      <li
        v-for="row in props.rows"
        :key="row.id"
        class="group relative grid gap-x-3 gap-y-3 border-b border-border-subtle px-4 py-4 transition-colors duration-(--duration-fast) last:border-0 hover:bg-surface-hover md:gap-x-6"
        :class="[rowGrid, selectedSet.has(row.id) ? 'bg-accent-surface hover:bg-accent-surface' : '']"
      >
        <span
          v-if="props.unreadIds.has(row.id)"
          class="absolute inset-y-0 left-0 w-(--border-medium) bg-accent-solid"
          aria-hidden="true"
        />

        <div v-if="props.selectable" class="proposal-check relative z-10 pt-0.5">
          <UiCheckbox
            :model-value="selectedSet.has(row.id)"
            :label="t('data.table.selectRow', { label: tr(row.title) })"
            @update:model-value="(checked: boolean) => toggleRow(row.id, checked)"
          />
        </div>

        <div class="flex min-w-0 gap-4">
          <div class="aspect-4/3 w-16 shrink-0 self-start overflow-hidden rounded-md border border-border-subtle bg-surface-sunken sm:w-28">
            <UiImage v-if="row.cover" :image="row.cover" ratio="4 / 3" sizes="112px" rounded="0" />
            <div v-else class="flex size-full items-center justify-center text-text-subtle" aria-hidden="true">
              <UiIcon name="image" size="1.5rem" />
            </div>
          </div>

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span
                class="inline-flex items-center rounded-sm px-2 py-0.5 text-[0.6875rem] font-semibold tracking-caps uppercase"
                :class="STATUS_TONE[row.status]"
              >
                {{ t(`admin.proposals.status.${row.status}`) }}
              </span>
              <span v-if="props.unreadIds.has(row.id)" class="inline-flex items-center gap-1.5 text-xs font-medium text-accent">
                <span class="size-1.5 rounded-full bg-accent-solid" aria-hidden="true" />
                {{ t('admin.proposals.row.unreadShort') }}
              </span>
              <UiButton
                v-if="isEditable(row)"
                class="relative z-10 ms-auto"
                variant="ghost"
                icon="edit"
                icon-only
                :to="localePath(`/admin/propositions/${row.id}/modifier`)"
                :label="t('admin.proposals.row.edit', { title: tr(row.title) })"
              />
            </div>

            <h3 class="mt-1.5 text-base leading-snug font-semibold text-heading">
              <!-- Le lien s'étend à toute la rangée ; la case à cocher reste au-dessus. -->
              <NuxtLink
                :to="localePath(`/admin/propositions/${row.id}`)"
                class="clamp-2 rounded-sm text-heading no-underline outline-none after:absolute after:inset-0 group-hover:text-accent focus-visible:ring-2 focus-visible:ring-focus"
                :title="tr(row.title)"
              >
                {{ tr(row.title) }}
              </NuxtLink>
            </h3>

            <p class="mt-1.5 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-text-muted">
              <span class="inline-flex min-w-0 items-center gap-1.5">
                <UiIcon name="building" size="0.875rem" class="shrink-0 text-text-subtle" />
                <span class="truncate text-text-secondary">{{ row.organization_acronym || row.organization_name }}</span>
                <span
                  v-if="row.co_organizer_count > 0"
                  class="rounded-full border border-border px-1.5 text-[0.6875rem] font-bold text-text-secondary"
                  :title="t('admin.proposals.row.coOrganizers', row.co_organizer_count)"
                >
                  +{{ row.co_organizer_count }}
                  <span class="sr-only">{{ t('admin.proposals.row.coOrganizers', row.co_organizer_count) }}</span>
                </span>
              </span>
              <span v-if="row.organization_country" class="inline-flex items-center gap-1.5">
                <UiIcon name="globe" size="0.875rem" class="text-text-subtle" />
                {{ tr(row.organization_country) }}
              </span>
              <span class="inline-flex items-center gap-1.5">
                <UiIcon name="monitor" size="0.875rem" class="text-text-subtle" />
                {{ t(`admin.proposals.format.${row.format}`) }}
              </span>
              <span class="inline-flex items-center gap-1.5 whitespace-nowrap">
                <UiIcon name="calendar" size="0.875rem" class="text-text-subtle" />
                {{ row.submitted_at ? date(row.submitted_at, props.timezone) : t('admin.proposals.row.notSubmitted') }}
              </span>
            </p>

            <p
              v-if="row.open_change_requests > 0 || row.is_knocked_out"
              class="mt-1.5 flex flex-wrap gap-x-4 text-xs font-medium"
            >
              <span v-if="row.open_change_requests > 0" class="text-warning">
                {{ t('admin.proposals.row.changeRequests', row.open_change_requests) }}
              </span>
              <span v-if="row.is_knocked_out" class="text-danger">{{ t('admin.proposals.row.knockedOut') }}</span>
            </p>
          </div>
        </div>

        <div
          class="grid grid-cols-[minmax(0,1fr)_auto] gap-6 md:grid-cols-[11rem_5.5rem]"
          :class="props.selectable ? 'col-start-2 md:col-start-3' : ''"
        >
          <div class="min-w-0">
            <div class="flex items-baseline justify-between gap-2">
              <span class="text-[0.6875rem] font-semibold tracking-caps text-text-subtle uppercase">
                {{ t('admin.proposals.columns.reviews') }}
              </span>
              <span class="font-mono text-sm tabular-nums">
                {{ row.review_count }}/{{ expectedReviews(row) }}
                <span class="sr-only">
                  {{ t('admin.proposals.row.reviewProgress', { done: row.review_count, expected: expectedReviews(row) }) }}
                </span>
              </span>
            </div>
            <div v-if="reviewSegments(row).length > 0" class="mt-1.5 flex gap-1" aria-hidden="true">
              <span
                v-for="(done, index) in reviewSegments(row)"
                :key="index"
                class="h-1.5 flex-1 rounded-full"
                :class="done
                  ? (row.review_count >= expectedReviews(row) ? 'bg-success-solid' : 'bg-accent-solid')
                  : 'bg-border'"
              />
            </div>
            <p v-if="row.overdue_reviews > 0" class="mt-1 text-xs text-warning">
              {{ t('admin.proposals.row.late', row.overdue_reviews) }}
            </p>
            <p
              v-else-if="row.assigned_reviewers === 0 && row.review_count === 0 && row.status !== 'draft'"
              class="mt-1 text-xs text-text-subtle"
            >
              {{ t('admin.proposals.row.reviewersNone') }}
            </p>
            <p v-else-if="pendingReviewers(row)" class="mt-1 truncate text-xs text-text-subtle" :title="pendingReviewers(row)">
              {{ t('admin.proposals.row.reviewersPending', { names: pendingReviewers(row) }) }}
            </p>
          </div>

          <div class="text-right">
            <template v-if="row.average_score !== null">
              <p class="font-display text-2xl leading-none font-semibold text-heading tabular-nums">
                {{ row.average_score.toFixed(1) }}<span class="text-xs font-normal text-text-subtle">/20</span>
              </p>
              <p class="mt-1 text-xs text-text-muted">{{ t('admin.proposals.row.rank', { rank: row.event_rank }) }}</p>
            </template>
            <p v-else class="text-xs text-text-subtle">{{ t('admin.proposals.row.noScore') }}</p>
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

/* Libellé des cases gardé pour les lecteurs d'écran, sorti du flux à l'œil. */
.proposal-check :deep(label) {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>
