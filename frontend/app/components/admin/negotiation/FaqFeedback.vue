<script setup lang="ts">
import type { AdminFaqFeedback, AdminFaqReport } from '~/types/admin-negotiation-savoir'

// Aucun auteur : ni le lecteur qui répond, ni celui qui signale, ni l'expert qui clôt (R9).
const props = defineProps<{
  feedback: AdminFaqFeedback
  reports: AdminFaqReport[]
}>()

const { t } = useI18n()
const { dateTime, zoneLabel } = useDateTime()

const MOTIFS = ['too_vague', 'off_topic', 'outdated'] as const

const zone = (instant: string): string =>
  t('admin.negociations.faq.detail.zoned', {
    date: dateTime(instant, KNOWLEDGE_TIMEZONE),
    zone: zoneLabel(KNOWLEDGE_TIMEZONE),
  })

const ouverts = computed(() => props.reports.filter((r) => r.status === 'open').length)
</script>

<template>
  <div class="space-y-6">
    <section class="rounded-lg border border-border bg-surface p-4">
      <h2 class="text-lg font-semibold">{{ t('admin.negociations.faq.detail.feedback.title') }}</h2>
      <dl class="mt-3 grid grid-cols-2 gap-3">
        <div>
          <dt class="text-sm text-text-muted">{{ t('admin.negociations.faq.detail.feedback.helpful') }}</dt>
          <dd class="text-2xl font-semibold tabular-nums">{{ props.feedback.helpful }}</dd>
        </div>
        <div>
          <dt class="text-sm text-text-muted">{{ t('admin.negociations.faq.detail.feedback.notHelpful') }}</dt>
          <dd class="text-2xl font-semibold tabular-nums">{{ props.feedback.not_helpful }}</dd>
        </div>
      </dl>
      <h3 class="mt-4 text-sm font-semibold">{{ t('admin.negociations.faq.detail.feedback.reasons') }}</h3>
      <ul class="mt-2 space-y-1 text-sm">
        <li v-for="motif in MOTIFS" :key="motif" class="flex justify-between gap-3">
          <span>{{ t(`admin.negociations.faq.detail.feedback.reason.${motif}`) }}</span>
          <span class="tabular-nums">{{ props.feedback[motif] }}</span>
        </li>
      </ul>
    </section>

    <section class="rounded-lg border border-border bg-surface p-4">
      <h2 class="text-lg font-semibold">
        {{ t('admin.negociations.faq.detail.reports.title') }}
        <UiBadge
          v-if="ouverts > 0"
          class="ms-2 align-middle"
          intent="warning"
          size="sm"
          :label="t('admin.negociations.faq.list.openReports', { count: ouverts }, ouverts)"
        />
      </h2>
      <p v-if="props.reports.length === 0" class="mt-2 text-sm text-text-muted">
        {{ t('admin.negociations.faq.detail.reports.empty') }}
      </p>
      <ol v-else class="mt-3 space-y-4">
        <li v-for="report in props.reports" :key="report.id" class="border-t border-border pt-3 first:border-t-0 first:pt-0">
          <div class="flex flex-wrap items-center gap-2">
            <UiBadge
              :intent="report.status === 'open' ? 'warning' : 'neutral'"
              size="sm"
              :label="t(`admin.negociations.faq.detail.reports.status.${report.status}`)"
            />
            <UiBadge
              v-if="report.outcome"
              size="sm"
              :label="t(`admin.negociations.faq.detail.reports.outcome.${report.outcome}`)"
            />
          </div>
          <p class="mt-2 text-sm">
            <template v-if="report.reasons.length">
              {{ report.reasons.map((r) => t(`admin.negociations.faq.detail.reports.reason.${r}`)).join(' · ') }}
            </template>
            <template v-else-if="report.from_feedback">
              {{ t('admin.negociations.faq.detail.reports.fromFeedback') }}
            </template>
          </p>
          <p v-if="report.details" class="mt-1 text-sm break-words text-text-secondary">
            {{ t('admin.negociations.faq.detail.reports.details', { text: report.details }) }}
          </p>
          <p class="mt-1 text-xs text-text-muted">
            {{ t('admin.negociations.faq.detail.reports.created', { date: zone(report.created_at) }) }}
          </p>
          <p v-if="report.handled_at" class="text-xs text-text-muted">
            {{ t('admin.negociations.faq.detail.reports.handled', { date: zone(report.handled_at) }) }}
          </p>
        </li>
      </ol>
    </section>
  </div>
</template>
