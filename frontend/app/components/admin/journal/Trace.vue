<script setup lang="ts">
import type { JournalRequestTrace } from '~/types/admin-journal'

/**
 * « Même demande, dans l'ordre » : les lignes du journal, les écritures et les
 * événements émis pour un code de demande, sur un seul fil. C'est ce qui
 * répond à « qu'est-ce qui a été enregistré avant l'erreur ? ».
 */

const props = defineProps<{ trace: JournalRequestTrace; currentEntryId?: string | null }>()

const { t } = useI18n()
const labels = useJournalLabels()

const items = computed(() => journalTraceItems(props.trace))

function fieldsText(fields: { name: string; masked: boolean }[]): string {
  return fields
    .map((field) =>
      field.masked
        ? t('admin.journal.changes.maskedField', { name: labels.fieldLabel(field.name) })
        : labels.fieldLabel(field.name),
    )
    .join(', ')
}
</script>

<template>
  <ol class="mt-3 border-l-2 border-border">
    <li
      v-for="item in items"
      :key="item.key"
      class="relative pb-3 pl-4 text-sm before:absolute before:top-1 before:-left-[6px] before:size-2.5 before:rounded-full"
      :class="
        item.kind === 'entry' && item.entry.level === 'error'
          ? 'before:bg-danger'
          : item.kind === 'entry' && item.entry.level === 'warn'
            ? 'before:bg-warning'
            : 'before:bg-border-strong'
      "
    >
      <template v-if="item.kind === 'entry'">
        <span class="block font-semibold" :class="item.entry.id === currentEntryId ? 'text-text' : 'text-text-secondary'">
          {{ t(`admin.journal.kindTitle.${item.entry.kind}`) }}
          <span v-if="item.entry.method" class="font-mono text-xs font-normal">{{ item.entry.method }} {{ item.entry.route }}</span>
        </span>
        <small class="block text-text-muted">{{ labels.preciseTime(item.at) }}</small>
        <pre
          v-if="item.entry.message"
          class="mt-1 rounded-md bg-surface-sunken p-2 font-mono text-xs whitespace-pre-wrap text-text-secondary"
        >{{ item.entry.message }}</pre>
      </template>

      <template v-else-if="item.kind === 'change'">
        <span class="block font-semibold">
          {{
            t('admin.journal.trace.change', {
              action: t(`admin.journal.action.${item.change.action}`),
              entity: labels.changeEntityLabel(item.change),
            })
          }}
          <span class="font-mono text-xs font-normal" :title="item.change.entity_id ?? undefined">
            {{ journalShortId(item.change.entity_id) }}
          </span>
        </span>
        <small class="block text-text-muted">
          {{ labels.preciseTime(item.at) }}<template v-if="item.change.fields.length">, {{ fieldsText(item.change.fields) }}</template>
        </small>
      </template>

      <template v-else>
        <span class="block font-semibold">
          {{ t('admin.journal.trace.event', { label: labels.eventLabel(item.event.event_type) }) }}
        </span>
        <small class="block text-text-muted">
          {{ labels.preciseTime(item.at) }},
          {{
            item.event.published_at
              ? t('admin.journal.trace.published', { time: labels.preciseTime(item.event.published_at) })
              : t('admin.journal.trace.pending', item.event.attempts)
          }}
        </small>
        <pre
          v-if="item.event.last_error"
          class="mt-1 rounded-md bg-surface-sunken p-2 font-mono text-xs whitespace-pre-wrap text-text-secondary"
        >{{ item.event.last_error }}</pre>
      </template>
    </li>
  </ol>
</template>
