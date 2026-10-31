<script setup lang="ts">
import type { ProposalSessionSlot } from '~/types/admin-review'
import type { Proposal } from '~/types/programme/proposal'
import type { SessionStatus } from '~/types/programme/session'
import type { Intent } from '~/types/ui'
import type { TimeZoneName, Uuid } from '~/types/shared'

/**
 * LA BANDE DE PROGRAMMATION : ce que l'organisation a demandé, l'écart, ce que
 * l'équipe a retenu. Le créneau souhaité vit sur le dossier, le retenu sur la
 * séance : les confondre est l'erreur que cette bande empêche.
 *
 * Une carte retenue par séance : un cycle de webinaires en compte plusieurs.
 */

interface Props {
  proposal: Proposal
  sessions: ProposalSessionSlot[]
  timezone: TimeZoneName
  zoneLabel: string
  canSchedule: boolean
  plannerTo: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ edit: [sessionId: Uuid]; place: [sessionId: Uuid, fromPreferred: boolean] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeRange } = useDateTime()

const STATUS_INTENT: Record<SessionStatus, Intent> = {
  planned: 'neutral',
  scheduled: 'info',
  live: 'danger',
  completed: 'neutral',
  postponed: 'warning',
  cancelled: 'danger',
}

const preferredStart = computed(() => props.proposal.preferred_start_at)
const preferredEnd = computed(() => {
  if (props.proposal.preferred_end_at) return props.proposal.preferred_end_at
  const start = preferredStart.value
  return start && props.proposal.duration_minutes ? endOfSlot(start, props.proposal.duration_minutes) : null
})

const placed = computed(() => props.sessions.filter((slot) => slot.room_id !== null))
const first = computed(() => placed.value[0] ?? null)

const DAY_MS = 86_400_000

/** L'écart se lit en jours de calendrier DU PAVILLON, pas en heures écoulées. */
const shift = computed(() => {
  if (props.sessions.length === 0) return null
  const slot = first.value
  if (!slot) return t('admin.proposal.dashboard.slot.shift.toPlace')
  const wished = preferredStart.value
  if (!wished) return null
  const days = Math.round(
    (Date.parse(dayKeyInZone(slot.starts_at, props.timezone)) - Date.parse(dayKeyInZone(wished, props.timezone))) / DAY_MS,
  )
  if (days > 0) return t('admin.proposal.dashboard.slot.shift.later', days)
  if (days < 0) return t('admin.proposal.dashboard.slot.shift.earlier', -days)
  return Date.parse(slot.starts_at) === Date.parse(wished)
    ? t('admin.proposal.dashboard.slot.shift.same')
    : t('admin.proposal.dashboard.slot.shift.sameDay')
})

function meetingLabel(slot: ProposalSessionSlot): string | null {
  if (!slot.meeting) return slot.is_streamed ? t('admin.proposal.dashboard.slot.meeting.none') : null
  const sync = slot.meeting.sync_status
  if (sync === 'synced') return t('admin.proposal.dashboard.slot.meeting.synced')
  if (sync === 'not_required') return t('admin.proposal.dashboard.slot.meeting.notRequired')
  if (sync === 'failed' || sync === 'abandoned') return t('admin.proposal.dashboard.slot.meeting.failed')
  return t('admin.proposal.dashboard.slot.meeting.pending')
}

function statusLabel(slot: ProposalSessionSlot): string {
  const status = t(`admin.proposal.dashboard.slot.sessionStatus.${slot.status}`)
  return slot.published_at
    ? t('admin.proposal.dashboard.slot.withPublication', { status, publication: t('admin.proposal.dashboard.slot.published') })
    : t('admin.proposal.dashboard.slot.withPublication', { status, publication: t('admin.proposal.dashboard.slot.unpublished') })
}

const CARD = 'flex flex-col gap-2 rounded-lg p-5'
</script>

<template>
  <section
    :aria-label="t('admin.proposal.dashboard.slot.label')"
    class="grid grid-cols-1 gap-3 min-[56rem]:grid-cols-[1fr_auto_1.35fr] min-[56rem]:items-stretch"
  >
    <!-- SOUHAITÉ -->
    <div :class="[CARD, 'border border-border-subtle bg-surface-sunken']">
      <span class="text-xs font-semibold tracking-caps text-text-subtle uppercase">
        {{ t('admin.proposal.dashboard.slot.preferred') }}
      </span>
      <template v-if="preferredStart">
        <span class="text-xl font-bold text-text">{{ date(preferredStart, props.timezone) }}</span>
        <span class="text-lg tabular-nums">
          {{ preferredEnd ? timeRange(preferredStart, preferredEnd, props.timezone, props.zoneLabel) : timeRange(preferredStart, null, props.timezone, props.zoneLabel) }}
        </span>
      </template>
      <template v-else>
        <span class="text-xl font-bold text-text">{{ t('admin.proposal.dashboard.slot.preferredNone') }}</span>
        <span class="text-sm text-text-muted">{{ t('admin.proposal.dashboard.slot.preferredNoneHint') }}</span>
      </template>
      <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm text-text-secondary">
        <span>{{ t('admin.proposal.dashboard.slot.requested', props.proposal.requested_sessions) }}</span>
        <span v-if="props.proposal.duration_minutes">
          {{ t('admin.proposal.dashboard.meta.duration', { minutes: props.proposal.duration_minutes }) }}
        </span>
      </div>
      <p
        v-if="props.proposal.scheduling_constraints"
        class="mt-1 rounded-md border border-warning-border bg-warning-surface px-4 py-3 text-sm text-text"
      >
        {{ t('admin.proposal.dashboard.slot.constraints', { text: props.proposal.scheduling_constraints }) }}
      </p>
    </div>

    <!-- ÉCART -->
    <div v-if="shift" class="flex items-center justify-center">
      <span
        class="inline-flex items-center gap-2 rounded-md bg-info-surface px-3 py-2 text-sm font-bold text-info min-[56rem]:flex-col"
      >
        <UiIcon name="arrow-right" size="1rem" class="max-[56rem]:rotate-90" />
        {{ shift }}
      </span>
    </div>
    <div v-else class="hidden min-[56rem]:block" />

    <!-- RETENU -->
    <div class="flex flex-col gap-3">
      <div
        v-if="props.sessions.length === 0"
        :class="[CARD, 'border border-dashed border-border-strong bg-surface-raised']"
      >
        <span class="text-xs font-semibold tracking-caps text-text-subtle uppercase">
          {{ t('admin.proposal.dashboard.slot.retained') }}
        </span>
        <span class="text-xl font-bold text-text">{{ t('admin.proposal.dashboard.slot.noSession') }}</span>
        <span class="text-sm text-text-muted">{{ t('admin.proposal.dashboard.slot.noSessionHint') }}</span>
      </div>

      <template v-for="slot in props.sessions" :key="slot.id">
        <div
          v-if="slot.room_id !== null"
          :class="[CARD, 'border-2 border-success-border bg-surface-raised']"
        >
          <span class="text-xs font-semibold tracking-caps text-text-subtle uppercase">
            {{ t('admin.proposal.dashboard.slot.retained') }}
            <template v-if="props.sessions.length > 1">
              · {{ t('admin.proposal.dashboard.slot.occurrence', { n: slot.sequence_number }) }}
            </template>
          </span>
          <span class="text-xl font-bold text-text">{{ date(slot.starts_at, slot.timezone) }}</span>
          <span class="text-lg tabular-nums">{{ timeRange(slot.starts_at, slot.ends_at, slot.timezone, props.zoneLabel) }}</span>
          <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-text-secondary">
            <span v-if="slot.room_name" class="inline-flex items-center gap-1">
              <UiIcon name="map-pin" size="1rem" />
              {{ tr(slot.room_name) }}
            </span>
            <UiThemeTag
              v-for="track in slot.tracks"
              :key="track.id"
              size="sm"
              :theme="{ code: track.id, label: track.title, color: track.color_hex }"
            />
            <UiBadge size="sm" :intent="STATUS_INTENT[slot.status]" :label="statusLabel(slot)" />
          </div>
          <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm text-text-secondary">
            <span v-if="meetingLabel(slot)">{{ meetingLabel(slot) }}</span>
            <span>{{ t('admin.proposal.dashboard.slot.registrations', slot.registration_count) }}</span>
            <span v-if="slot.waitlist_count">{{ t('admin.proposal.dashboard.slot.waitlist', { count: slot.waitlist_count }) }}</span>
          </div>
          <div class="mt-auto flex flex-wrap items-center gap-2 pt-3">
            <UiButton v-if="props.canSchedule" variant="secondary" @click="emit('edit', slot.id)">
              {{ t('admin.proposal.dashboard.slot.edit') }}
            </UiButton>
            <UiButton variant="ghost" icon-trailing="arrow-right" :to="props.plannerTo">
              {{ t('admin.proposal.dashboard.slot.openPlanner') }}
            </UiButton>
          </div>
        </div>

        <div
          v-else
          :class="[CARD, 'border-2 border-dashed border-warning-border bg-warning-surface']"
        >
          <span class="text-xs font-semibold tracking-caps text-text-subtle uppercase">
            {{ t('admin.proposal.dashboard.slot.retained') }}
            <template v-if="props.sessions.length > 1">
              · {{ t('admin.proposal.dashboard.slot.occurrence', { n: slot.sequence_number }) }}
            </template>
          </span>
          <span class="text-xl font-bold text-text">{{ t('admin.proposal.dashboard.slot.unplaced') }}</span>
          <span class="text-sm text-text-secondary">{{ t('admin.proposal.dashboard.slot.unplacedHint') }}</span>
          <div v-if="props.canSchedule" class="mt-auto flex flex-wrap gap-2 pt-3">
            <UiButton @click="emit('place', slot.id, false)">
              {{ t('admin.proposal.dashboard.slot.place') }}
            </UiButton>
            <UiButton v-if="preferredStart" variant="secondary" @click="emit('place', slot.id, true)">
              {{ t('admin.proposal.dashboard.slot.takePreferred') }}
            </UiButton>
          </div>
        </div>
      </template>
    </div>
  </section>
</template>
