<script setup lang="ts">
import type { JournalEntry, JournalRequestTrace } from '~/types/admin-journal'

/**
 * Le détail d'une ligne, ou d'un code de demande retrouvé : qui, où, quelle
 * réponse, puis tout ce qui porte le même code, dans l'ordre.
 *
 * Retrouvé par son code, la ligne affichée en tête est la plus grave de la
 * demande : c'est elle que la personne qui a signalé l'erreur a vue.
 */

const props = defineProps<{
  entry: JournalEntry | null
  requestId: string | null
}>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const api = useApi()
const toast = useToast()
const localePath = useLocalePath()
const labels = useJournalLabels()

const {
  data: trace,
  status,
  error,
  refresh,
} = useAsyncData<JournalRequestTrace | null>(
  'admin-journal-trace',
  async () => (props.requestId ? api.adminJournal.trace(props.requestId) : null),
  { watch: [() => props.requestId], lazy: true, default: () => null },
)

const SEVERITY = { http_error: 0, job_failed: 1, http_refused: 2, http_slow: 3 } as const

const head = computed<JournalEntry | null>(() => {
  if (props.entry) return props.entry
  const entries = trace.value?.entries ?? []
  return [...entries].sort((a, b) => SEVERITY[a.kind] - SEVERITY[b.kind])[0] ?? null
})

const isTraceEmpty = computed(
  () => !!trace.value && trace.value.entries.length + trace.value.changes.length + trace.value.events.length === 0,
)

const errorMessage = computed(() => (error.value ? apiErrorMessage(error.value, (key) => t(key)) : ''))

async function copyRequestId(): Promise<void> {
  if (!props.requestId) return
  try {
    await navigator.clipboard.writeText(props.requestId)
    toast.show({ intent: 'success', title: t('admin.journal.detail.copied') })
  } catch {
    toast.show({ intent: 'warning', title: t('admin.journal.detail.copyFailed') })
  }
}
</script>

<template>
  <aside
    class="rounded-lg border border-border bg-surface-raised p-4 lg:sticky lg:top-4"
    :aria-label="t('admin.journal.detail.label')"
  >
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <AdminJournalKindBadge v-if="head" :kind="head.kind" />
        <h2 class="mt-2 text-lg font-semibold">
          {{ head ? t(`admin.journal.kindTitle.${head.kind}`) : t('admin.journal.detail.lookupTitle') }}
        </h2>
      </div>
      <UiButton variant="ghost" icon="close" icon-only :label="t('admin.journal.detail.close')" @click="emit('close')" />
    </div>

    <p v-if="head?.message" class="mt-2 text-sm">{{ head.message }}</p>

    <dl class="mt-3 grid grid-cols-[7rem_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-sm">
      <template v-if="head">
        <dt class="text-text-muted">{{ t('admin.journal.detail.when') }}</dt>
        <dd>{{ labels.fullTime(head.occurred_at) }}</dd>

        <dt class="text-text-muted">{{ t('admin.journal.detail.person') }}</dt>
        <dd>
          <template v-if="head.actor">
            <NuxtLink :to="localePath(`/admin/utilisateurs/${head.actor.id}`)">{{ head.actor.display_name }}</NuxtLink>
            <span v-if="head.actor.organization_name">, {{ head.actor.organization_name }}</span>
          </template>
          <span v-else class="text-text-muted">
            {{ head.source === 'worker' ? t('admin.journal.entries.worker') : t('admin.journal.entries.visitor') }}
          </span>
        </dd>

        <template v-if="head.route">
          <dt class="text-text-muted">{{ t('admin.journal.detail.route') }}</dt>
          <dd class="font-mono text-xs [overflow-wrap:anywhere]">{{ head.method }} {{ head.route }}</dd>
        </template>

        <template v-if="head.status !== null">
          <dt class="text-text-muted">{{ t('admin.journal.detail.response') }}</dt>
          <dd>
            {{
              head.duration_ms !== null
                ? t('admin.journal.detail.responseValue', { status: head.status, duration: head.duration_ms })
                : head.status
            }}
          </dd>
        </template>

        <template v-if="head.error_code">
          <dt class="text-text-muted">{{ t('admin.journal.detail.errorCode') }}</dt>
          <dd class="font-mono text-xs">{{ head.error_code }}</dd>
        </template>

        <template v-if="head.task">
          <dt class="text-text-muted">{{ t('admin.journal.detail.task') }}</dt>
          <dd>
            {{ labels.taskLabel(head.task) }}
            <NuxtLink v-if="head.job_id" class="block text-xs" :to="{ query: { onglet: 'taches' } }">
              {{ t('admin.journal.detail.seeFailures') }}
            </NuxtLink>
          </dd>
        </template>
      </template>

      <template v-if="requestId">
        <dt class="text-text-muted">{{ t('admin.journal.detail.requestId') }}</dt>
        <dd class="flex items-start gap-1">
          <span class="min-w-0 font-mono text-xs [overflow-wrap:anywhere]">{{ requestId }}</span>
          <UiButton
            variant="ghost"
            icon="copy"
            icon-only
            :label="t('admin.journal.detail.copy')"
            @click="copyRequestId"
          />
        </dd>
      </template>
    </dl>

    <h3 class="mt-4 text-sm font-semibold">{{ t('admin.journal.detail.sameRequest') }}</h3>

    <p v-if="!requestId" class="mt-2 text-sm text-text-muted">{{ t('admin.journal.detail.noRequest') }}</p>

    <div v-else-if="status === 'pending' && !trace" class="mt-3 flex flex-col gap-3" aria-busy="true">
      <UiSkeletonLoader v-for="n in 3" :key="n" :lines="2" />
    </div>

    <UiErrorState
      v-else-if="error"
      class="mt-3"
      compact
      :description="errorMessage"
      :retry-label="t('common.actions.retry')"
      @retry="refresh()"
    />

    <p v-else-if="isTraceEmpty" class="mt-2 text-sm text-text-muted">
      {{ entry ? t('admin.journal.detail.traceEmpty') : t('admin.journal.detail.lookupEmpty') }}
    </p>

    <AdminJournalTrace v-else-if="trace" :trace="trace" :current-entry-id="head?.id ?? null" />
  </aside>
</template>
