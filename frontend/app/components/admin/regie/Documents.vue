<script setup lang="ts">
import type { RegieDocument } from '~/types/admin-regie'
import type { TimeZoneName } from '~/types/shared'

interface Props {
  documents: RegieDocument[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()
const { t } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()

const rows = computed(() =>
  props.documents.map((document) => {
    const kind = documentKind(document.mime_type, document.filename)
    const parts = [t(`admin.regie.documents.kind.${kind}`)]
    if (document.byte_size !== null) parts.push(formatByteSize(document.byte_size, dt.intlLocale.value))
    parts.push(t('admin.regie.documents.uploaded', { date: dt.date(document.uploaded_at, props.timezone) }))
    return {
      document,
      title: tr(document.title) || document.filename || t('admin.regie.documents.untitled'),
      badge: t(`admin.regie.documents.badge.${kind}`),
      meta: new Intl.ListFormat(dt.intlLocale.value, { type: 'unit', style: 'short' }).format(parts),
    }
  }),
)
</script>

<template>
  <div class="flex flex-wrap gap-2 border-t border-border-subtle bg-surface-sunken px-4 py-3.5 sm:px-5">
    <p class="w-full text-xs font-bold tracking-caps text-text-muted uppercase">{{ t('admin.regie.documents.title') }}</p>
    <p v-if="!rows.length" class="text-sm font-semibold text-warning">{{ t('admin.regie.documents.none') }}</p>
    <template v-for="row in rows" :key="row.document.id">
      <a
        v-if="row.document.url"
        :href="row.document.url"
        target="_blank"
        rel="noopener noreferrer"
        class="group inline-flex min-h-(--target-min) max-w-full items-center gap-2.5 rounded-md border border-border-subtle bg-surface-raised py-1.5 pr-3 pl-2 text-text no-underline transition-colors duration-(--duration-fast) hover:border-accent-solid"
      >
        <i class="grid size-9 shrink-0 place-items-center rounded-sm bg-surface-inverse font-condensed text-xs font-bold text-text-on-inverse not-italic">{{ row.badge }}</i>
        <span class="min-w-0">
          <b class="block truncate text-sm leading-tight">{{ row.title }}</b>
          <small class="text-xs text-text-muted">{{ row.meta }}</small>
        </span>
        <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
      </a>
      <span
        v-else
        class="inline-flex min-h-(--target-min) max-w-full items-center gap-2.5 rounded-md border border-dashed border-border py-1.5 pr-3 pl-2 text-text-muted"
      >
        <i class="grid size-9 shrink-0 place-items-center rounded-sm bg-neutral-surface font-condensed text-xs font-bold not-italic">{{ row.badge }}</i>
        <span class="min-w-0">
          <b class="block truncate text-sm leading-tight">{{ row.title }}</b>
          <small class="text-xs">{{ t('admin.regie.documents.notReady') }}</small>
        </span>
      </span>
    </template>
  </div>
</template>
