<script setup lang="ts">
import type {
  AdminDocumentRow,
  AdminDocumentState,
  ExtractionState,
} from '~/types/admin-negotiation-documents'
import type { EffectivePermission } from '~/types/identity'
import type { TaxonomyTerm } from '~/types/reference'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES DOCUMENTS DE GUIDE NÉGO — la liste du back-office.
 *
 * Portée GLOBALE, comme les codes : publier ou corriger ouvre la liste, et
 * `hasPermission` sans identifiant d'édition n'accepte que cette portée. Le
 * bouton de création suit `can_publish`, que l'API rend : un expert lit, il ne
 * crée pas.
 *
 * Le libellé du type vient du vocabulaire `document_type`, jamais d'un fichier
 * de traduction : un administrateur peut le modifier.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationDocuments' }],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const { date, timeWithZone } = useDateTime()

useHead(() => ({ title: t('admin.negociations.documents.list.title') }))

/** Un document n'appartient à aucune édition : le fuseau est celui de qui lit. */
const timezone = computed<TimeZoneName>(
  () => (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC') as TimeZoneName,
)

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canRead = computed(
  () =>
    hasPermission(granted.value, 'negotiation.document.publish') ||
    hasPermission(granted.value, 'negotiation.correction.post'),
)

const { data: screen, status, error, refresh } = await useAsyncData(
  'admin-negotiation-documents',
  () => api.adminNegotiationDocuments.documents(),
  { lazy: true },
)

// Sans le vocabulaire, le type s'affiche par son code : la liste reste lisible.
const { data: types } = await useAsyncData<TaxonomyTerm[]>(
  'reference-terms-document_type',
  () => api.reference.terms('document_type'),
  { default: () => [], lazy: true },
)

const isForbidden = computed(
  () => (!canRead.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)
const canPublish = computed(() => screen.value?.can_publish ?? false)
const errorMessage = computed(() => (error.value ? apiErrorMessage(error.value, t) : ''))

/** Les paramètres sont en FRANÇAIS : ils apparaissent dans une URL qu'on partage. */
const filters = computed(() => ({
  etat: typeof route.query.etat === 'string' ? route.query.etat : '',
  type: typeof route.query.type === 'string' ? route.query.type : '',
  q: typeof route.query.q === 'string' ? route.query.q : '',
}))

function setFilter(patch: Record<string, string>): void {
  const next = { ...route.query }
  for (const [key, value] of Object.entries(patch)) {
    if (value) next[key] = value
    else delete next[key]
  }
  router.replace({ query: next })
}

const allRows = computed<AdminDocumentRow[]>(() => screen.value?.documents ?? [])
const filtered = computed(() => Boolean(filters.value.etat || filters.value.type || filters.value.q))

const rows = computed<AdminDocumentRow[]>(() => {
  const needle = normalizeSearch(filters.value.q)
  return allRows.value.filter(
    (row) =>
      (!filters.value.etat || row.state === filters.value.etat) &&
      (!filters.value.type || row.type === filters.value.type) &&
      (!needle || normalizeSearch(row.title).includes(needle)),
  )
})

const typeLabels = computed(
  () => new Map(types.value.map((term) => [term.code, resolveI18nText(term.label, locale.value)])),
)

function typeLabel(code: string): string {
  return typeLabels.value.get(code) ?? code
}

const STATES: AdminDocumentState[] = ['draft', 'published', 'unpublished']

const stateOptions = computed(() => [
  { value: '', label: t('admin.negociations.documents.list.filters.allStates') },
  ...STATES.map((etat) => ({ value: etat, label: t(`admin.negociations.documents.list.state.${etat}`) })),
])

/** Les types présents dans la liste seulement : un filtre qui ne ramène rien n'aide pas. */
const typeOptions = computed(() => [
  { value: '', label: t('admin.negociations.documents.list.filters.allTypes') },
  ...[...new Set(allRows.value.map((row) => row.type))]
    .map((code) => ({ value: code, label: typeLabel(code) }))
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
])

const columns = computed<TableColumn[]>(() => [
  { key: 'title', label: t('admin.negociations.documents.list.columns.title') },
  { key: 'type', label: t('admin.negociations.documents.list.columns.type'), hideBelow: 'lg', width: '11rem' },
  { key: 'version', label: t('admin.negociations.documents.list.columns.version'), hideBelow: 'sm', width: '7rem' },
  { key: 'state', label: t('admin.negociations.documents.list.columns.state'), width: '8rem' },
  { key: 'source', label: t('admin.negociations.documents.list.columns.source'), hideBelow: 'xl', width: '7rem' },
  { key: 'extraction', label: t('admin.negociations.documents.list.columns.extraction'), hideBelow: 'lg', width: '11rem' },
  { key: 'updated', label: t('admin.negociations.documents.list.columns.updated'), hideBelow: 'xl', width: '12rem' },
])

function extractionPages(extraction: ExtractionState): string {
  return extraction.page_count === null
    ? ''
    : t('admin.negociations.documents.list.pages', { count: extraction.page_count }, extraction.page_count)
}

function openDocument(row: AdminDocumentRow): void {
  void navigateTo(localePath(`/admin/negociations/documents/${row.id}`))
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.documents.list.forbidden.scope')"
      :description="t('admin.negociations.documents.list.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.negociations.documents.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.negociations.documents.list.subtitle') }}
          </p>
        </div>
        <UiButton v-if="canPublish" icon="plus" :to="localePath('/admin/negociations/documents/nouveau')">
          {{ t('admin.negociations.documents.list.new') }}
        </UiButton>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :description="errorMessage"
        :request-id="incidentReference(error) ?? undefined"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <template v-else-if="allRows.length === 0 && !filtered && status !== 'pending'">
        <UiEmptyState
          v-if="canPublish"
          class="mt-8"
          icon="document"
          :title="t('admin.negociations.documents.list.empty.title')"
          :description="t('admin.negociations.documents.list.empty.description')"
          :action-label="t('admin.negociations.documents.list.new')"
          @action="navigateTo(localePath('/admin/negociations/documents/nouveau'))"
        />
        <UiEmptyState
          v-else
          class="mt-8"
          icon="document"
          :title="t('admin.negociations.documents.list.empty.title')"
          :description="t('admin.negociations.documents.list.empty.reader')"
        />
      </template>

      <template v-else>
        <section
          class="mt-6 flex flex-wrap items-end gap-3"
          :aria-label="t('admin.negociations.documents.list.filters.label')"
        >
          <UiSearchInput
            class="min-w-60 grow"
            :model-value="filters.q"
            :placeholder="t('admin.negociations.documents.list.filters.search')"
            :disabled="status === 'pending'"
            @update:model-value="(value: string) => setFilter({ q: value })"
          />
          <UiFormField :label="t('admin.negociations.documents.list.filters.state')" hide-optional>
            <template #default="{ control }">
              <UiSelect
                v-bind="control"
                :model-value="filters.etat"
                :options="stateOptions"
                :disabled="status === 'pending'"
                @update:model-value="(value: string) => setFilter({ etat: value })"
              />
            </template>
          </UiFormField>
          <UiFormField :label="t('admin.negociations.documents.list.filters.type')" hide-optional>
            <template #default="{ control }">
              <UiSelect
                v-bind="control"
                :model-value="filters.type"
                :options="typeOptions"
                :disabled="status === 'pending'"
                @update:model-value="(value: string) => setFilter({ type: value })"
              />
            </template>
          </UiFormField>
        </section>

        <UiTable
          class="mt-4"
          :columns="columns"
          :rows="rows"
          row-key="id"
          row-label-key="title"
          :caption="t('admin.negociations.documents.list.caption')"
          :loading="status === 'pending'"
          @row-click="openDocument"
        >
          <template #cell-title="{ row }">
            <span class="font-medium">{{ row.title }}</span>
            <UiBadge
              v-if="row.restricted"
              class="ms-2 align-middle"
              size="sm"
              icon="lock"
              :label="t('admin.negociations.documents.list.restricted')"
            />
            <span v-if="row.supersedes" class="mt-0.5 block text-sm text-text-muted">
              {{
                t('admin.negociations.documents.list.supersedes', {
                  title: row.supersedes.title,
                  version: row.supersedes.version,
                })
              }}
            </span>
            <span v-if="row.superseded_by" class="mt-0.5 block text-sm text-text-muted">
              {{
                t('admin.negociations.documents.list.supersededBy', {
                  title: row.superseded_by.title,
                  version: row.superseded_by.version,
                })
              }}
            </span>
          </template>

          <template #cell-type="{ row }">{{ typeLabel(row.type) }}</template>

          <template #cell-version="{ row }">
            <span class="font-mono text-sm">{{ row.version }}</span>
          </template>

          <template #cell-state="{ row }">
            <UiBadge
              :intent="DOCUMENT_STATE_INTENT[row.state]"
              :label="t(`admin.negociations.documents.list.state.${row.state}`)"
            />
          </template>

          <template #cell-source="{ row }">
            <span v-if="row.source">{{ t(`admin.negociations.documents.list.source.${row.source}`) }}</span>
            <span v-else class="text-text-muted">{{ t('admin.negociations.documents.list.source.none') }}</span>
          </template>

          <template #cell-extraction="{ row }">
            <template v-if="row.extraction">
              <UiBadge
                :intent="EXTRACTION_STATUS_INTENT[row.extraction.status]"
                :label="t(`admin.negociations.documents.list.extraction.${row.extraction.status}`)"
              />
              <span
                v-if="row.extraction.serve_as_is || row.extraction.page_count !== null"
                class="mt-1 block text-sm text-text-muted"
              >
                <template v-if="row.extraction.serve_as_is">
                  {{ t('admin.negociations.documents.list.extraction.asIs') }}
                </template>
                <template v-if="row.extraction.serve_as_is && row.extraction.page_count !== null"> · </template>
                {{ extractionPages(row.extraction) }}
              </span>
            </template>
            <span v-else-if="row.source !== 'link'" class="text-text-muted">
              {{ t('admin.negociations.documents.list.extraction.none') }}
            </span>
          </template>

          <template #cell-updated="{ row }">
            <span class="block">{{ date(row.updated_at, timezone) }}</span>
            <span class="block text-sm text-text-muted">{{ timeWithZone(row.updated_at, timezone) }}</span>
          </template>

          <template #empty>
            <UiEmptyState
              icon="search"
              filtered
              :title="t('admin.negociations.documents.list.noResults.title')"
              :description="
                t('admin.negociations.documents.list.noResults.description', { total: allRows.length })
              "
              :action-label="t('admin.negociations.documents.list.noResults.action')"
              @action="setFilter({ etat: '', type: '', q: '' })"
            />
          </template>
        </UiTable>
      </template>
    </template>
  </div>
</template>
