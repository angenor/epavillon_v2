<script setup lang="ts">
import type { AdminDocument, AdminDocumentPreview, AdminPreviewPage } from '~/types/admin-negotiation-documents'
import type { EffectivePermission } from '~/types/identity'

/**
 * L'APERÇU D'UN DOCUMENT DE GUIDE NÉGO — feuilleter le guide page par page,
 * l'image à côté de la forme lisible, avant de le publier (SC-001 bis).
 *
 * Portée globale seulement, comme tout le back-office de Guide Négo : l'API
 * refuse en 403, et l'écran le dit sans attendre quand aucune des deux
 * permissions n'est détenue.
 *
 * Seule la page courante se rend : le guide en compte 92.
 *
 * L'expert y pose ses notes de correction, sur la page feuilletée et le passage
 * sélectionné dans la forme lisible.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationDocuments', to: '/admin/negociations/documents' },
    { labelKey: 'admin.negociations.documents.preview.breadcrumb.document' },
    { labelKey: 'admin.negociations.documents.preview.breadcrumb.preview' },
  ],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const apiUrl = useApiUrl()

const id = computed(() => String(route.params.id))
const fichePath = computed(() => localePath(`/admin/negociations/documents/${id.value}`))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const allowed = computed(
  () =>
    hasPermission(granted.value, 'negotiation.document.publish') ||
    hasPermission(granted.value, 'negotiation.correction.post'),
)

/** Un document supprimé et une adresse forgée se confondent, comme sur la fiche. */
async function orNotFound<T>(request: Promise<T>): Promise<T | null> {
  try {
    return await request
  } catch (error) {
    if (error instanceof ApiRequestError && error.status === 404) return null
    throw error
  }
}

const {
  data: doc,
  status: docStatus,
  error: docError,
  refresh: refreshDoc,
} = await useAsyncData<AdminDocument | null>(
  () => `admin-negotiation-document-preview-doc-${id.value}`,
  () => orNotFound(api.adminNegotiationDocuments.document(id.value)),
  { default: () => null, lazy: true },
)

const {
  data: preview,
  status: previewStatus,
  error: previewError,
  refresh: refreshPreview,
} = await useAsyncData<AdminDocumentPreview | null>(
  () => `admin-negotiation-document-preview-${id.value}`,
  () => orNotFound(api.adminNegotiationDocuments.apercu(id.value)),
  { default: () => null, lazy: true },
)

const forbidden = computed(
  () =>
    (!allowed.value && permissionStatus.value !== 'pending') ||
    isForbiddenError(docError.value) ||
    isForbiddenError(previewError.value),
)
const loadError = computed(() => docError.value ?? previewError.value)
const notFound = computed(
  () =>
    !loadError.value &&
    ((docStatus.value === 'success' && !doc.value) || (previewStatus.value === 'success' && !preview.value)),
)
const loading = computed(
  () => (docStatus.value === 'pending' && !doc.value) || (previewStatus.value === 'pending' && !preview.value),
)

const title = computed(() => resolveI18nText(doc.value?.title, locale.value))
useHead(() => ({
  title: title.value
    ? t('admin.negociations.documents.preview.titleOf', { title: title.value })
    : t('admin.negociations.documents.preview.title'),
}))

function retry(): void {
  void refreshDoc()
  void refreshPreview()
}

// « Ouvrir tel quel » — `send` n'est jamais rejoué : un échec se dit, il ne se retente pas seul.
const extraction = computed(() => preview.value?.extraction ?? doc.value?.extraction ?? null)
const savingAsIs = ref(false)
const asIsError = ref<string | null>(null)

async function setServeAsIs(value: boolean): Promise<void> {
  if (savingAsIs.value) return
  savingAsIs.value = true
  asIsError.value = null
  try {
    doc.value = await api.adminNegotiationDocuments.ouvrirTelQuel(id.value, value)
    await refreshPreview()
  } catch (error) {
    asIsError.value = apiErrorMessage(error, t)
  } finally {
    savingAsIs.value = false
  }
}

// La page courante vit dans l'URL : un lien partagé rouvre la même page.
const pages = computed<AdminPreviewPage[]>(() => preview.value?.pages ?? [])
const labels = computed(() => new Map(pages.value.map((page) => [page.index, page.label])))

const requestedIndex = computed(() => {
  const raw = route.query.page
  const parsed = Number.parseInt(typeof raw === 'string' ? raw : '', 10)
  return Number.isFinite(parsed) ? parsed : 1
})

const position = computed(() => {
  const list = pages.value
  if (list.length === 0) return -1
  const found = list.findIndex((page) => page.index === requestedIndex.value)
  if (found >= 0) return found
  return requestedIndex.value > (list[list.length - 1]?.index ?? 0) ? list.length - 1 : 0
})

const current = computed<AdminPreviewPage | null>(() => pages.value[position.value] ?? null)

function goTo(index: number): void {
  if (index === current.value?.index) return
  void router.replace({ query: { ...route.query, page: String(index) } })
}

function step(delta: number): void {
  const next = pages.value[position.value + delta]
  if (next) goTo(next.index)
}

const pageOptions = computed(() =>
  pages.value.map((page) => ({
    value: String(page.index),
    label: t('admin.negociations.documents.preview.pager.option', { index: page.index, label: page.label }),
  })),
)

const FIELD_SELECTOR =
  'input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="listbox"], [role="menu"], [role="slider"], [role="tablist"]'

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
  if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return
  const target = event.target
  if (target instanceof Element && target.closest(FIELD_SELECTOR)) return
  if (pages.value.length === 0) return
  event.preventDefault()
  step(event.key === 'ArrowLeft' ? -1 : 1)
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))

// L'image de la page : absolue par l'API, absente du jeu d'exemple.
const imageSrc = computed(() => (current.value?.image ? apiUrl(current.value.image) : null))
const imageFailed = ref(false)
watch(imageSrc, () => {
  imageFailed.value = false
})

const imageFrame = ref<HTMLElement | null>(null)
const readingBox = ref<HTMLElement | null>(null)

function showOrigin(): void {
  imageFrame.value?.scrollIntoView({ behavior: 'smooth', block: 'start' })
  imageFrame.value?.focus({ preventScroll: true })
}

const imageMessage = computed(() => {
  if (!current.value?.image) return t('admin.negociations.documents.preview.image.missing')
  if (!imageSrc.value) return t('admin.negociations.documents.preview.image.offline')
  return t('admin.negociations.documents.preview.image.failed')
})

const notReadyMessage = computed(() => {
  const e = extraction.value
  if (!e) return t('admin.negociations.documents.preview.notReady.noFile')
  return t('admin.negociations.documents.preview.notReady.description', {
    state: t(`admin.negociations.documents.list.extraction.${e.status}`),
  })
})
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="forbidden"
      :required-scope="t('admin.negociations.documents.preview.forbidden.scope')"
      :description="t('admin.negociations.documents.preview.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="space-y-3">
        <UiButton variant="ghost" icon="arrow-left" :to="fichePath">
          {{ t('admin.negociations.documents.preview.back') }}
        </UiButton>
        <div class="min-w-0">
          <UiSkeletonLoader v-if="loading && !title" width="60%" height="2.25rem" />
          <h1 v-else class="text-3xl leading-tight font-semibold text-balance">
            {{ title || t('admin.negociations.documents.preview.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.negociations.documents.preview.subtitle') }}
          </p>
        </div>
      </header>

      <UiErrorState
        v-if="loadError"
        class="mt-8"
        :title="t('admin.negociations.documents.preview.error.title')"
        :description="apiErrorMessage(loadError, t)"
        :retry-label="t('common.actions.retry')"
        :retrying="loading"
        @retry="retry"
      />

      <UiEmptyState
        v-else-if="notFound"
        class="mt-8"
        icon="search"
        :title="t('admin.negociations.documents.form.detail.notFound.title')"
        :description="t('admin.negociations.documents.form.detail.notFound.description')"
        :action-label="t('admin.negociations.documents.form.backToList')"
        :action-to="localePath('/admin/negociations/documents')"
      />

      <div v-else-if="loading" class="mt-6 space-y-6" aria-busy="true">
        <UiSkeletonLoader height="9rem" />
        <div class="grid gap-6 lg:grid-cols-2">
          <UiSkeletonLoader height="32rem" />
          <UiSkeletonLoader variant="text" :lines="12" />
        </div>
      </div>

      <template v-else-if="preview">
        <AdminNegotiationPreviewVerdict
          class="mt-6"
          :extraction="extraction"
          :quality="preview.quality"
          :extractor="preview.extractor"
          :can-publish="doc?.can_publish ?? false"
          :saving="savingAsIs"
          :save-error="asIsError"
          @update:serve-as-is="setServeAsIs"
        />

        <UiEmptyState
          v-if="!current"
          class="mt-8"
          icon="document"
          :title="t('admin.negociations.documents.preview.notReady.title')"
          :description="notReadyMessage"
          :action-label="t('admin.negociations.documents.preview.notReady.action')"
          :action-to="fichePath"
        />

        <div v-else class="mt-8 grid gap-6 xl:grid-cols-[17rem_minmax(0,1fr)]">
          <aside
            class="max-h-80 overflow-y-auto rounded-lg border border-border bg-surface-raised p-3 xl:sticky xl:top-4 xl:max-h-[calc(100vh-2rem)] xl:self-start"
          >
            <AdminNegotiationPreviewOutline
              :outline="preview.outline"
              :labels="labels"
              :current-index="current.index"
              @go="goTo"
            />
          </aside>

          <section class="min-w-0">
            <nav
              class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-border bg-surface py-3 lg:sticky lg:top-(--admin-header-height) lg:z-10"
              :aria-label="t('admin.negociations.documents.preview.pager.label')"
            >
              <div class="flex shrink-0 items-center gap-2">
                <UiButton
                  variant="secondary"
                  icon="chevron-left"
                  icon-only
                  :label="t('admin.negociations.documents.preview.pager.previous')"
                  :disabled="position <= 0"
                  @click="step(-1)"
                />
                <div class="w-40 sm:w-48">
                  <UiSelect
                    :label="t('admin.negociations.documents.preview.pager.select')"
                    hide-label
                    hide-optional
                    :model-value="String(current.index)"
                    :options="pageOptions"
                    @update:model-value="(value: string) => goTo(Number(value))"
                  />
                </div>
                <UiButton
                  variant="secondary"
                  icon="chevron-right"
                  icon-only
                  :label="t('admin.negociations.documents.preview.pager.next')"
                  :disabled="position >= pages.length - 1"
                  @click="step(1)"
                />
              </div>
              <p class="min-w-0 flex-1 basis-48 text-sm" aria-live="polite">
                <strong class="font-semibold">
                  {{ t('admin.negociations.documents.preview.pager.position', { index: current.index, total: pages.length }) }}
                </strong>
                <span class="text-text-muted">
                  · {{ t('admin.negociations.documents.preview.pager.printed', { label: current.label }) }}
                </span>
                <span class="mt-0.5 hidden text-xs text-text-muted lg:block">
                  {{ t('admin.negociations.documents.preview.pager.keyboard') }}
                </span>
              </p>
            </nav>

            <div class="mt-4 grid gap-6 lg:grid-cols-2">
              <figure
                ref="imageFrame"
                tabindex="-1"
                class="min-w-0 scroll-mt-4 rounded-lg lg:scroll-mt-40 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-focus)"
              >
                <figcaption class="mb-2 text-sm font-semibold tracking-caps text-text-muted uppercase">
                  {{ t('admin.negociations.documents.preview.image.title') }}
                </figcaption>
                <img
                  v-if="imageSrc && !imageFailed"
                  :key="imageSrc"
                  :src="imageSrc"
                  :alt="t('admin.negociations.documents.preview.image.alt', { label: current.label })"
                  class="h-auto w-full rounded-md border border-border bg-surface-sunken"
                  @error="imageFailed = true"
                >
                <div
                  v-else
                  class="flex aspect-[1/1.414] w-full items-center justify-center rounded-md border border-dashed border-border-strong bg-surface-sunken p-6 text-center text-sm text-text-muted"
                >
                  <p class="max-w-xs">
                    <UiIcon name="image" class="mx-auto mb-2 block" />
                    {{ imageMessage }}
                  </p>
                </div>
              </figure>

              <article class="min-w-0">
                <header class="mb-2 flex flex-wrap items-baseline justify-between gap-2">
                  <h2 class="text-sm font-semibold tracking-caps text-text-muted uppercase">
                    {{ t('admin.negociations.documents.preview.reading.title') }}
                  </h2>
                  <p class="flex flex-wrap gap-x-3 text-xs text-text-muted">
                    <span class="sr-only">{{ t('admin.negociations.documents.preview.reading.legend') }}</span>
                    <em>{{ t('admin.negociations.documents.preview.reading.italic') }}</em>
                    <em class="rounded-sm bg-accent-surface px-0.5 underline decoration-accent decoration-dotted underline-offset-4">
                      {{ t('admin.negociations.documents.preview.reading.term') }}
                    </em>
                  </p>
                </header>
                <div ref="readingBox" class="rounded-lg border border-border bg-surface-raised p-4 sm:p-6">
                  <AdminNegotiationPreviewBlocks
                    v-if="current.blocks.length"
                    :key="current.index"
                    :blocks="current.blocks"
                    @show-origin="showOrigin"
                  />
                  <p v-else class="text-text-muted">
                    {{ t('admin.negociations.documents.preview.reading.empty') }}
                  </p>
                </div>
                <AdminNegotiationPreviewNotes
                  class="mt-8"
                  :document-id="id"
                  :page="current"
                  :labels="labels"
                  :source="readingBox"
                  @go="goTo"
                />
              </article>
            </div>
          </section>
        </div>
      </template>
    </template>
  </div>
</template>
