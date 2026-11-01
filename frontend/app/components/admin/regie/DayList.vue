<script setup lang="ts">
import type { RegieLiveElsewhere, RegieSession } from '~/types/admin-regie'
import type { IsoDate, TimeZoneName } from '~/types/shared'

interface Props {
  day: IsoDate
  sessions: RegieSession[]
  liveElsewhere: RegieLiveElsewhere | null
  nextId: string | null
  onAir: string | null
  nowMs: number
  timezone: TimeZoneName
  zoneLabel: string | null
  canSchedule: boolean
  canManageMeeting: boolean
  busy: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  presence: [session: RegieSession]
  absent: [session: RegieSession]
  cancel: [session: RegieSession]
  'start-live': [session: RegieSession]
  'end-live': [session: RegieSession]
  'create-meeting': [session: RegieSession]
  restore: [session: RegieSession]
  resume: [session: RegieSession]
}>()

const { t } = useI18n()
const dt = useDateTime()

const heading = computed(() => {
  const label = new Intl.DateTimeFormat(dt.intlLocale.value, { weekday: 'long', day: 'numeric', month: 'long', timeZone: 'UTC' })
    .format(new Date(`${props.day}T12:00:00Z`))
  return label.charAt(0).toLocaleUpperCase(dt.intlLocale.value) + label.slice(1)
})

const cancelled = computed(() => props.sessions.filter((session) => session.status === 'cancelled').length)
const count = computed(() => {
  const total = t('admin.regie.day.count', { count: props.sessions.length }, props.sessions.length)
  return cancelled.value ? t('admin.regie.day.withCancelled', { total, count: cancelled.value }, cancelled.value) : total
})
</script>

<template>
  <section class="rounded-lg border border-border-subtle bg-surface-raised shadow-sm" aria-labelledby="regie-day">
    <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 border-b border-border-subtle px-4 py-3.5 sm:px-5">
      <h2 id="regie-day" class="heading-condensed text-[1.35rem]">{{ heading }}</h2>
      <span class="text-sm text-text-muted">
        {{ t('admin.regie.day.meta', { count, zone: dt.zoneLabel(props.timezone, props.zoneLabel ?? undefined) }) }}
      </span>
    </div>
    <ol class="m-0 list-none p-0">
      <AdminRegieSlot
        v-for="session in props.sessions"
        :key="session.session_id"
        :session="session"
        :state="sessionDisplayState(session, props.nowMs)"
        :is-next="session.session_id === props.nextId"
        :start-block="startLiveBlock(session, props.sessions, props.liveElsewhere)"
        :on-air="props.onAir"
        :now-ms="props.nowMs"
        :timezone="props.timezone"
        :can-schedule="props.canSchedule"
        :can-manage-meeting="props.canManageMeeting"
        :busy="props.busy"
        @presence="emit('presence', session)"
        @absent="emit('absent', session)"
        @cancel="emit('cancel', session)"
        @start-live="emit('start-live', session)"
        @end-live="emit('end-live', session)"
        @create-meeting="emit('create-meeting', session)"
        @restore="emit('restore', session)"
        @resume="emit('resume', session)"
      />
    </ol>
  </section>
</template>
