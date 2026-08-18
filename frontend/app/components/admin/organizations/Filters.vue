<script setup lang="ts">
import type { OrganizationListFilters, OrganizationFacet } from '~/types/admin-organizations'
import type { OrganizationStatus } from '~/types/org'
import type { SelectOption } from '~/types/ui'

/**
 * Filtres de la liste des organisations.
 *
 * L'ÉTAT N'EST PAS ICI. Le composant reçoit les filtres et émet le prochain jeu ;
 * la page les tient dans l'URL. Même partage qu'en A7 et A10.
 *
 * LE FILTRE « FICHES À REGARDER » N'EST PAS UN FILTRE COMME LES AUTRES : c'est
 * le geste par lequel commence une séance de nettoyage du référentiel. Il ne se
 * cache pas dans une liste déroulante — c'est une case à cocher, posée au bout
 * de la barre, qui ramène les fiches sous le seuil de confiance.
 */

interface Props {
  filters: OrganizationListFilters
  countries: OrganizationFacet[]
  types: OrganizationFacet[]
  total: number
  shown: number
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:filters': [value: OrganizationListFilters] }>()

const { t } = useI18n()
const { tr } = useI18nText()

/** `merged` et `rejected` sont proposées : une fiche absorbée reste consultable. */
const STATUSES: OrganizationStatus[] = ['candidate', 'active', 'merged', 'archived', 'rejected']

function facetLabel(facet: OrganizationFacet): string {
  const label = typeof facet.label === 'string' ? facet.label : tr(facet.label)
  return t('admin.organization.list.filters.facet', { name: label, count: facet.count })
}

const countryOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...props.countries.map((facet) => ({ value: facet.value, label: facetLabel(facet) })),
])

const typeOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...props.types.map((facet) => ({ value: facet.value, label: facetLabel(facet) })),
])

const statusOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...STATUSES.map((status) => ({
    value: status,
    label: t('admin.organization.list.status.' + status),
  })),
])

/** Trois états et non deux : « sans importance » n'est pas « non ». */
const TRISTATE = ['', 'yes', 'no'] as const

const verifiedOptions = computed<SelectOption[]>(() =>
  TRISTATE.map((value) => ({
    value,
    label: t(
      value === ''
        ? 'admin.organization.list.filters.verifiedAny'
        : value === 'yes'
          ? 'admin.organization.list.filters.verifiedYes'
          : 'admin.organization.list.filters.verifiedNo',
    ),
  })),
)

const duplicateOptions = computed<SelectOption[]>(() =>
  TRISTATE.map((value) => ({
    value,
    label: t(
      value === ''
        ? 'admin.organization.list.filters.duplicateAny'
        : value === 'yes'
          ? 'admin.organization.list.filters.duplicateYes'
          : 'admin.organization.list.filters.duplicateNo',
    ),
  })),
)

function tristate(value: boolean | null): string {
  return value === null ? '' : value ? 'yes' : 'no'
}

function patch(next: Partial<OrganizationListFilters>): void {
  emit('update:filters', { ...props.filters, ...next })
}
</script>

<template>
  <fieldset
    class="rounded-lg border border-border bg-surface-raised p-4"
    :disabled="props.disabled"
  >
    <legend class="sr-only">{{ t('admin.organization.list.filters.legend') }}</legend>

    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-6">
      <div class="lg:col-span-2">
        <UiSearchInput
          :model-value="props.filters.search"
          :label="t('admin.organization.list.filters.search')"
          :placeholder="t('admin.organization.list.filters.search')"
          @update:model-value="(next: string) => patch({ search: next })"
        />
      </div>

      <UiSelect
        :model-value="props.filters.countries[0] ?? ''"
        :label="t('admin.organization.list.filters.countries')"
        :options="countryOptions"
        hide-optional
        @update:model-value="(next: string) => patch({ countries: next ? [next] : [] })"
      />

      <UiSelect
        :model-value="props.filters.types[0] ?? ''"
        :label="t('admin.organization.list.filters.types')"
        :options="typeOptions"
        hide-optional
        @update:model-value="(next: string) => patch({ types: next ? [next] : [] })"
      />

      <UiSelect
        :model-value="props.filters.statuses[0] ?? ''"
        :label="t('admin.organization.list.filters.statuses')"
        :options="statusOptions"
        hide-optional
        @update:model-value="
          (next: string) => patch({ statuses: next ? [next as OrganizationStatus] : [] })
        "
      />

      <UiSelect
        :model-value="tristate(props.filters.verified)"
        :label="t('admin.organization.list.filters.verified')"
        :options="verifiedOptions"
        hide-optional
        @update:model-value="
          (next: string) => patch({ verified: next === '' ? null : next === 'yes' })
        "
      />
    </div>

    <div class="mt-3 flex flex-wrap items-center justify-between gap-x-6 gap-y-2">
      <div class="flex flex-wrap items-center gap-x-6">
        <UiSelect
          :model-value="tristate(props.filters.has_duplicate)"
          :label="t('admin.organization.list.filters.duplicate')"
          :options="duplicateOptions"
          size="sm"
          hide-optional
          @update:model-value="
            (next: string) => patch({ has_duplicate: next === '' ? null : next === 'yes' })
          "
        />

        <!-- Le geste par lequel commence une séance de nettoyage : les fiches que
             ni le sceau ni un domaine vérifié n'ont encore validées. -->
        <UiCheckbox
          :model-value="props.filters.max_trust_score !== null"
          :label="t('admin.organization.list.filters.lowTrust', { threshold: LOW_TRUST_SCORE })"
          @update:model-value="
            (next: boolean) => patch({ max_trust_score: next ? LOW_TRUST_SCORE : null })
          "
        />
      </div>

      <p class="text-sm text-text-muted" aria-live="polite">
        {{ t('admin.organization.list.filters.shown', { shown: props.shown, total: props.total }) }}
      </p>
    </div>
  </fieldset>
</template>
