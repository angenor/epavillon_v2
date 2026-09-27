<script setup lang="ts">
import type { ExpertQueueReportGroup, FaqReportOutcome } from '~/types/admin-negotiation-queue'
import type { Uuid } from '~/types/shared'

// Aucun auteur : ni celui qui signale, ni l'expert qui clôt (R9).
const props = defineProps<{
  group: ExpertQueueReportGroup
}>()

const emit = defineEmits<{ close: [reportId: Uuid, outcome: FaqReportOutcome] }>()

const { t } = useI18n()
const localePath = useLocalePath()
const { date, dateTime, zoneLabel } = useDateTime()

const MOTIFS = ['too_vague', 'off_topic', 'outdated'] as const
const ISSUES: FaqReportOutcome[] = ['revised', 'confirmed', 'dismissed']

const zone = (instant: string): string =>
  t('admin.negociations.file.zoned', {
    date: dateTime(instant, KNOWLEDGE_TIMEZONE),
    zone: zoneLabel(KNOWLEDGE_TIMEZONE),
  })

const motifs = (reasons: string[]): string =>
  reasons.map((r) => t(`admin.negociations.file.reason.${r}`)).join(' · ')
</script>

<template>
  <article class="rounded-lg border border-border bg-surface p-4 sm:p-5">
    <header class="flex flex-wrap items-start justify-between gap-3">
      <div class="min-w-0 flex-[1_1_16rem]">
        <h2 class="text-lg leading-snug font-semibold text-balance break-words">{{ props.group.entry.question }}</h2>
        <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
          <UiBadge
            :intent="KNOWLEDGE_STATUS_INTENT[props.group.entry.status]"
            size="sm"
            :label="t(`admin.negociations.faq.state.${props.group.entry.status}`)"
          />
          <span v-if="props.group.entry.verified_on" class="text-text-muted">
            {{ t('admin.negociations.file.item.verified', { date: date(props.group.entry.verified_on, 'UTC') }) }}
          </span>
          <span v-else class="text-text-muted">{{ t('admin.negociations.file.item.notVerified') }}</span>
        </div>
      </div>
      <UiButton
        variant="secondary"
        icon-trailing="arrow-right"
        :to="localePath(`/admin/negociations/faq/${props.group.entry.id}`)"
      >
        {{ t('admin.negociations.file.item.open') }}
      </UiButton>
    </header>

    <dl class="mt-4 flex flex-wrap gap-x-5 gap-y-1 text-sm text-text-secondary">
      <div class="flex gap-1.5">
        <dt>{{ t('admin.negociations.file.item.helpful') }}</dt>
        <dd class="font-semibold text-text tabular-nums">{{ props.group.feedback.helpful }}</dd>
      </div>
      <div class="flex gap-1.5">
        <dt>{{ t('admin.negociations.file.item.notHelpful') }}</dt>
        <dd class="font-semibold text-text tabular-nums">{{ props.group.feedback.not_helpful }}</dd>
      </div>
      <div v-for="motif in MOTIFS" :key="motif" class="flex gap-1.5">
        <dt>{{ t(`admin.negociations.file.item.feedback.${motif}`) }}</dt>
        <dd class="font-semibold text-text tabular-nums">{{ props.group.feedback[motif] }}</dd>
      </div>
    </dl>

    <h3 class="mt-5 text-sm font-semibold">
      {{ t('admin.negociations.file.item.reports', { count: props.group.reports.length }, props.group.reports.length) }}
    </h3>
    <ol class="mt-2 space-y-4">
      <li
        v-for="report in props.group.reports"
        :key="report.id"
        class="rounded-md border border-border bg-surface-raised p-3 sm:p-4"
      >
        <p class="font-medium">
          <template v-if="report.reasons.length">{{ motifs(report.reasons) }}</template>
          <template v-else-if="report.from_feedback">{{ t('admin.negociations.file.fromFeedback') }}</template>
        </p>
        <p v-if="report.details" class="mt-1 text-sm break-words text-text-secondary">
          {{ t('admin.negociations.file.details', { text: report.details }) }}
        </p>
        <p class="mt-1 text-xs text-text-muted">
          {{ t('admin.negociations.file.received', { date: zone(report.created_at) }) }}
        </p>

        <div class="mt-3 flex flex-wrap items-center gap-2">
          <span class="w-full text-sm text-text-secondary sm:w-auto">{{ t('admin.negociations.file.close.label') }}</span>
          <UiButton
            v-for="issue in ISSUES"
            :key="issue"
            variant="secondary"
            @click="emit('close', report.id, issue)"
          >
            {{ t(`admin.negociations.file.outcome.${issue}`) }}
          </UiButton>
        </div>
      </li>
    </ol>
  </article>
</template>
