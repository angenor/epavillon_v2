<script setup lang="ts">
import type { AssignableRole, UserFacet, UserListFilters } from '~/types/admin-users'
import type { PersonStatus, ScopeType } from '~/types/identity'
import type { SelectOption } from '~/types/ui'

/**
 * LES FILTRES DE LA LISTE.
 *
 * DEUX QUESTIONS DIFFÉRENTES, DEUX FILTRES DISTINCTS — c'est la seule décision de
 * cet écran qui mérite d'être défendue :
 *
 *   « Qui est révisionniste ? »             → le filtre RÔLE
 *   « Qui a un rôle sur la COP31 ? »        → le filtre PORTÉE
 *
 * Une liste déroulante unique qui offrirait « Révisionniste COP31 » et
 * « Révisionniste COP30 » comme deux entrées rendrait la première question
 * impossible à poser — alors que c'est la plus fréquente. Le prix est un filtre
 * de plus ; le gain est de pouvoir croiser les deux.
 *
 * DEUX INTERRUPTEURS QUI NE SE DEVINENT PAS : « sans rôle » et « sans compte ».
 * Le premier trouve les comptes ordinaires ; le second trouve les personnes
 * créées par une invitation ou saisies comme intervenantes, qui n'ont jamais eu
 * de quoi se connecter — et que rien d'autre ne permet de retrouver.
 */

interface Props {
  filters: UserListFilters
  roles: AssignableRole[]
  countries: UserFacet[]
  organizations: UserFacet[]
  /** Éditions et organisations, pour le filtre de portée. */
  scopeTargets: { value: string; label: string; scope_type: ScopeType }[]
  total: number
  shown: number
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:filters': [value: UserListFilters] }>()

const { t } = useI18n()
const { tr } = useI18nText()

function patch(part: Partial<UserListFilters>): void {
  emit('update:filters', { ...props.filters, ...part })
}

const STATUSES: PersonStatus[] = ['active', 'suspended', 'blocked', 'anonymized']

const roleOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.user.list.filters.anyRole') },
  ...props.roles
    .filter((role) => role.active_count > 0)
    .map((role) => ({
      value: role.code,
      label: tr(role.label),
      description: t('admin.user.list.filters.roleCount', { count: role.active_count }),
    })),
])

const scopeOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.user.list.filters.anyScope') },
  { value: 'global', label: t('admin.user.roles.scopeType.global') },
  ...props.scopeTargets.map((target) => ({
    value: `${target.scope_type}:${target.value}`,
    label: target.label,
    description: t(`admin.user.roles.scopeType.${target.scope_type}`),
  })),
])

const scopeValue = computed(() => {
  if (props.filters.scope_type === null) return ''
  if (props.filters.scope_type === 'global') return 'global'
  return `${props.filters.scope_type}:${props.filters.scope_id ?? ''}`
})

function setScope(raw: string): void {
  if (raw === '') return patch({ scope_type: null, scope_id: null })
  if (raw === 'global') return patch({ scope_type: 'global', scope_id: null })
  const [scopeType, scopeId] = raw.split(':')
  patch({ scope_type: scopeType as ScopeType, scope_id: scopeId ?? null })
}

const countryOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.user.list.filters.anyCountry') },
  ...props.countries.map((facet) => ({
    value: facet.value,
    label: typeof facet.label === 'string' ? facet.label : tr(facet.label),
    description: String(facet.count),
  })),
])

const organizationOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.user.list.filters.anyOrganization') },
  ...props.organizations.map((facet) => ({
    value: facet.value,
    label: typeof facet.label === 'string' ? facet.label : tr(facet.label),
    description: String(facet.count),
  })),
])

function toggleStatus(status: PersonStatus): void {
  const statuses = props.filters.statuses.includes(status)
    ? props.filters.statuses.filter((entry) => entry !== status)
    : [...props.filters.statuses, status]
  patch({ statuses })
}
</script>

<template>
  <section
    class="rounded-lg border border-border bg-surface-raised p-4"
    :aria-label="t('admin.user.list.filters.label')"
  >
    <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
      <UiSearchInput
        :model-value="filters.search"
        :label="t('admin.user.list.filters.search')"
        :placeholder="t('admin.user.list.filters.searchPlaceholder')"
        :disabled="disabled"
        @update:model-value="patch({ search: $event })"
      />

      <UiFormField :label="t('admin.user.list.filters.role')">
        <UiSelect
          :model-value="filters.roles[0] ?? ''"
          :options="roleOptions"
          :disabled="disabled"
          @update:model-value="patch({ roles: $event ? [$event] : [] })"
        />
      </UiFormField>

      <!-- LE FILTRE DE PORTÉE — l'autre moitié de la question. -->
      <UiFormField
        :label="t('admin.user.list.filters.scope')"
        :hint="t('admin.user.list.filters.scopeHint')"
      >
        <UiSelect
          :model-value="scopeValue"
          :options="scopeOptions"
          :disabled="disabled"
          @update:model-value="setScope($event)"
        />
      </UiFormField>

      <UiFormField :label="t('admin.user.list.filters.organization')">
        <UiSelect
          :model-value="filters.organizations[0] ?? ''"
          :options="organizationOptions"
          :disabled="disabled"
          @update:model-value="patch({ organizations: $event ? [$event] : [] })"
        />
      </UiFormField>
    </div>

    <div class="mt-4 flex flex-wrap items-center gap-x-4 gap-y-3">
      <!-- LE STATUT SE COCHE, IL NE SE JETONNE PAS. `UiChip` dit « ce filtre est
           appliqué, voici comment l'enlever » : quatre jetons posés d'avance se
           liraient comme quatre filtres actifs, c'est-à-dire l'inverse d'une
           liste non filtrée. -->
      <fieldset class="flex flex-wrap items-center gap-x-4 gap-y-2">
        <legend class="float-start me-3 text-sm text-text-muted">
          {{ t('admin.user.list.filters.status') }}
        </legend>
        <UiCheckbox
          v-for="status in STATUSES"
          :key="status"
          :model-value="filters.statuses.includes(status)"
          :label="t(`admin.user.status.${status}`)"
          :disabled="disabled"
          @update:model-value="toggleStatus(status)"
        />
      </fieldset>

      <UiSwitch
        :model-value="filters.without_role"
        :label="t('admin.user.list.filters.withoutRole')"
        :disabled="disabled"
        @update:model-value="patch({ without_role: $event })"
      />
      <UiSwitch
        :model-value="filters.without_account"
        :label="t('admin.user.list.filters.withoutAccount')"
        :disabled="disabled"
        @update:model-value="patch({ without_account: $event })"
      />

      <UiFormField :label="t('admin.user.list.filters.country')" class="ml-auto w-full sm:w-56">
        <UiSelect
          :model-value="filters.countries[0] ?? ''"
          :options="countryOptions"
          size="sm"
          :disabled="disabled"
          @update:model-value="patch({ countries: $event ? [$event] : [] })"
        />
      </UiFormField>
    </div>

    <p class="mt-3 text-sm text-text-muted" aria-live="polite">
      {{ t('admin.user.list.filters.count', { shown, total }) }}
      <button
        v-if="hasActiveUserFilters(filters)"
        type="button"
        class="ml-2 cursor-pointer text-accent underline underline-offset-2"
        @click="emit('update:filters', NO_USER_FILTERS)"
      >
        {{ t('admin.user.list.filters.reset') }}
      </button>
    </p>
  </section>
</template>
