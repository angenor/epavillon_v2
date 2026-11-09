<script setup lang="ts">
import type { RegieSession, RegieSpeakersGesture } from '~/types/admin-regie'
import type { TimeZoneName } from '~/types/shared'
import type { MenuItem } from '~/types/ui'
import type { RegieDisplayState, StartLiveBlock } from '~/utils/regie'

interface Props {
  session: RegieSession
  state: RegieDisplayState
  isNext: boolean
  startBlock: StartLiveBlock | null
  onAir: string | null
  nowMs: number
  timezone: TimeZoneName
  canSchedule: boolean
  canManageMeeting: boolean
  busy: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  presence: []
  absent: []
  cancel: []
  'start-live': []
  'end-live': []
  'create-meeting': []
  restore: []
  resume: []
  speakers: [gesture: RegieSpeakersGesture]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()
const localePath = useLocalePath()

const id = computed(() => props.session.session_id)
const time = (value: string | null): string => dt.time(value, props.timezone)
const isCancelled = computed(() => props.session.status === 'cancelled')
const isOpen = computed(() => props.state === 'upcoming' || props.state === 'ongoing')

/** Ce qui manque ou va bien, d'un coup d'œil : vert confirmé, jaune à surveiller. */
const flags = computed(() => {
  const s = props.session
  const list: { tone: 'ok' | 'warn' | 'plain'; text: string }[] = []
  if (isCancelled.value || props.state === 'postponed') return list
  if (s.is_streamed && props.state !== 'past') {
    if (!s.meeting) list.push({ tone: 'warn', text: t('admin.regie.slot.flags.noMeeting') })
    else if (s.meeting.ready) list.push({ tone: 'ok', text: t('admin.regie.slot.flags.meetingReady') })
    else list.push({ tone: 'warn', text: t('admin.regie.slot.flags.meetingNotReady') })
  }
  if (s.documents.length) list.push({ tone: 'ok', text: t('admin.regie.slot.flags.documents', { count: s.documents.length }, s.documents.length) })
  else if (isOpen.value) list.push({ tone: 'warn', text: t('admin.regie.slot.flags.noDocuments') })
  if (s.presence.state === 'confirmed') list.push({ tone: 'ok', text: t('admin.regie.slot.flags.presenceConfirmed') })
  else if (s.presence.state === 'absent') list.push({ tone: 'warn', text: t('admin.regie.slot.flags.absent') })
  else if (isOpen.value) list.push({ tone: 'warn', text: t('admin.regie.slot.flags.presenceUnknown') })
  if (!s.is_published) list.push({ tone: 'plain', text: t('admin.regie.slot.flags.unpublished') })
  return list
})

const FLAG_TONES = { ok: 'text-success', warn: 'font-semibold text-warning', plain: 'text-text-secondary' } as const

/** Annulée : la raison, puis ce qui a suivi (message, courriel). */
const cancellationText = computed(() => {
  const c = props.session.cancellation
  if (!c) return null
  const parts = [
    c.cancelled_at
      ? t('admin.regie.slot.cancelledAt', { time: time(c.cancelled_at), reason: tr(c.reason_label) || t('admin.regie.slot.noReason') })
      : tr(c.reason_label),
  ]
  const n = c.notice
  if (n.state === 'sent') parts.push(t('admin.regie.notice.sent', { count: n.recipients, time: time(n.sent_at) }, n.recipients))
  else if (n.state === 'pending' && n.due_at) {
    const minutes = Math.max(1, Math.ceil((Date.parse(n.due_at) - props.nowMs) / 60_000))
    parts.push(t('admin.regie.notice.pending', { count: n.recipients, minutes }, n.recipients))
  } else parts.push(t('admin.regie.notice.none'))
  return parts.filter(Boolean).join(' ')
})

const showSpeakers = computed(
  () => props.canSchedule && props.session.is_streamed && (isOpen.value || props.state === 'live'),
)

const canResume = computed(
  () =>
    props.session.status === 'completed' &&
    props.session.last_action_id !== null &&
    props.session.resumable_until !== null &&
    Date.parse(props.session.resumable_until) > props.nowMs,
)

const proposalId = computed(() => props.session.documents[0]?.proposal_id ?? null)

const menu = computed<MenuItem[]>(() => {
  const items: MenuItem[] = []
  if (proposalId.value) items.push({ value: 'proposal', label: t('admin.regie.slot.menu.proposal'), icon: 'document' })
  items.push({ value: 'planner', label: t('admin.regie.slot.menu.planner'), icon: 'calendar' })
  if (props.canSchedule && props.session.status === 'live') {
    items.push({ value: 'end', label: t('admin.regie.now.endLive'), icon: 'broadcast', separatorBefore: true })
  }
  if (props.canSchedule && !isCancelled.value && props.state !== 'past' && props.session.status !== 'live') {
    items.push({ value: 'cancel', label: t('admin.regie.slot.menu.cancel'), icon: 'ban', destructive: true, separatorBefore: true })
  }
  return items
})

function onMenu(value: string): void {
  if (value === 'proposal' && proposalId.value) navigateTo(localePath(`/admin/propositions/${proposalId.value}`))
  else if (value === 'planner') navigateTo(localePath('/admin/programmation'))
  else if (value === 'cancel') emit('cancel')
  else if (value === 'end') emit('end-live')
}

const blockText = computed(() =>
  props.startBlock ? t(`admin.regie.startBlock.${props.startBlock}`, { name: props.onAir ?? '' }) : undefined,
)
</script>

<template>
  <li
    class="grid grid-cols-[4.375rem_minmax(0,1fr)] items-center gap-x-4 gap-y-2 border-b border-border-subtle px-4 py-3.5 last:border-b-0 sm:px-5 md:grid-cols-[6rem_minmax(0,1fr)]"
    :class="[
      props.state === 'live' && 'bg-live-surface',
      props.state === 'past' && 'opacity-60',
      props.isNext && 'shadow-[inset_4px_0_0_var(--color-accent-solid)]',
    ]"
  >
    <div class="font-condensed">
      <b class="block text-[1.8rem] leading-[0.9] font-bold tabular-nums" :class="props.state === 'live' ? 'text-live' : 'text-text'">
        {{ time(props.session.starts_at) }}
      </b>
      <span class="font-sans text-xs text-text-muted">{{ t('admin.regie.slot.until', { end: time(props.session.ends_at) }) }}</span>
    </div>

    <div class="min-w-0">
      <h3
        class="font-condensed text-[1.15rem] leading-[1.15] font-semibold"
        :class="isCancelled && 'text-text-muted line-through decoration-[1.5px]'"
      >
        {{ tr(props.session.title) }}
      </h3>
      <AdminRegieOrganizations class="mt-1" :organizations="props.session.organizations" :is-joint="props.session.is_joint" />
      <p v-if="cancellationText" class="mt-1.5 text-xs text-text-secondary">{{ cancellationText }}</p>
      <p v-else-if="flags.length" class="mt-1.5 flex flex-wrap gap-x-3 gap-y-1 text-xs">
        <span v-for="flag in flags" :key="flag.text" :class="FLAG_TONES[flag.tone]">{{ flag.text }}</span>
      </p>
    </div>

    <div class="col-span-full flex flex-wrap items-center gap-2 md:col-span-1 md:col-start-2">
      <UiStatusBadge size="sm" :state="props.state" :label="t(`admin.regie.state.${props.state}`)" />
      <template v-if="props.canSchedule">
        <template v-if="isOpen && props.session.status !== 'live'">
          <template v-if="props.session.presence.state === 'unknown'">
            <UiButton size="sm" variant="secondary" :loading="props.busy === `presence:${id}`" @click="emit('presence')">
              {{ t('admin.regie.slot.present') }}
            </UiButton>
            <UiButton size="sm" variant="ghost" @click="emit('absent')">{{ t('admin.regie.slot.absent') }}</UiButton>
          </template>
          <UiButton
            v-if="props.session.is_streamed && !props.session.meeting && props.canManageMeeting"
            size="sm"
            variant="secondary"
            :loading="props.busy === `meeting:${id}`"
            @click="emit('create-meeting')"
          >
            {{ t('admin.regie.slot.createMeeting') }}
          </UiButton>
          <UiButton
            v-if="props.isNext"
            size="sm"
            :disabled="props.startBlock !== null"
            :title="blockText"
            :loading="props.busy === `start:${id}`"
            @click="emit('start-live')"
          >
            {{ t('admin.regie.slot.startLive') }}
          </UiButton>
        </template>
        <UiButton
          v-if="isCancelled && props.session.cancellation?.action_id"
          size="sm"
          variant="secondary"
          @click="emit('restore')"
        >
          {{ t('admin.regie.slot.restore') }}
        </UiButton>
        <UiButton v-if="canResume" size="sm" variant="secondary" :loading="props.busy === `resume:${id}`" @click="emit('resume')">
          {{ t('admin.regie.slot.resume') }}
        </UiButton>
      </template>
      <UiContextMenu class="regie-menu" :items="menu" :label="t('admin.regie.slot.menu.label')" align="end" @select="onMenu" />
    </div>
    <p v-if="props.isNext && blockText && props.canSchedule && isOpen" class="col-span-full text-xs text-text-secondary md:col-start-2">
      {{ blockText }}
    </p>
    <AdminRegieSpeakersRow
      v-if="showSpeakers"
      :summary="props.session.speakers"
      :timezone="props.timezone"
      @register="emit('speakers', 'register')"
      @resend="emit('speakers', 'resend')"
      @owner="emit('speakers', 'owner')"
    />
  </li>
</template>

<style scoped>
/* Le menu commun dessine un déclencheur de 32 px : la régie se pilote au doigt, 44 px. */
.regie-menu :deep(button[aria-haspopup='menu']) {
  width: var(--target-min);
  height: var(--target-min);
  border-color: var(--color-border);
}
</style>
