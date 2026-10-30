<script setup lang="ts">
import type { LiveDeskMeeting } from '~/types/admin-incidents'
import type { TimeZoneName } from '~/types/shared'

/**
 * La réunion Zoom d'une activité, sur le poste de direct : son état, combien de
 * personnes sont dans la salle, et les inscriptions que Zoom a refusées.
 *
 * Le compte des présents vient des notifications d'entrée et de sortie de Zoom.
 * Tant que Zoom n'en a envoyé aucune (abonnement manquant), « 0 dans la salle »
 * mentirait : on le dit à la place.
 */

interface Props {
  meeting: LiveDeskMeeting
  timezone: TimeZoneName
  zoneLabel?: string
  presenceNotifications: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { timeWithZone } = useDateTime()

const notCreated = computed(() => props.meeting.status === 'failed' || props.meeting.sync_status === 'failed')
const hidden = computed(() => props.meeting.failure_count - props.meeting.failures.length)
</script>

<template>
  <div class="mt-2 flex flex-col gap-1 text-sm">
    <p v-if="notCreated" class="flex flex-wrap items-center gap-x-2 gap-y-1 text-danger">
      <UiIcon name="warning" size="0.85rem" aria-hidden="true" />
      <span class="font-medium">{{ t('admin.incident.list.desk.meeting.notCreated') }}</span>
      <span v-if="meeting.last_sync_error" class="text-text-muted">{{ meeting.last_sync_error }}</span>
    </p>

    <p v-else-if="meeting.status === 'started'" class="flex flex-wrap items-center gap-x-3 gap-y-1">
      <span class="flex items-center gap-1.5 font-medium text-live">
        <UiIcon name="video" size="0.85rem" aria-hidden="true" />
        {{ t('admin.incident.list.desk.meeting.started') }}
      </span>
      <template v-if="presenceNotifications">
        <strong class="tabular-nums">{{ t('admin.incident.list.desk.meeting.present', { count: meeting.present_count }) }}</strong>
        <span class="text-text-muted tabular-nums">{{ t('admin.incident.list.desk.meeting.joined', { count: meeting.joined_count }) }}</span>
      </template>
      <span v-else class="text-text-muted">{{ t('admin.incident.list.desk.meeting.noPresence') }}</span>
    </p>

    <p v-else class="flex flex-wrap items-center gap-x-3 gap-y-1 text-text-muted">
      <span class="flex items-center gap-1.5">
        <UiIcon name="video" size="0.85rem" aria-hidden="true" />
        {{ t(`admin.incident.list.desk.meeting.status.${meeting.status}`) }}
      </span>
      <span class="tabular-nums">{{ t('admin.incident.list.desk.meeting.registered', { count: meeting.registered_count }) }}</span>
    </p>

    <details v-if="meeting.failure_count > 0" class="group">
      <summary class="inline-flex min-h-(--target-min) cursor-pointer items-center gap-1.5 rounded-sm font-medium text-warning">
        <UiIcon name="warning" size="0.85rem" aria-hidden="true" />
        {{ t('admin.incident.list.desk.meeting.failures', { count: meeting.failure_count }) }}
        <UiIcon name="chevron-down" size="0.9rem" class="group-open:rotate-180" aria-hidden="true" />
      </summary>
      <ul class="flex flex-col gap-2 ps-6 pb-1">
        <li v-for="failure in meeting.failures" :key="failure.participant_id" class="min-w-0">
          <p class="flex flex-wrap gap-x-2">
            <span class="font-medium">{{ failure.display_name }}</span>
            <span class="break-all text-text-muted">{{ failure.email }}</span>
          </p>
          <p>
            {{ t(`admin.incident.list.desk.meeting.reason.${failure.reason}`) }}
            <span class="text-text-muted">
              ·
              {{
                failure.sync_status === 'failed' && failure.next_retry_at
                  ? t('admin.incident.list.desk.meeting.retryAt', { time: timeWithZone(failure.next_retry_at, timezone, zoneLabel) })
                  : t('admin.incident.list.desk.meeting.abandoned')
              }}
            </span>
          </p>
          <p v-if="failure.detail" class="text-xs break-words text-text-muted">{{ failure.detail }}</p>
        </li>
        <li v-if="hidden > 0" class="text-text-muted">{{ t('admin.incident.list.desk.meeting.more', { count: hidden }) }}</li>
      </ul>
    </details>
  </div>
</template>
