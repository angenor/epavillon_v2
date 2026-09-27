<script setup lang="ts">
import type { AdminFaqRow, AdminKnowledgeStatus } from '~/types/admin-negotiation-savoir'
import type { EffectivePermission } from '~/types/identity'
import type { TaxonomyTerm } from '~/types/reference'
import type { TableColumn } from '~/types/ui'

// Filtrée par l'API (`q` par trigramme) : la liste ne se refiltre pas ici.

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationFaq' }],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const { date } = useDateTime()

useHead(() => ({ title: t('admin.negociations.faq.list.title') }))

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
  rubrique: texteDe(route.query.rubrique),
  etat: texteDe(route.query.etat),
}))
const filtered = computed(() => Boolean(filters.value.q || filters.value.rubrique || filters.value.etat))

function setFilter(patch: Record<string, string>): void {
  const next = { ...route.query }
  for (const [key, value] of Object.entries(patch)) {
    if (value) next[key] = value
    else delete next[key]
  }
  router.replace({ query: next })
}

const { data: screen, status, error, refresh } = await useAsyncData(
  'admin-negotiation-faq',
  () =>
    api.adminNegotiationSavoir.faq({
      q: filters.value.q || undefined,
      section: filters.value.rubrique || undefined,
      status: (filters.value.etat || undefined) as AdminKnowledgeStatus | undefined,
    }),
  { lazy: true, watch: [filters] },
)

const { data: rubriques } = await useAsyncData<TaxonomyTerm[]>(
  'reference-terms-faq_section',
  () => api.reference.terms('faq_section'),
  { default: () => [], lazy: true },
)

const isForbidden = computed(
  () => (!canRead.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)
const canPublish = computed(() => screen.value?.can_publish ?? false)
const rows = computed<AdminFaqRow[]>(() => screen.value?.entries ?? [])

const libellesDeRubrique = computed(
  () => new Map(rubriques.value.map((r) => [r.code, resolveI18nText(r.label, locale.value)])),
)
const rubrique = (code: string): string => libellesDeRubrique.value.get(code) ?? code

const sectionOptions = computed(() => [
  { value: '', label: t('admin.negociations.faq.list.filters.allSections') },
  ...rubriques.value.map((r) => ({ value: r.code, label: resolveI18nText(r.label, locale.value) })),
])

const stateOptions = computed(() => [
  { value: '', label: t('admin.negociations.faq.list.filters.allStates') },
  ...KNOWLEDGE_STATUSES.map((s) => ({ value: s, label: t(`admin.negociations.faq.state.${s}`) })),
])

const columns = computed<TableColumn[]>(() => [
  { key: 'question', label: t('admin.negociations.faq.list.columns.question') },
  { key: 'section', label: t('admin.negociations.faq.list.columns.section'), hideBelow: 'lg', width: '14rem' },
  { key: 'status', label: t('admin.negociations.faq.list.columns.status'), width: '8rem' },
  { key: 'verified', label: t('admin.negociations.faq.list.columns.verified'), hideBelow: 'lg', width: '11rem' },
  { key: 'reports', label: t('admin.negociations.faq.list.columns.reports'), align: 'end', hideBelow: 'sm', width: '8rem' },
])

function openEntry(row: AdminFaqRow): void {
  void navigateTo(localePath(`/admin/negociations/faq/${row.id}`))
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.faq.list.forbidden.scope')"
      :description="t('admin.negociations.faq.list.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.negociations.faq.list.title') }}</h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.faq.list.subtitle') }}</p>
        </div>
        <UiButton v-if="canPublish" icon="plus" :to="localePath('/admin/negociations/faq/nouveau')">
          {{ t('admin.negociations.faq.list.new') }}
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
        icon="info"
        :title="t('admin.negociations.faq.list.empty.title')"
        :description="t(canPublish ? 'admin.negociations.faq.list.empty.description' : 'admin.negociations.faq.list.empty.reader')"
        :action-label="canPublish ? t('admin.negociations.faq.list.new') : undefined"
        @action="navigateTo(localePath('/admin/negociations/faq/nouveau'))"
      />

      <template v-else>
        <section class="mt-6 flex flex-wrap items-end gap-3" :aria-label="t('admin.negociations.faq.list.filters.label')">
          <UiSearchInput
            class="min-w-60 grow"
            :model-value="filters.q"
            :placeholder="t('admin.negociations.faq.list.filters.search')"
            @update:model-value="(value: string) => setFilter({ q: value })"
          />
          <UiSelect
            :model-value="filters.rubrique"
            :label="t('admin.negociations.faq.list.filters.section')"
            :options="sectionOptions"
            hide-optional
            @update:model-value="(value: string) => setFilter({ rubrique: value })"
          />
          <UiSelect
            :model-value="filters.etat"
            :label="t('admin.negociations.faq.list.filters.state')"
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
          row-label-key="question"
          :caption="t('admin.negociations.faq.list.caption')"
          :loading="status === 'pending'"
          @row-click="openEntry"
        >
          <template #cell-question="{ row }">
            <span class="font-medium">{{ row.question }}</span>
            <span v-if="!row.has_answer" class="mt-0.5 block text-sm text-text-muted">
              {{ t('admin.negociations.faq.list.noAnswer') }}
            </span>
          </template>
          <template #cell-section="{ row }">{{ rubrique(row.section_code) }}</template>
          <template #cell-status="{ row }">
            <UiBadge
              :intent="KNOWLEDGE_STATUS_INTENT[row.status]"
              :label="t(`admin.negociations.faq.state.${row.status}`)"
            />
          </template>
          <template #cell-verified="{ row }">
            <span v-if="row.verified_on">{{ date(row.verified_on, 'UTC') }}</span>
            <span v-else class="text-text-muted">{{ t('admin.negociations.faq.list.notVerified') }}</span>
          </template>
          <template #cell-reports="{ row }">
            <UiBadge
              v-if="row.open_reports > 0"
              intent="warning"
              :label="t('admin.negociations.faq.list.openReports', { count: row.open_reports }, row.open_reports)"
            />
            <span v-else class="text-text-muted">0</span>
          </template>

          <template #empty>
            <UiEmptyState
              icon="search"
              filtered
              :title="t('admin.negociations.faq.list.noResults.title')"
              :description="t('admin.negociations.faq.list.noResults.description')"
              :action-label="t('admin.negociations.faq.list.noResults.action')"
              @action="setFilter({ q: '', rubrique: '', etat: '' })"
            />
          </template>
        </UiTable>
      </template>
    </template>
  </div>
</template>
