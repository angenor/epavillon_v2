<script setup lang="ts">
import type { RegieJournalEntry } from '~/types/admin-regie'
import type { TimeZoneName } from '~/types/shared'
import type { JournalUndoMode } from '~/utils/regie'

interface Props {
  entries: RegieJournalEntry[]
  isToday: boolean
  nowMs: number
  timezone: TimeZoneName
  canSchedule: boolean
  canPublishIncident: boolean
  busy: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ act: [entry: RegieJournalEntry, mode: JournalUndoMode] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()
const time = (value: string | null): string => dt.time(value, props.timezone)

function label(entry: RegieJournalEntry): string {
  if (entry.kind === 'incident_published') {
    return t('admin.regie.journal.kind.incident_published', { message: tr(entry.incident_message) })
  }
  return t(`admin.regie.journal.kind.${entry.kind}`, { time: time(entry.session_starts_at) })
}

function detail(entry: RegieJournalEntry): string {
  const author = entry.actor_is_me
    ? t('admin.regie.journal.byMe')
    : entry.actor_name
      ? t('admin.regie.journal.by', { name: entry.actor_name })
      : ''
  const n = entry.notice
  if (!n || entry.undone_at) return author
  let notice = ''
  if (n.state === 'sent') notice = t('admin.regie.notice.sent', { count: n.recipients, time: time(n.sent_at) }, n.recipients)
  else if (n.state === 'pending' && n.due_at) {
    const minutes = Math.max(1, Math.ceil((Date.parse(n.due_at) - props.nowMs) / 60_000))
    notice = t('admin.regie.notice.pending', { count: n.recipients, minutes }, n.recipients)
  } else if (n.state === 'withdrawn') notice = t('admin.regie.notice.withdrawn')
  return [author, notice].filter(Boolean).join(t('admin.regie.journal.separator'))
}

const rows = computed(() =>
  props.entries.map((entry) => {
    const mode = journalUndoMode(entry)
    const allowed = mode === 'unpublish' ? props.canPublishIncident : props.canSchedule
    return {
      entry,
      label: label(entry),
      detail: entry.undone_at
        ? t('admin.regie.journal.undone', { time: time(entry.undone_at), name: entry.undone_by ?? '' })
        : detail(entry),
      mode: allowed ? mode : null,
    }
  }),
)
</script>

<template>
  <section class="rounded-lg border border-border-subtle bg-surface-raised shadow-sm" aria-labelledby="regie-journal">
    <div class="flex items-center justify-between gap-3 border-b border-border-subtle px-4 py-3.5 sm:px-5">
      <h2 id="regie-journal" class="heading-condensed text-[1.35rem]">{{ t('admin.regie.journal.title') }}</h2>
      <span v-if="props.isToday" class="text-sm text-text-muted">{{ t('admin.regie.journal.today') }}</span>
    </div>
    <p v-if="!rows.length" class="px-4 py-4 text-sm text-text-muted sm:px-5">{{ t('admin.regie.journal.empty') }}</p>
    <ul v-else class="m-0 list-none p-0">
      <li
        v-for="row in rows"
        :key="row.entry.id"
        class="grid grid-cols-[3.25rem_minmax(0,1fr)_auto] items-center gap-2.5 border-b border-border-subtle px-4 py-2.5 text-sm last:border-b-0 sm:px-5"
      >
        <time :datetime="row.entry.acted_at" class="font-condensed text-[1.05rem] font-bold tabular-nums">{{ time(row.entry.acted_at) }}</time>
        <span class="min-w-0">
          <span :class="row.entry.undone_at && 'text-text-muted line-through'">{{ row.label }}</span>
          <small v-if="row.detail" class="block text-xs text-text-muted">{{ row.detail }}</small>
        </span>
        <UiButton
          v-if="row.mode"
          size="sm"
          variant="ghost"
          :loading="props.busy === `undo:${row.entry.id}`"
          @click="emit('act', row.entry, row.mode)"
        >
          {{ t(`admin.regie.journal.action.${row.mode}`) }}
        </UiButton>
        <span v-else />
      </li>
    </ul>
  </section>
</template>
