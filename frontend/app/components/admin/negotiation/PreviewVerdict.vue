<script setup lang="ts">
import type { ExtractionQuality, ExtractionState } from '~/types/admin-negotiation-documents'
import type { TimeZoneName } from '~/types/shared'

interface Props {
  extraction: ExtractionState | null
  quality: ExtractionQuality | null
  extractor: string | null
  canPublish: boolean
  saving?: boolean
  saveError?: string | null
}

const props = defineProps<Props>()
defineEmits<{ 'update:largeTextChoice': [choice: boolean | null] }>()

const { t, locale } = useI18n()
const { date, timeWithZone } = useDateTime()

const timezone = computed<TimeZoneName>(
  () => (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC') as TimeZoneName,
)

const number = (value: number): string => new Intl.NumberFormat(locale.value).format(value)

const facts = computed(() => {
  const e = props.extraction
  if (!e) return []
  const unknown = t('admin.negociations.documents.preview.verdict.unknown')
  return [
    {
      key: 'pages',
      label: t('admin.negociations.documents.preview.verdict.pages'),
      value: e.page_count === null ? unknown : number(e.page_count),
    },
    {
      key: 'bytes',
      label: t('admin.negociations.documents.preview.verdict.readingBytes'),
      value: e.reading_bytes === null ? unknown : formatByteSize(e.reading_bytes, locale.value),
    },
    {
      key: 'extracted',
      label: t('admin.negociations.documents.preview.verdict.extractedAt'),
      value: e.extracted_at
        ? t('admin.negociations.documents.preview.verdict.extractedAtValue', {
            date: date(e.extracted_at, timezone.value),
            time: timeWithZone(e.extracted_at, timezone.value),
          })
        : unknown,
    },
    {
      key: 'extractor',
      label: t('admin.negociations.documents.preview.verdict.extractor'),
      value: props.extractor ?? unknown,
    },
  ]
})

const indicators = computed(() => {
  const q = props.quality
  if (!q) return []
  const label = (key: string): string => t(`admin.negociations.documents.preview.quality.${key}`)
  return [
    { key: 'pages', label: label('pages'), value: number(q.pages) },
    { key: 'pagesWithText', label: label('pagesWithText'), value: number(q.pages_avec_texte) },
    { key: 'pagesWithOrigin', label: label('pagesWithOrigin'), value: number(q.pages_a_origine) },
    { key: 'tables', label: label('tables'), value: number(q.tableaux) },
    { key: 'figures', label: label('figures'), value: number(q.figures) },
    { key: 'notes', label: label('notes'), value: number(q.notes) },
    { key: 'headings', label: label('headings'), value: number(q.titres) },
    { key: 'outlineEntries', label: label('outlineEntries'), value: number(q.entrees_du_sommaire) },
    {
      key: 'outlineSource',
      label: label('outlineSource'),
      value: label(q.sommaire_depuis_signets ? 'fromBookmarks' : 'fromHeadings'),
    },
    { key: 'thinItalics', label: label('thinItalics'), value: number(q.italiques_maigres) },
    { key: 'terms', label: label('terms'), value: number(q.termes) },
    { key: 'hyphensKept', label: label('hyphensKept'), value: number(q.cesures_gardees) },
    { key: 'hyphensJoined', label: label('hyphensJoined'), value: number(q.cesures_recollees) },
  ]
})
</script>

<template>
  <section
    class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5"
    :aria-label="t('admin.negociations.documents.preview.verdict.label')"
  >
    <div class="flex flex-wrap items-start justify-between gap-x-8 gap-y-4">
      <div class="min-w-0 space-y-3">
        <h2 class="text-sm font-semibold tracking-caps text-text-muted uppercase">
          {{ t('admin.negociations.documents.preview.verdict.title') }}
        </h2>
        <p v-if="!extraction" class="text-text-muted">
          {{ t('admin.negociations.documents.list.extraction.none') }}
        </p>
        <div v-else class="flex flex-wrap gap-2">
          <UiBadge
            :intent="EXTRACTION_STATUS_INTENT[extraction.status]"
            :label="t(`admin.negociations.documents.list.extraction.${extraction.status}`)"
          />
          <UiBadge
            v-if="extraction.is_reflowable !== null"
            :intent="extraction.is_reflowable ? 'success' : 'warning'"
            :label="
              t(
                extraction.is_reflowable
                  ? 'admin.negociations.documents.preview.verdict.reflowable'
                  : 'admin.negociations.documents.preview.verdict.notReflowable',
              )
            "
          />
          <UiBadge
            :intent="extraction.large_text ? 'info' : 'neutral'"
            :label="
              t(
                extraction.large_text
                  ? 'admin.negociations.documents.preview.verdict.largeTextOffered'
                  : 'admin.negociations.documents.preview.verdict.largeTextWithheld',
              )
            "
          />
        </div>
      </div>

      <div v-if="extraction" class="w-full max-w-md">
        <AdminNegotiationLargeTextChoice
          :extraction="extraction"
          :can-publish="canPublish"
          :saving="saving"
          @update:choice="(choice: boolean | null) => $emit('update:largeTextChoice', choice)"
        />
        <UiAlert
          v-if="saveError"
          class="mt-3"
          intent="danger"
          compact
          :title="t('admin.negociations.documents.preview.largeText.failed')"
          :message="saveError"
        />
      </div>
    </div>

    <UiAlert
      v-if="extraction?.status === 'failed'"
      class="mt-4"
      intent="danger"
      :title="t('admin.negociations.documents.preview.verdict.failed')"
      :message="extraction.failure_reason ?? undefined"
    />

    <dl v-if="facts.length" class="mt-4 grid gap-x-6 gap-y-3 sm:grid-cols-2 xl:grid-cols-4">
      <div v-for="fact in facts" :key="fact.key" class="min-w-0">
        <dt class="text-xs text-text-muted">{{ fact.label }}</dt>
        <dd class="mt-0.5 break-words">{{ fact.value }}</dd>
      </div>
    </dl>

    <template v-if="indicators.length">
      <h3 class="mt-5 text-sm font-semibold text-text-secondary">
        {{ t('admin.negociations.documents.preview.quality.title') }}
      </h3>
      <dl class="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-5 xl:grid-cols-7">
        <div
          v-for="item in indicators"
          :key="item.key"
          class="rounded-md border border-border-subtle bg-surface px-3 py-2"
        >
          <dt class="text-xs text-text-muted">{{ item.label }}</dt>
          <dd class="mt-0.5 text-lg font-semibold tabular-nums">{{ item.value }}</dd>
        </div>
      </dl>
    </template>
  </section>
</template>
