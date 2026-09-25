<script setup lang="ts">
import type { AdminGlossaryRow, AdminKnowledgeStatus } from '~/types/admin-negotiation-savoir'
import type { EffectivePermission } from '~/types/identity'
import type { TaxonomyTerm } from '~/types/reference'
import type { TableColumn } from '~/types/ui'

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationGlossary' }],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const { date, timeWithZone } = useDateTime()

useHead(() => ({ title: t('admin.negociations.lexique.list.title') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canRead = computed(
  () =>
    hasPermission(granted.value, 'negotiation.knowledge.publish') ||
    hasPermission(granted.value, 'negotiation.knowledge.review'),
)

const texteDe = (valeur: unknown): string => (typeof valeur === 'string' ? valeur : '')

const filters = computed(() => ({
  q: texteDe(route.query.q),
  famille: texteDe(route.query.famille),
  etat: texteDe(route.query.etat),
}))
const filtered = computed(() => Boolean(filters.value.q || filters.value.famille || filters.value.etat))

function setFilter(patch: Record<string, string>): void {
  const next = { ...route.query }
  for (const [key, value] of Object.entries(patch)) {
    if (value) next[key] = value
    else delete next[key]
  }
  router.replace({ query: next })
}

const { data: screen, status, error, refresh } = await useAsyncData(
  'admin-negotiation-glossary',
  () =>
    api.adminNegotiationSavoir.lexique({
      q: filters.value.q || undefined,
      family: filters.value.famille || undefined,
      status: (filters.value.etat || undefined) as AdminKnowledgeStatus | undefined,
    }),
  { lazy: true, watch: [filters] },
)

const { data: familles } = await useAsyncData<TaxonomyTerm[]>(
  'reference-terms-glossary_family',
  () => api.reference.terms('glossary_family'),
  { default: () => [], lazy: true },
)

const isForbidden = computed(
  () => (!canRead.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)
const canPublish = computed(() => screen.value?.can_publish ?? false)
const rows = computed<AdminGlossaryRow[]>(() => screen.value?.entries ?? [])

const libellesDeFamille = computed(
  () => new Map(familles.value.map((f) => [f.code, resolveI18nText(f.label, locale.value)])),
)
const famille = (code: string): string => libellesDeFamille.value.get(code) ?? code

const familyOptions = computed(() => [
  { value: '', label: t('admin.negociations.lexique.list.filters.allFamilies') },
  ...familles.value.map((f) => ({ value: f.code, label: resolveI18nText(f.label, locale.value) })),
])

const stateOptions = computed(() => [
  { value: '', label: t('admin.negociations.lexique.list.filters.allStates') },
  ...KNOWLEDGE_STATUSES.map((s) => ({ value: s, label: t(`admin.negociations.lexique.state.${s}`) })),
])

const columns = computed<TableColumn[]>(() => [
  { key: 'term', label: t('admin.negociations.lexique.list.columns.term') },
  { key: 'translation', label: t('admin.negociations.lexique.list.columns.translation'), hideBelow: 'lg' },
  { key: 'family', label: t('admin.negociations.lexique.list.columns.family'), hideBelow: 'xl', width: '10rem' },
  { key: 'status', label: t('admin.negociations.lexique.list.columns.status'), width: '8rem' },
  { key: 'updated', label: t('admin.negociations.lexique.list.columns.updated'), hideBelow: 'xl', width: '11rem' },
])

function openEntry(row: AdminGlossaryRow): void {
  void navigateTo(localePath(`/admin/negociations/lexique/${row.id}`))
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.lexique.list.forbidden.scope')"
      :description="t('admin.negociations.lexique.list.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.negociations.lexique.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.lexique.list.subtitle') }}</p>
        </div>
        <UiButton v-if="canPublish" icon="plus" :to="localePath('/admin/negociations/lexique/nouveau')">
          {{ t('admin.negociations.lexique.list.new') }}
        </UiButton>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :description="apiErrorMessage(error, t)"
        :request-id="incidentReference(error) ?? undefined"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiEmptyState
        v-else-if="rows.length === 0 && !filtered && status !== 'pending'"
        class="mt-8"
        icon="list"
        :title="t('admin.negociations.lexique.list.empty.title')"
        :description="
          t(canPublish ? 'admin.negociations.lexique.list.empty.description' : 'admin.negociations.lexique.list.empty.reader')
        "
        :action-label="canPublish ? t('admin.negociations.lexique.list.new') : undefined"
        @action="navigateTo(localePath('/admin/negociations/lexique/nouveau'))"
      />

      <template v-else>
        <section
          class="mt-6 flex flex-wrap items-end gap-3"
          :aria-label="t('admin.negociations.lexique.list.filters.label')"
        >
          <UiSearchInput
            class="min-w-60 grow"
            :model-value="filters.q"
            :placeholder="t('admin.negociations.lexique.list.filters.search')"
            @update:model-value="(value: string) => setFilter({ q: value })"
          />
          <UiSelect
            :model-value="filters.famille"
            :label="t('admin.negociations.lexique.list.filters.family')"
            :options="familyOptions"
            hide-optional
            @update:model-value="(value: string) => setFilter({ famille: value })"
          />
          <UiSelect
            :model-value="filters.etat"
            :label="t('admin.negociations.lexique.list.filters.state')"
            :options="stateOptions"
            hide-optional
            @update:model-value="(value: string) => setFilter({ etat: value })"
          />
        </section>

        <UiTable
          class="mt-4"
          :columns="columns"
          :rows="rows"
          row-key="id"
          row-label-key="term"
          :caption="t('admin.negociations.lexique.list.caption')"
          :loading="status === 'pending'"
          @row-click="openEntry"
        >
          <template #cell-term="{ row }">
            <span class="font-medium">{{ row.term }}</span>
            <span v-if="row.acronym" class="ms-2 font-mono text-sm text-text-muted">{{ row.acronym }}</span>
          </template>
          <template #cell-translation="{ row }">{{ row.translation }}</template>
          <template #cell-family="{ row }">{{ famille(row.family_code) }}</template>
          <template #cell-status="{ row }">
            <UiBadge
              :intent="KNOWLEDGE_STATUS_INTENT[row.status]"
              :label="t(`admin.negociations.lexique.state.${row.status}`)"
            />
          </template>
          <template #cell-updated="{ row }">
            <span class="block">{{ date(row.updated_at, KNOWLEDGE_TIMEZONE) }}</span>
            <span class="block text-sm text-text-muted">{{ timeWithZone(row.updated_at, KNOWLEDGE_TIMEZONE) }}</span>
          </template>

          <template #empty>
            <UiEmptyState
              icon="search"
              filtered
              :title="t('admin.negociations.lexique.list.noResults.title')"
              :description="t('admin.negociations.lexique.list.noResults.description')"
              :action-label="t('admin.negociations.lexique.list.noResults.action')"
              @action="setFilter({ q: '', famille: '', etat: '' })"
            />
          </template>
        </UiTable>
      </template>
    </template>
  </div>
</template>
