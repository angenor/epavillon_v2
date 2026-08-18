<script setup lang="ts">
import type {
  PlannerSession,
  UnplacedFacets,
  UnplacedFilters,
  UnplacedSortKey,
} from '~/types/admin-planner'
import type { ParticipationMode } from '~/types/event/edition'
import type { SelectOption } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES ACTIVITÉS RETENUES QUI N'ONT PAS ENCORE DE PLACE.
 *
 * C'est la moitié gauche de l'écran, et son compteur — « 12 activités restant à
 * placer » — est la mesure de l'avancement du travail. Il descend à zéro quand
 * le pavillon est composé.
 *
 * FILTRABLE ET TRIABLE PAR NOTE, comme le demande le prompt. La note est le tri
 * par défaut : devant quarante dossiers retenus et deux salles, la question de
 * l'équipe est « lesquels d'abord ». Les facettes sont calculées sur la liste
 * ENTIÈRE, jamais sur la liste filtrée : des valeurs qui disparaissent au fur et
 * à mesure du filtrage empêchent d'élargir sa recherche.
 *
 * DEUX VIDES DIFFÉRENTS, et les confondre envoie chercher des activités qui sont
 * bien là : « tout est placé » n'est pas « aucun résultat pour ces filtres ».
 */

interface Props {
  sessions: PlannerSession[]
  facets: UnplacedFacets
  filters: UnplacedFilters
  sort: UnplacedSortKey
  timezone: TimeZoneName
  zoneLabel?: string
  /** Total avant filtrage : le compteur compte le travail, pas l'affichage. */
  total: number
  /** Faux sur écran étroit : le glisser-déposer n'y a pas de sens. */
  draggable?: boolean
  loading?: boolean
  disabled?: boolean
  selectedId?: string | null
}

const props = withDefaults(defineProps<Props>(), { draggable: true })
const emit = defineEmits<{
  'update:filters': [filters: UnplacedFilters]
  'update:sort': [sort: UnplacedSortKey]
  place: [session: PlannerSession]
  dragstart: [session: PlannerSession]
  dragend: []
}>()

const { t } = useI18n()

const SORT_KEYS: UnplacedSortKey[] = ['score', 'preferred', 'duration', 'title']

const sortOptions = computed<SelectOption[]>(() =>
  SORT_KEYS.map((key) => ({ value: key, label: t(`admin.planner.sort.${key}`) })),
)

/**
 * Les listes de filtre déclarent une option « toutes » de valeur VIDE, plutôt
 * qu'un `placeholder` : une invite désactivée ne se re-sélectionne pas, et l'on
 * ne pourrait plus retirer le filtre (correction relevée au prompt A3).
 */
const themeOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.planner.filters.allThemes') },
  ...props.facets.themes.map((facet) => ({
    value: facet.value,
    label: `${facet.label} (${facet.count})`,
  })),
])

const formatOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.planner.filters.allFormats') },
  ...props.facets.formats.map((facet) => ({
    value: facet.value,
    label: `${facet.label} (${facet.count})`,
  })),
])

const organizationOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.planner.filters.allOrganizations') },
  ...props.facets.organizations.map((facet) => ({
    value: facet.value,
    label: `${facet.label} (${facet.count})`,
  })),
])

function patch(changes: Partial<UnplacedFilters>): void {
  emit('update:filters', { ...props.filters, ...changes })
}

/**
 * Les filtres sont REPLIÉS par défaut, et s'ouvrent d'eux-mêmes quand il y en a
 * un d'actif : arriver sur un panneau filtré dont on ne voit pas les critères,
 * c'est chercher longtemps une activité qui est pourtant là.
 */
const filtersOpen = ref(false)

const filterCount = computed(
  () =>
    props.filters.themes.length + props.filters.formats.length + props.filters.organizations.length,
)

watch(
  () => filterCount.value,
  (count) => {
    if (count > 0) filtersOpen.value = true
  },
  { immediate: true },
)

/** Ce qui filtre, en toutes lettres, retirable un par un. */
const activeChips = computed(() => {
  const chips: { key: string; facet: string; label: string; color?: string | null; clear: () => void }[] = []

  for (const code of props.filters.themes) {
    const facet = props.facets.themes.find((entry) => entry.value === code)
    chips.push({
      key: `theme-${code}`,
      facet: t('admin.planner.filters.theme'),
      label: facet?.label ?? code,
      color: facet?.color,
      clear: () => patch({ themes: [] }),
    })
  }
  for (const format of props.filters.formats) {
    chips.push({
      key: `format-${format}`,
      facet: t('admin.planner.filters.format'),
      label: t(`admin.planner.format.${format}`),
      clear: () => patch({ formats: [] }),
    })
  }
  for (const organization of props.filters.organizations) {
    const facet = props.facets.organizations.find((entry) => entry.value === organization)
    chips.push({
      key: `org-${organization}`,
      facet: t('admin.planner.filters.organization'),
      label: facet?.label ?? organization,
      clear: () => patch({ organizations: [] }),
    })
  }
  return chips
})

const hasFilters = computed(
  () =>
    props.filters.search.trim() !== '' ||
    props.filters.themes.length > 0 ||
    props.filters.formats.length > 0 ||
    props.filters.organizations.length > 0,
)

function reset(): void {
  emit('update:filters', { search: '', themes: [], formats: [], organizations: [] })
}
</script>

<template>
  <section class="flex min-h-0 flex-col rounded-lg border border-border bg-surface" aria-labelledby="planner-unplaced-title">
    <header class="border-b border-border px-4 py-3">
      <div class="flex items-center gap-2">
        <h2 id="planner-unplaced-title" class="text-sm font-semibold tracking-wide text-text uppercase">
          {{ t('admin.planner.unplaced.title') }}
        </h2>
        <UiCounter :value="props.total" :tone="props.total > 0 ? 'accent' : 'neutral'" />
      </div>
      <!-- LE COMPTEUR DU PROMPT, écrit en toutes lettres : le nombre seul, dans
           une pastille, ne dit pas de quoi il s'agit. -->
      <p class="mt-1 text-xs text-text-muted">
        {{ t('admin.planner.unplaced.remaining', props.total) }}
      </p>
    </header>

    <!-- UNE SEULE LIGNE DE COMMANDES, ET LES FILTRES REPLIÉS.
         Les quatre listes déroulantes occupaient toute la hauteur visible du
         panneau : on ne voyait pas une seule activité avant de défiler, sur un
         écran dont c'est pourtant la moitié du sujet. Ce qui sert à chaque
         ouverture reste dehors — chercher une activité, changer l'ordre — et le
         reste attend qu'on le demande. Rien n'est perdu : le bouton porte le
         nombre de filtres actifs, et les jetons rappellent lesquels. -->
    <div class="space-y-2 border-b border-border px-3 py-2">
      <div class="flex items-center gap-2">
        <UiSearchInput
          class="min-w-0 flex-1"
          :model-value="props.filters.search"
          :label="t('admin.planner.filters.search')"
          :placeholder="t('admin.planner.filters.searchPlaceholder')"
          hide-label
          size="sm"
          :disabled="props.disabled"
          @update:model-value="(value: string) => patch({ search: value })"
        />

        <UiButton
          size="sm"
          :variant="filterCount > 0 ? 'secondary' : 'ghost'"
          icon="filter"
          :aria-expanded="filtersOpen"
          aria-controls="planner-unplaced-filters"
          :disabled="props.disabled"
          @click="filtersOpen = !filtersOpen"
        >
          {{ filterCount > 0 ? t('admin.planner.filters.openWithCount', { count: filterCount }) : t('admin.planner.filters.open') }}
        </UiButton>
      </div>

      <!-- Le tri reste dehors : le prompt le demande explicitement, et l'ordre
           du comité est la première question devant un pavillon à remplir. -->
      <label class="flex items-center gap-2 text-xs text-text-muted">
        <span class="shrink-0">{{ t('admin.planner.sort.label') }}</span>
        <select
          class="min-w-0 flex-1 cursor-pointer rounded-md border border-border bg-surface-raised px-2 py-1 text-xs text-text"
          :value="props.sort"
          :disabled="props.disabled"
          @change="emit('update:sort', ($event.target as HTMLSelectElement).value as UnplacedSortKey)"
        >
          <option v-for="option in sortOptions" :key="option.value" :value="option.value">
            {{ option.label }}
          </option>
        </select>
      </label>

      <div v-show="filtersOpen" id="planner-unplaced-filters" class="space-y-2 pt-1">
        <UiSelect
          :model-value="props.filters.themes[0] ?? ''"
          :options="themeOptions"
          :label="t('admin.planner.filters.theme')"
          hide-optional
          size="sm"
          :disabled="props.disabled"
          @update:model-value="(value: string) => patch({ themes: value ? [value] : [] })"
        />
        <UiSelect
          :model-value="props.filters.formats[0] ?? ''"
          :options="formatOptions"
          :label="t('admin.planner.filters.format')"
          hide-optional
          size="sm"
          :disabled="props.disabled"
          @update:model-value="(value: string) => patch({ formats: value ? [value as ParticipationMode] : [] })"
        />
        <UiSelect
          :model-value="props.filters.organizations[0] ?? ''"
          :options="organizationOptions"
          :label="t('admin.planner.filters.organization')"
          hide-optional
          size="sm"
          :disabled="props.disabled"
          @update:model-value="(value: string) => patch({ organizations: value ? [value] : [] })"
        />
      </div>

      <!-- Filtres repliés, mais JAMAIS invisibles : un filtre actif qu'on ne voit
           pas fait chercher longtemps une activité qui est bien là. -->
      <div v-if="hasFilters" class="flex flex-wrap items-center gap-1.5">
        <UiChip
          v-for="chip in activeChips"
          :key="chip.key"
          :label="chip.label"
          :facet="chip.facet"
          :dot-color="chip.color"
          @remove="chip.clear()"
        />
        <button type="button" class="cursor-pointer text-xs text-text-muted underline hover:text-text" @click="reset">
          {{ t('admin.planner.filters.reset') }}
        </button>
      </div>
      <p v-if="hasFilters" class="text-xs text-text-muted">
        {{ t('admin.planner.filters.shown', { shown: props.sessions.length, total: props.total }) }}
      </p>
    </div>

    <!-- La liste défile seule : la grille du calendrier, à droite, ne doit pas
         partir hors de l'écran parce que quarante activités attendent. -->
    <div class="min-h-0 flex-1 overflow-y-auto p-3">
      <div v-if="props.loading" class="space-y-3">
        <UiSkeletonLoader v-for="index in 4" :key="index" height="7.5rem" rounded="0.5rem" />
      </div>

      <UiEmptyState
        v-else-if="props.total === 0"
        compact
        icon="check-circle"
        :title="t('admin.planner.unplaced.empty.title')"
        :description="t('admin.planner.unplaced.empty.description')"
      />

      <UiEmptyState
        v-else-if="props.sessions.length === 0"
        compact
        filtered
        :title="t('admin.planner.unplaced.noResults.title')"
        :description="t('admin.planner.unplaced.noResults.description', { total: props.total })"
        :action-label="t('admin.planner.filters.reset')"
        @action="reset"
      />

      <ul v-else class="space-y-3">
        <li v-for="session in props.sessions" :key="session.id">
          <AdminPlannerActivityCard
            :session="session"
            :timezone="props.timezone"
            :zone-label="props.zoneLabel"
            :draggable="props.draggable"
            :disabled="props.disabled"
            :selected="props.selectedId === session.id"
            @place="emit('place', $event)"
            @dragstart="emit('dragstart', $event)"
            @dragend="emit('dragend')"
          />
        </li>
      </ul>
    </div>
  </section>
</template>
