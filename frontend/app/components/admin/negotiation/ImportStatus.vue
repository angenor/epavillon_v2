<script setup lang="ts">
import type { ImportRun, OfficialImportAdmin } from '~/types/negotiation-sessions'
import type { Intent, TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'état de l'import (FR-019) : servi ou coupé, santé de la source, journal.
 * `serving` vient de l'API, qui lit la règle de coupure : l'écran ne la refait pas.
 */

const props = defineProps<{ etat: OfficialImportAdmin }>()

const { t } = useI18n()
const localePath = useLocalePath()
const { date, timeWithZone } = useDateTime()

const fuseau = computed(() => props.etat.edition.timezone as TimeZoneName)
const moment = (valeur: string | null): string =>
  valeur
    ? t('admin.negociations.import.status.moment', {
        date: date(valeur, fuseau.value),
        time: timeWithZone(valeur, fuseau.value),
      })
    : t('admin.negociations.import.status.never')

const etatAffiche = computed<{ cle: 'serving' | 'cut' | 'disabled'; intent: Intent }>(() => {
  if (!props.etat.enabled) return { cle: 'disabled', intent: 'neutral' }
  return props.etat.serving ? { cle: 'serving', intent: 'success' } : { cle: 'cut', intent: 'danger' }
})

const motif = computed(() => {
  if (!props.etat.enabled) return t('admin.negociations.import.cut.disabled')
  if (props.etat.serving) return null
  return props.etat.last_success_at
    ? t('admin.negociations.import.cut.unreachable')
    : t('admin.negociations.import.cut.neverRead')
})

const colonnes = computed<TableColumn[]>(() => [
  { key: 'started_at', label: t('admin.negociations.import.journal.columns.startedAt') },
  { key: 'outcome', label: t('admin.negociations.import.journal.columns.outcome') },
  { key: 'session_count', label: t('admin.negociations.import.journal.columns.sessions'), align: 'end', hideOnMobile: true },
  { key: 'change_count', label: t('admin.negociations.import.journal.columns.changes'), align: 'end' },
  { key: 'manual', label: t('admin.negociations.import.journal.columns.origin'), hideOnMobile: true },
  { key: 'error', label: t('admin.negociations.import.journal.columns.error'), hideOnMobile: true },
])

const lignes = computed(() => props.etat.runs.map((r, i) => ({ ...r, cle: `${r.started_at}-${i}` })))
const issue = (r: ImportRun): Intent => (r.outcome === 'success' ? 'success' : 'danger')
</script>

<template>
  <section class="space-y-6">
    <div class="flex flex-wrap items-center gap-3">
      <h2 class="text-lg font-semibold">{{ t('admin.negociations.import.status.title') }}</h2>
      <UiBadge :intent="etatAffiche.intent" :label="t(`admin.negociations.import.state.${etatAffiche.cle}`)" />
    </div>

    <UiAlert v-if="motif" :intent="etat.enabled ? 'danger' : 'info'" compact :message="motif" />
    <UiAlert
      v-if="etat.failing_since"
      intent="warning"
      compact
      :message="t('admin.negociations.import.status.failingSince', { moment: moment(etat.failing_since) })"
    />

    <dl class="grid gap-x-8 gap-y-4 sm:grid-cols-2">
      <div>
        <dt class="text-sm text-text-muted">{{ t('admin.negociations.import.status.lastSuccess') }}</dt>
        <dd class="font-medium">{{ moment(etat.last_success_at) }}</dd>
      </div>
      <div>
        <dt class="text-sm text-text-muted">{{ t('admin.negociations.import.status.lastAttempt') }}</dt>
        <dd class="font-medium">{{ moment(etat.last_attempt_at) }}</dd>
      </div>
      <div>
        <dt class="text-sm text-text-muted">{{ t('admin.negociations.import.status.missed') }}</dt>
        <dd class="font-medium">
          {{ t('admin.negociations.import.status.missedValue', { count: etat.missed_reads, threshold: etat.missed_threshold }) }}
        </dd>
      </div>
      <div>
        <dt class="text-sm text-text-muted">{{ t('admin.negociations.import.status.changes') }}</dt>
        <dd class="font-medium tabular-nums">{{ etat.last_change_count ?? '—' }}</dd>
      </div>
      <div>
        <dt class="text-sm text-text-muted">{{ t('admin.negociations.import.status.sessions') }}</dt>
        <dd class="font-medium tabular-nums">{{ etat.session_count }}</dd>
      </div>
      <div>
        <dt class="text-sm text-text-muted">{{ t('admin.negociations.import.status.withoutTheme') }}</dt>
        <dd class="flex flex-wrap items-center gap-x-3 font-medium tabular-nums">
          {{ etat.agenda_items_without_theme }}
          <NuxtLink
            v-if="etat.agenda_items_without_theme > 0"
            class="text-sm font-normal text-accent underline underline-offset-2"
            :to="localePath({ path: '/admin/negociations/ordre-du-jour', query: { edition: etat.edition.slug } })"
          >
            {{ t('admin.negociations.import.status.withoutThemeLink') }}
          </NuxtLink>
        </dd>
      </div>
      <div v-if="etat.last_error" class="sm:col-span-2">
        <dt class="text-sm text-text-muted">{{ t('admin.negociations.import.status.error') }}</dt>
        <dd class="font-medium text-danger">{{ etat.last_error }}</dd>
      </div>
    </dl>

    <div>
      <h2 class="text-lg font-semibold">{{ t('admin.negociations.import.journal.title') }}</h2>
      <UiTable
        class="mt-3"
        :columns="colonnes"
        :rows="lignes"
        row-key="cle"
        :caption="t('admin.negociations.import.journal.caption')"
        visually-hidden-caption
        dense
        :hoverable="false"
      >
        <template #cell-started_at="{ row }">{{ moment(row.started_at) }}</template>
        <template #cell-outcome="{ row }">
          <UiBadge size="sm" :intent="issue(row)" :label="t(`admin.negociations.import.journal.outcomes.${row.outcome}`)" />
        </template>
        <template #cell-session_count="{ row }">{{ row.session_count ?? '—' }}</template>
        <template #cell-change_count="{ row }">{{ row.change_count ?? '—' }}</template>
        <template #cell-manual="{ row }">
          {{ t(`admin.negociations.import.journal.origins.${row.manual ? 'manual' : 'chain'}`) }}
        </template>
        <template #cell-error="{ row }">
          <span class="text-sm text-text-muted">{{ row.error ?? '' }}</span>
        </template>
        <template #empty>{{ t('admin.negociations.import.journal.empty') }}</template>
      </UiTable>
    </div>
  </section>
</template>
