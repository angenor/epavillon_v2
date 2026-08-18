<script setup lang="ts">
import type {
  ProposalFacet,
  ProposalFacets,
  ProposalFlag,
  ProposalListFilters,
} from '~/types/admin-proposals'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { ParticipationMode } from '~/types/event/edition'

/**
 * LES FILTRES DE LA LISTE DES PROPOSITIONS.
 *
 * HUIT FILTRES, ET ILS NE SE RESSEMBLENT PAS — c'est délibéré, la forme dit
 * l'usage :
 *
 *  · STATUT, FORMAT, SIGNAUX se prennent en un coup d'œil : des jetons qu'on
 *    bascule, avec leur décompte. Ce sont les filtres qu'on utilise vingt fois
 *    par jour, ils ne méritent pas une liste déroulante à ouvrir.
 *  · THÉMATIQUE, PAYS, ORGANISATION, RÉVISIONNISTE ont trop de valeurs pour
 *    tenir en jetons — dix-huit thématiques, treize organisations. Listes
 *    déroulantes, dont les choix retenus reviennent en jetons retirables
 *    au-dessus du tableau : on doit voir ce qui filtre sans rouvrir les listes.
 *
 * LES DÉCOMPTES SONT CEUX DU PÉRIMÈTRE, filtres non appliqués. « Retenu (17) »
 * reste lisible quand on a déjà coché « En évaluation » ; recalculés à chaque
 * coche, ils tomberaient tous à zéro sauf un et le filtre cesserait de dire ce
 * qu'il reste à explorer.
 *
 * LES LIBELLÉS VIENNENT DE DEUX ENDROITS, et ne se confondent pas : statut,
 * format et signaux sont des libellés d'INTERFACE (i18n) ; thématique, pays,
 * organisation et personne sont des DONNÉES, résolues depuis la base. Recopier
 * une thématique dans un fichier de traduction est le défaut n° 1 de la v1.
 */

interface Props {
  filters: ProposalListFilters
  facets: ProposalFacets
  /** Nombre de dossiers de l'édition, avant tout filtre. */
  total: number
  /** Nombre de dossiers affichés après filtrage. */
  shown: number
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:filters': [filters: ProposalListFilters] }>()

const { t } = useI18n()
const { tr } = useI18nText()

/** Le panneau des listes déroulantes, replié par défaut sur écran étroit. */
const isExpanded = ref(false)

const labelOf = (facet: ProposalFacet): string =>
  typeof facet.label === 'string' ? facet.label : facet.label ? tr(facet.label) : facet.value

function patch(partial: Partial<ProposalListFilters>): void {
  emit('update:filters', { ...props.filters, ...partial })
}

/** Bascule d'une valeur dans une liste — un jeton coché se décoche. */
function toggle<T extends string>(list: T[], value: T): T[] {
  return list.includes(value) ? list.filter((entry) => entry !== value) : [...list, value]
}

const activeCount = computed(
  () =>
    props.filters.statuses.length +
    props.filters.themes.length +
    props.filters.formats.length +
    props.filters.countries.length +
    props.filters.organizations.length +
    props.filters.flags.length +
    (props.filters.reviewer ? 1 : 0) +
    (props.filters.search.trim() ? 1 : 0),
)

/**
 * Les jetons de filtre ACTIFS, tous filtres confondus, avec de quoi les retirer
 * un à un. Ils sont la seule vue complète de ce qui filtre la liste : sans eux,
 * un filtre posé dans une liste déroulante repliée devient invisible, et
 * l'équipe croit à une liste vide.
 */
interface ActiveChip {
  key: string
  facet: string
  label: string
  color?: string | null
  remove: () => void
}

const activeChips = computed<ActiveChip[]>(() => {
  const chips: ActiveChip[] = []

  for (const status of props.filters.statuses) {
    chips.push({
      key: `status:${status}`,
      facet: t('admin.proposals.filters.status'),
      label: t(`admin.proposals.status.${status}`),
      remove: () => patch({ statuses: toggle(props.filters.statuses, status) }),
    })
  }
  for (const format of props.filters.formats) {
    chips.push({
      key: `format:${format}`,
      facet: t('admin.proposals.filters.format'),
      label: t(`admin.proposals.format.${format}`),
      remove: () => patch({ formats: toggle(props.filters.formats, format) }),
    })
  }
  for (const code of props.filters.themes) {
    const facet = props.facets.themes.find((entry) => entry.value === code)
    chips.push({
      key: `theme:${code}`,
      facet: t('admin.proposals.filters.theme'),
      label: facet ? labelOf(facet) : code,
      color: facet?.color ?? null,
      remove: () => patch({ themes: toggle(props.filters.themes, code) }),
    })
  }
  for (const iso of props.filters.countries) {
    const facet = props.facets.countries.find((entry) => entry.value === iso)
    chips.push({
      key: `country:${iso}`,
      facet: t('admin.proposals.filters.country'),
      label: facet ? labelOf(facet) : iso,
      remove: () => patch({ countries: toggle(props.filters.countries, iso) }),
    })
  }
  for (const id of props.filters.organizations) {
    const facet = props.facets.organizations.find((entry) => entry.value === id)
    chips.push({
      key: `org:${id}`,
      facet: t('admin.proposals.filters.organization'),
      label: facet ? labelOf(facet) : id,
      remove: () => patch({ organizations: toggle(props.filters.organizations, id) }),
    })
  }
  if (props.filters.reviewer) {
    const facet = props.facets.reviewers.find((entry) => entry.value === props.filters.reviewer)
    chips.push({
      key: `reviewer:${props.filters.reviewer}`,
      facet: t('admin.proposals.filters.reviewer'),
      label: facet ? labelOf(facet) : props.filters.reviewer,
      remove: () => patch({ reviewer: null }),
    })
  }
  for (const flag of props.filters.flags) {
    chips.push({
      key: `flag:${flag}`,
      facet: t('admin.proposals.filters.signals'),
      label: t(`admin.proposals.filters.flags.${flag}`),
      remove: () => patch({ flags: toggle(props.filters.flags, flag) }),
    })
  }

  return chips
})

/** Options d'une liste déroulante : « Tous » explicite, puis les facettes. */
function optionsOf(facets: ProposalFacet[]): { value: string; label: string; description?: string }[] {
  return [
    { value: '', label: t('common.labels.all') },
    ...facets.map((facet) => ({
      value: facet.value,
      label: labelOf(facet),
      description: t('admin.proposals.filters.facetCount', { count: facet.count }),
    })),
  ]
}

const FLAGS: ProposalFlag[] = ['unreviewed', 'late', 'unread']

function flagCount(flag: ProposalFlag): number {
  return props.facets.flags.find((facet) => facet.value === flag)?.count ?? 0
}

/**
 * Habillage commun des jetons bascule. Un filtre actif prend l'aplat d'accent —
 * il doit se distinguer sans lecture, une barre en compte parfois quinze.
 */
const CHIP
  = 'inline-flex min-h-(--target-compact) cursor-pointer items-center gap-2 rounded-md border'
    + ' px-3 text-sm transition-colors duration-(--duration-fast)'
    + ' disabled:cursor-not-allowed disabled:opacity-50'
const CHIP_IDLE = 'border-border bg-surface-raised text-text-secondary hover:border-accent hover:text-accent'
const CHIP_ON = 'border-accent-solid bg-accent-solid text-accent-contrast'
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised" :aria-label="t('admin.proposals.filters.title')">
    <div class="flex flex-wrap items-end gap-3 px-4 py-3">
      <UiSearchInput
        class="min-w-0 flex-1 basis-64"
        :model-value="props.filters.search"
        :label="t('admin.proposals.filters.search')"
        :placeholder="t('admin.proposals.filters.searchPlaceholder')"
        :result-count="props.shown"
        hide-label
        hide-optional
        :disabled="props.disabled"
        @update:model-value="(value: string) => patch({ search: value })"
      />

      <p class="text-sm whitespace-nowrap text-text-muted tabular-nums">
        {{ props.shown === props.total
          ? t('admin.proposals.results.count', props.total)
          : t('admin.proposals.results.filtered', { shown: props.shown, total: props.total }) }}
      </p>

      <UiButton
        variant="ghost"
        size="sm"
        :icon="isExpanded ? 'chevron-up' : 'filter'"
        :aria-expanded="isExpanded"
        aria-controls="admin-proposal-filters"
        @click="isExpanded = !isExpanded"
      >
        {{ isExpanded ? t('admin.proposals.filters.hide') : t('admin.proposals.filters.show') }}
        <UiCounter v-if="activeCount > 0" class="ml-2" :value="activeCount" tone="accent" />
      </UiButton>
    </div>

    <!-- JETONS À BASCULE : statut, format, signaux. Les filtres les plus
         employés ne se cachent pas derrière une liste déroulante. -->
    <div class="flex flex-wrap items-center gap-x-6 gap-y-3 border-t border-separator px-4 py-3">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-xs tracking-caps text-text-subtle uppercase">
          {{ t('admin.proposals.filters.status') }}
        </span>
        <button
          v-for="facet in props.facets.statuses"
          :key="facet.value"
          type="button"
          :class="[CHIP, props.filters.statuses.includes(facet.value as ProposalStatus) ? CHIP_ON : CHIP_IDLE]"
          :aria-pressed="props.filters.statuses.includes(facet.value as ProposalStatus)"
          :disabled="props.disabled"
          @click="patch({ statuses: toggle(props.filters.statuses, facet.value as ProposalStatus) })"
        >
          {{ t(`admin.proposals.status.${facet.value}`) }}
          <span class="text-xs tabular-nums opacity-70">{{ facet.count }}</span>
        </button>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <span class="text-xs tracking-caps text-text-subtle uppercase">
          {{ t('admin.proposals.filters.format') }}
        </span>
        <button
          v-for="facet in props.facets.formats"
          :key="facet.value"
          type="button"
          :class="[CHIP, props.filters.formats.includes(facet.value as ParticipationMode) ? CHIP_ON : CHIP_IDLE]"
          :aria-pressed="props.filters.formats.includes(facet.value as ParticipationMode)"
          :disabled="props.disabled"
          @click="patch({ formats: toggle(props.filters.formats, facet.value as ParticipationMode) })"
        >
          {{ t(`admin.proposals.format.${facet.value}`) }}
          <span class="text-xs tabular-nums opacity-70">{{ facet.count }}</span>
        </button>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <span class="text-xs tracking-caps text-text-subtle uppercase">
          {{ t('admin.proposals.filters.signals') }}
        </span>
        <button
          v-for="flag in FLAGS"
          :key="flag"
          type="button"
          :class="[CHIP, props.filters.flags.includes(flag) ? CHIP_ON : CHIP_IDLE]"
          :aria-pressed="props.filters.flags.includes(flag)"
          :disabled="props.disabled"
          @click="patch({ flags: toggle(props.filters.flags, flag) })"
        >
          {{ t(`admin.proposals.filters.flags.${flag}`) }}
          <span class="text-xs tabular-nums opacity-70">{{ flagCount(flag) }}</span>
        </button>
      </div>
    </div>

    <!-- LISTES DÉROULANTES : les filtres à trop de valeurs pour tenir en jetons. -->
    <div
      v-show="isExpanded"
      id="admin-proposal-filters"
      class="grid gap-4 border-t border-separator px-4 py-4 sm:grid-cols-2 lg:grid-cols-4"
    >
      <UiSelect
        :model-value="props.filters.themes[0] ?? ''"
        :label="t('admin.proposals.filters.theme')"
        :options="optionsOf(props.facets.themes)"
        hide-optional
        block
        :disabled="props.disabled"
        @update:model-value="(value: string) => patch({ themes: value ? [value] : [] })"
      />
      <UiSelect
        :model-value="props.filters.countries[0] ?? ''"
        :label="t('admin.proposals.filters.country')"
        :options="optionsOf(props.facets.countries)"
        hide-optional
        block
        :disabled="props.disabled"
        @update:model-value="(value: string) => patch({ countries: value ? [value] : [] })"
      />
      <UiSelect
        :model-value="props.filters.organizations[0] ?? ''"
        :label="t('admin.proposals.filters.organization')"
        :options="optionsOf(props.facets.organizations)"
        hide-optional
        block
        :disabled="props.disabled"
        @update:model-value="(value: string) => patch({ organizations: value ? [value] : [] })"
      />
      <UiSelect
        :model-value="props.filters.reviewer ?? ''"
        :label="t('admin.proposals.filters.reviewer')"
        :options="[
          { value: '', label: t('admin.proposals.filters.anyReviewer') },
          ...props.facets.reviewers.map((facet) => ({
            value: facet.value,
            label: labelOf(facet),
            description: t('admin.proposals.filters.facetCount', { count: facet.count }),
          })),
        ]"
        hide-optional
        block
        :disabled="props.disabled"
        @update:model-value="(value: string) => patch({ reviewer: value || null })"
      />
    </div>

    <!-- CE QUI FILTRE, EN CLAIR. Un filtre posé puis replié doit rester visible :
         sans ces jetons, une liste vide se lit comme une panne. -->
    <div
      v-if="activeChips.length > 0"
      class="flex flex-wrap items-center gap-2 border-t border-separator px-4 py-3"
    >
      <UiChip
        v-for="chip in activeChips"
        :key="chip.key"
        :facet="chip.facet"
        :label="chip.label"
        :dot-color="chip.color"
        :disabled="props.disabled"
        @remove="chip.remove()"
      />
      <UiButton variant="link" size="sm" @click="emit('update:filters', {
        search: '',
        statuses: [],
        themes: [],
        formats: [],
        countries: [],
        organizations: [],
        reviewer: null,
        flags: [],
      })">
        {{ t('admin.proposals.filters.reset') }}
      </UiButton>
    </div>
  </section>
</template>
