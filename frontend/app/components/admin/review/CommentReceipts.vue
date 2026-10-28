<script setup lang="ts">
import type { CommentReceipts, CommentVisibility } from '~/types/programme/proposal'
import type { TimeZoneName } from '~/types/shared'

/**
 * SOUS UN MESSAGE QU'ON A ÉCRIT : qui l'a lu. Le compte d'abord (« Lu par 2 sur
 * 3 »), les noms et les heures dans un dépliant natif, ouvrable au clavier et
 * lu par les lecteurs d'écran sans script. L'organisation se dit à part : c'est
 * la seule lecture dont dépend la suite d'une demande de correction.
 */

interface Props {
  receipts: CommentReceipts | null
  visibility: CommentVisibility
  timezone: TimeZoneName
  zoneLabel: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { dateTime, zoneLabel } = useDateTime()

const summary = computed(() => summarizeReceipts(props.receipts))

function when(at: string): string {
  return `${dateTime(at, props.timezone)}, ${zoneLabel(props.timezone, props.zoneLabel)}`
}
</script>

<template>
  <div v-if="summary" class="mt-2 flex flex-col gap-1 text-sm text-text-muted">
    <template v-if="summary.audience > 0">
      <details v-if="summary.committee.length > 0" class="group">
        <summary class="inline-flex min-h-(--target-min) cursor-pointer items-center gap-1.5 rounded-sm hover:text-text">
          <UiIcon name="eye" size="1rem" aria-hidden="true" />
          {{ t('admin.proposal.review.comments.receipts.readBy', { read: summary.committee.length, total: summary.audience }) }}
          <UiIcon name="chevron-down" size="0.9rem" class="group-open:rotate-180" aria-hidden="true" />
        </summary>
        <ul class="mt-1 flex flex-col gap-1 ps-6">
          <li v-for="reader in summary.committee" :key="reader.person_id">
            {{ t('admin.proposal.review.comments.receipts.reader', { name: reader.display_name, date: when(reader.read_at) }) }}
          </li>
        </ul>
      </details>
      <p v-else class="inline-flex items-center gap-1.5">
        <UiIcon name="eye-off" size="1rem" aria-hidden="true" />
        {{ t('admin.proposal.review.comments.receipts.unreadBy', { total: summary.audience }) }}
      </p>
    </template>

    <p v-if="props.visibility === 'submitter'" class="inline-flex items-center gap-1.5">
      <UiIcon :name="summary.organisation ? 'check' : 'clock'" size="1rem" aria-hidden="true" />
      {{
        summary.organisation
          ? t('admin.proposal.review.comments.receipts.organisationRead', {
            name: summary.organisation.display_name,
            date: when(summary.organisation.read_at),
          })
          : t('admin.proposal.review.comments.receipts.organisationUnread')
      }}
    </p>
  </div>
</template>
