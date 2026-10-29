<script setup lang="ts">
import type { ProposalDashboardRow } from '~/types/views'

/**
 * LES AUTRES DOSSIERS, à gauche de la fiche (comme en v1) : on passe de l'un à
 * l'autre sans revenir à la liste, avec les filtres de la liste.
 *
 * Données, filtres et tri viennent de `useProposalNavigator`, donc de l'URL :
 * un lien envoyé ouvre la même sélection, et chaque dossier ouvert la garde.
 * Le dossier courant est marqué et ramené en vue à l'ouverture.
 *
 * Deux places : colonne collante sur grand écran (`collapsible`), tiroir sur
 * petit écran (`navigate` le referme après un choix).
 */

interface Props {
  currentId: string
  collapsible?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ collapse: []; navigate: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const nav = useProposalNavigator()

const titleId = useId()
const list = ref<HTMLElement | null>(null)

function unreadOf(id: string): number {
  return nav.unreadMessages.value.get(id) ?? 0
}

const total = computed(() => nav.allRows.value.length)
const shown = computed(() => nav.rows.value.length)

const activeCount = computed(
  () =>
    nav.filters.value.statuses.length +
    nav.filters.value.themes.length +
    nav.filters.value.formats.length +
    nav.filters.value.countries.length +
    nav.filters.value.organizations.length +
    nav.filters.value.flags.length +
    (nav.filters.value.reviewer ? 1 : 0) +
    (nav.minePending.value ? 1 : 0),
)

const isSearching = computed(() => nav.filters.value.search.trim() !== '')

/** Les filtres restent dépliés d'un dossier à l'autre. */
const filtersOpen = useState('review-navigator-filters-open', () => false)

function optionsOf(values: { value: string; label: string; count: number }[]) {
  return [
    { value: '', label: t('common.labels.all') },
    ...values.map((entry) => ({ value: entry.value, label: `${entry.label} (${entry.count})` })),
  ]
}

const facets = computed(() => nav.screen.value?.facets)

const statusOptions = computed(() =>
  optionsOf((facets.value?.statuses ?? []).map((facet) => ({
    value: facet.value,
    label: t(`admin.proposals.status.${facet.value}`),
    count: facet.count,
  }))),
)
const formatOptions = computed(() =>
  optionsOf((facets.value?.formats ?? []).map((facet) => ({
    value: facet.value,
    label: t(`admin.proposals.format.${facet.value}`),
    count: facet.count,
  }))),
)
const themeOptions = computed(() =>
  optionsOf((facets.value?.themes ?? []).map((facet) => ({
    value: facet.value,
    label: typeof facet.label === 'string' ? facet.label : facet.label ? tr(facet.label) : facet.value,
    count: facet.count,
  }))),
)

function patch(partial: Partial<typeof nav.filters.value>): void {
  nav.setFilters({ ...nav.filters.value, ...partial })
}

function toggleFlag(flag: 'unreviewed' | 'unread_messages'): void {
  const flags = nav.filters.value.flags
  patch({ flags: flags.includes(flag) ? flags.filter((entry) => entry !== flag) : [...flags, flag] })
}

function reset(): void {
  nav.setFilters({ ...EMPTY_PROPOSAL_FILTERS, search: nav.filters.value.search })
  nav.setMinePending(false)
}

const flagCount = (flag: 'unreviewed' | 'unread_messages'): number =>
  flag === 'unread_messages'
    ? nav.unreadMessages.value.size
    : (facets.value?.flags.find((facet) => facet.value === flag)?.count ?? 0)

const CHIP =
  'flex min-h-(--target-min) w-full cursor-pointer items-center justify-between gap-2 rounded-md border px-3 text-sm'
  + ' transition-colors duration-(--duration-fast)'
const CHIP_IDLE = 'border-border bg-surface-raised text-text-secondary hover:border-accent hover:text-accent'
const CHIP_ON = 'border-accent-solid bg-accent-solid text-accent-contrast'

function progressOf(row: ProposalDashboardRow): string {
  if (row.average_score !== null) return t('admin.proposal.review.navigator.score', { score: Number(row.average_score).toFixed(1) })
  const expected = row.required_reviews ?? row.assigned_reviewers
  return t('admin.proposal.review.navigator.reviews', { done: row.review_count, expected })
}

// Le dossier courant revient en vue une fois la liste arrivée, pas à chaque filtre.
// Défilement de la seule liste : `scrollIntoView` ferait aussi défiler la page.
let centered = false

function centerCurrent(): void {
  const container = list.value
  const current = container?.querySelector<HTMLElement>('[aria-current="page"]')
  if (centered || !container || !current) return
  container.scrollTop = current.offsetTop - container.clientHeight / 2 + current.offsetHeight / 2
  centered = true
}

watch(() => nav.rows.value.length, () => nextTick(centerCurrent), { flush: 'post' })
onMounted(centerCurrent)
</script>

<template>
  <nav class="flex h-full min-h-0 flex-col" :aria-labelledby="titleId">
    <div class="shrink-0 border-b border-border px-4 pt-3 pb-3">
      <div class="flex items-center justify-between gap-2">
        <h2 :id="titleId" class="text-base font-semibold">
          {{ t('admin.proposal.review.navigator.title') }}
          <span class="ms-1 text-sm font-normal text-text-subtle tabular-nums">
            {{ shown === total
              ? total
              : t('admin.proposal.review.navigator.count', { shown, total }) }}
          </span>
        </h2>
        <UiButton
          v-if="props.collapsible"
          variant="ghost"
          icon="sidebar"
          icon-only
          :label="t('admin.proposal.review.navigator.collapse')"
          @click="emit('collapse')"
        />
      </div>

      <UiSearchInput
        class="mt-2"
        :model-value="nav.filters.value.search"
        :label="t('admin.proposals.filters.search')"
        :placeholder="t('admin.proposals.filters.searchPlaceholder')"
        hide-label
        hide-optional
        block
        @update:model-value="(value: string) => patch({ search: value })"
      />

      <div class="mt-2 flex items-center justify-between gap-2">
        <UiButton
          variant="ghost"
          :icon="filtersOpen ? 'chevron-up' : 'filter'"
          :aria-expanded="filtersOpen"
          aria-controls="review-navigator-filters"
          @click="filtersOpen = !filtersOpen"
        >
          {{ t('admin.proposals.filters.title') }}
          <UiCounter v-if="activeCount > 0" class="ms-2" :value="activeCount" tone="accent" />
        </UiButton>
        <UiButton v-if="activeCount > 0" variant="link" size="sm" @click="reset()">
          {{ t('admin.proposal.review.navigator.reset') }}
        </UiButton>
      </div>

      <div v-show="filtersOpen" id="review-navigator-filters" class="mt-2 flex flex-col gap-3">
        <div class="flex flex-col gap-1.5">
          <button
            type="button"
            :class="[CHIP, nav.minePending.value ? CHIP_ON : CHIP_IDLE]"
            :aria-pressed="nav.minePending.value"
            @click="nav.setMinePending(!nav.minePending.value)"
          >
            {{ t('admin.proposal.review.navigator.mine') }}
            <span class="text-xs tabular-nums opacity-70">{{ nav.awaitingMine.value }}</span>
          </button>
          <button
            v-for="flag in (['unreviewed', 'unread_messages'] as const)"
            :key="flag"
            type="button"
            :class="[CHIP, nav.filters.value.flags.includes(flag) ? CHIP_ON : CHIP_IDLE]"
            :aria-pressed="nav.filters.value.flags.includes(flag)"
            @click="toggleFlag(flag)"
          >
            {{ t(`admin.proposals.filters.flags.${flag}`) }}
            <span class="text-xs tabular-nums opacity-70">{{ flagCount(flag) }}</span>
          </button>
        </div>
        <UiSelect
          :model-value="nav.filters.value.statuses[0] ?? ''"
          :label="t('admin.proposals.filters.status')"
          :options="statusOptions"
          hide-optional
          block
          @update:model-value="(value: string) => patch({ statuses: value ? [value as typeof nav.filters.value.statuses[number]] : [] })"
        />
        <UiSelect
          :model-value="nav.filters.value.themes[0] ?? ''"
          :label="t('admin.proposals.filters.theme')"
          :options="themeOptions"
          hide-optional
          block
          @update:model-value="(value: string) => patch({ themes: value ? [value] : [] })"
        />
        <UiSelect
          :model-value="nav.filters.value.formats[0] ?? ''"
          :label="t('admin.proposals.filters.format')"
          :options="formatOptions"
          hide-optional
          block
          @update:model-value="(value: string) => patch({ formats: value ? [value as typeof nav.filters.value.formats[number]] : [] })"
        />
      </div>
    </div>

    <div ref="list" class="relative min-h-0 flex-1 overflow-y-auto">
      <UiErrorState
        v-if="nav.error.value"
        class="m-4"
        :retry-label="t('common.actions.retry')"
        @retry="nav.refresh()"
      />
      <div v-else-if="nav.status.value === 'pending' && total === 0" class="flex flex-col gap-2 p-4">
        <UiSkeletonLoader v-for="index in 6" :key="index" height="4.5rem" />
      </div>
      <p v-else-if="shown === 0" class="p-4 text-sm text-text-muted">
        {{ isSearching || activeCount > 0
          ? t('admin.proposal.review.navigator.empty')
          : t('admin.proposals.noProposals.title') }}
      </p>

      <ol v-else class="divide-y divide-border-subtle">
        <li v-for="row in nav.rows.value" :key="row.id">
          <NuxtLink
            :to="nav.linkTo(row.id)"
            class="relative block min-h-(--target-min) border-s-2 py-2.5 ps-3.5 pe-4 no-underline transition-colors duration-(--duration-fast) outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-inset"
            :class="row.id === props.currentId
              ? 'border-accent bg-accent-surface'
              : 'border-transparent hover:bg-surface-hover'"
            :aria-current="row.id === props.currentId ? 'page' : undefined"
            @click="emit('navigate')"
          >
            <span class="flex items-center gap-2 text-xs">
              <span class="font-mono text-text-subtle">{{ row.reference_code }}</span>
              <span
                class="inline-flex items-center rounded-sm px-1.5 py-px text-[0.6875rem] font-semibold tracking-caps uppercase"
                :class="PROPOSAL_STATUS_TONE[row.status]"
              >
                {{ t(`admin.proposals.status.${row.status}`) }}
              </span>
              <span
                v-if="nav.unreadIds.value.has(row.id)"
                class="size-1.5 shrink-0 rounded-full bg-accent-solid"
                :title="t('admin.proposals.row.unread')"
              >
                <span class="sr-only">{{ t('admin.proposals.row.unreadShort') }}</span>
              </span>
              <span
                v-if="unreadOf(row.id)"
                class="ms-auto inline-flex items-center gap-1 font-semibold text-accent tabular-nums"
                :title="t('admin.proposals.row.unreadMessages', unreadOf(row.id))"
              >
                <UiIcon name="chat" size="0.95rem" />
                {{ unreadOf(row.id) }}
                <span class="sr-only">{{ t('admin.proposals.row.unreadMessages', unreadOf(row.id)) }}</span>
              </span>
            </span>
            <span
              class="clamp-2 mt-1 block text-sm leading-snug font-semibold"
              :class="row.id === props.currentId ? 'text-accent' : 'text-text'"
            >
              {{ tr(row.title) }}
            </span>
            <span class="mt-0.5 flex items-baseline justify-between gap-3 text-xs text-text-subtle">
              <span class="truncate">{{ row.organization_acronym || row.organization_name }}</span>
              <span class="shrink-0 tabular-nums">{{ progressOf(row) }}</span>
            </span>
          </NuxtLink>
        </li>
      </ol>
    </div>
  </nav>
</template>
