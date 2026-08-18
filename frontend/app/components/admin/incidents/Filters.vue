<script setup lang="ts">
import type {
  IncidentFilters,
  IncidentState,
  IncidentStateCounts,
} from '~/types/admin-incidents'
import type { IncidentScope, IncidentSeverity } from '~/types/live'
import type { TaxonomyTerm } from '~/types/reference'
import type { SelectOption } from '~/types/ui'

/**
 * LES FILTRES DE LA LISTE.
 *
 * L'ÉTAT SE COCHE, ET IL EST EN PREMIER. C'est la seule question qu'on pose
 * vraiment à cet écran — « qu'est-ce qui est en ligne en ce moment ? » —, et
 * l'état porte son compte : cocher « Rédigé » quand il n'y en a aucun ne rend
 * rien, autant le savoir avant de cliquer.
 *
 * LES NATURES VIENNENT DE LA BASE (`incident_kind`), pas d'une liste écrite ici.
 * L'IFDD en ajoute depuis le back-office : une liste figée dans le code se
 * désynchroniserait au premier terme ajouté — c'est le défaut n° 1 de la v1.
 */

interface Props {
  filters: IncidentFilters
  counts: IncidentStateCounts
  kinds: TaxonomyTerm[]
  total: number
  shown: number
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:filters': [value: IncidentFilters] }>()

const { t } = useI18n()
const { tr } = useI18nText()

function patch(part: Partial<IncidentFilters>): void {
  emit('update:filters', { ...props.filters, ...part })
}

const SEVERITIES: IncidentSeverity[] = ['info', 'warning', 'error', 'critical']
const SCOPES: IncidentScope[] = ['global', 'event', 'event_day', 'session', 'organization']

const severityOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...SEVERITIES.map((severity) => ({
    value: severity,
    label: t(`admin.incident.form.severity.option.${severity}`),
  })),
])

const scopeOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...SCOPES.map((scope) => ({ value: scope, label: t(`incident-banner.scope.${scope}`) })),
])

const kindOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('common.labels.all') },
  ...props.kinds.map((kind) => ({ value: kind.code, label: tr(kind.label) })),
])

function toggleState(state: IncidentState): void {
  const states = props.filters.states.includes(state)
    ? props.filters.states.filter((entry) => entry !== state)
    : [...props.filters.states, state]
  patch({ states })
}
</script>

<template>
  <section
    class="rounded-lg border border-border bg-surface-raised p-4"
    :aria-label="t('admin.incident.list.filters.label')"
  >
    <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <UiSearchInput
        :model-value="filters.search"
        :label="t('admin.incident.list.filters.search')"
        :placeholder="t('admin.incident.list.filters.searchPlaceholder')"
        :disabled="disabled"
        @update:model-value="patch({ search: $event })"
      />

      <UiFormField :label="t('admin.incident.list.filters.severity')" hide-optional>
        <UiSelect
          :model-value="filters.severities[0] ?? ''"
          :options="severityOptions"
          hide-optional
          :disabled="disabled"
          @update:model-value="patch({ severities: $event ? [$event as IncidentSeverity] : [] })"
        />
      </UiFormField>

      <UiFormField :label="t('admin.incident.list.filters.scope')" hide-optional>
        <UiSelect
          :model-value="filters.scopes[0] ?? ''"
          :options="scopeOptions"
          hide-optional
          :disabled="disabled"
          @update:model-value="patch({ scopes: $event ? [$event as IncidentScope] : [] })"
        />
      </UiFormField>

      <UiFormField :label="t('admin.incident.list.filters.kind')" hide-optional>
        <UiSelect
          :model-value="filters.kinds[0] ?? ''"
          :options="kindOptions"
          hide-optional
          :disabled="disabled"
          @update:model-value="patch({ kinds: $event ? [$event] : [] })"
        />
      </UiFormField>
    </div>

    <fieldset class="mt-4 flex flex-wrap items-center gap-x-4 gap-y-2">
      <legend class="float-start me-3 text-sm text-text-muted">
        {{ t('admin.incident.list.filters.state') }}
      </legend>
      <UiCheckbox
        v-for="state in INCIDENT_STATES"
        :key="state"
        :model-value="filters.states.includes(state)"
        :label="`${t(`admin.incident.list.state.${state}`)} (${counts[state]})`"
        :disabled="disabled || counts[state] === 0"
        @update:model-value="toggleState(state)"
      />
    </fieldset>

    <p class="mt-3 text-sm text-text-muted" aria-live="polite">
      {{ t('admin.incident.list.filters.shown', { shown, total }) }}
      <button
        v-if="hasActiveIncidentFilters(filters)"
        type="button"
        class="ml-2 cursor-pointer text-accent underline underline-offset-2"
        @click="emit('update:filters', NO_INCIDENT_FILTERS)"
      >
        {{ t('admin.incident.list.filters.reset') }}
      </button>
    </p>
  </section>
</template>
