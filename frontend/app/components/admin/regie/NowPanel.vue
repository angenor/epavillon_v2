<script setup lang="ts">
import type { RegieSession } from '~/types/admin-regie'
import type { TimeZoneName } from '~/types/shared'
import type { StartLiveBlock } from '~/utils/regie'

interface Props {
  session: RegieSession
  nowMs: number
  timezone: TimeZoneName
  zoneLabel: string | null
  canSchedule: boolean
  canManageMeeting: boolean
  canPublishIncident: boolean
  startBlock: StartLiveBlock | null
  /** Ce qui tourne déjà, pour dire pourquoi « Démarrer » est éteint. */
  onAir: string | null
  /** Clé du geste en cours (`start:<id>`…), pour le témoin du bouton. */
  busy: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'open-host': []
  'end-live': []
  'report-overrun': []
  'start-live': []
  'create-meeting': []
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const dt = useDateTime()

const isLive = computed(() => props.session.status === 'live')
const progress = computed(() => slotProgress(props.session, props.nowMs))
const id = computed(() => props.session.session_id)
const startsIn = computed(() => Math.max(0, Math.ceil((Date.parse(props.session.starts_at) - props.nowMs) / 60_000)))

const band = computed(() => {
  if (isLive.value) {
    return t('admin.regie.now.elapsed', {
      elapsed: progress.value.elapsedMinutes,
      remaining: progress.value.remainingMinutes,
    })
  }
  return startsIn.value > 0 ? t('admin.regie.now.startsIn', { minutes: startsIn.value }) : t('admin.regie.now.due')
})

const presenceDetail = computed(() => {
  const count = props.session.meeting?.present_count
  return isLive.value && count !== null && count !== undefined ? t('admin.regie.now.present', { count }, count) : null
})

const until = computed(() => {
  const params = {
    end: dt.time(props.session.ends_at, props.timezone),
    zone: dt.zoneLabel(props.timezone, props.zoneLabel ?? undefined),
    room: tr(props.session.room_name),
  }
  return params.room ? t('admin.regie.now.untilRoom', params) : t('admin.regie.now.until', params)
})

const needsMeeting =computed(() => props.session.is_streamed && !props.session.meeting)
const blockText = computed(() =>
  props.startBlock ? t(`admin.regie.startBlock.${props.startBlock}`, { name: props.onAir ?? '' }) : null,
)
</script>

<template>
  <section
    class="overflow-hidden rounded-lg border bg-surface-raised shadow-sm"
    :class="isLive ? 'border-live-border' : 'border-border-subtle'"
    aria-labelledby="regie-now"
  >
    <div
      class="flex flex-wrap items-center gap-x-4 gap-y-1 px-4 py-2.5 font-condensed text-[1.05rem] font-bold tracking-[0.05em] uppercase sm:px-5"
      :class="isLive ? 'bg-live-surface text-live' : 'bg-accent-surface text-accent'"
    >
      <h2 id="regie-now">{{ isLive ? t('admin.regie.now.live') : t('admin.regie.now.next') }}</h2>
      <span class="ml-auto font-semibold text-text-secondary">{{ band }}</span>
    </div>

    <div class="grid gap-5 p-4 sm:p-5 md:grid-cols-[13.75rem_minmax(0,1fr)]">
      <div class="hidden aspect-video overflow-hidden rounded-md bg-surface-sunken md:block">
        <img
          v-if="props.session.cover"
          :src="props.session.cover.sources.thumb?.url ?? props.session.cover.url"
          :alt="tr(props.session.cover.alt_text)"
          class="size-full object-cover"
        >
      </div>
      <div class="min-w-0">
        <p class="font-condensed" :class="isLive ? 'text-live' : 'text-text'">
          <b class="text-[2.4rem] leading-[0.9] font-bold tabular-nums">{{ dt.time(props.session.starts_at, props.timezone) }}</b>
          <span class="ml-2 text-base font-semibold tracking-[0.04em] text-text-secondary uppercase">
{{ until }}</span>
        </p>
        <h3 class="mt-2 font-condensed text-[1.6rem] leading-[1.1] font-semibold text-balance">{{ tr(props.session.title) }}</h3>
        <AdminRegieOrganizations
          class="mt-2"
          size="md"
          :organizations="props.session.organizations"
          :is-joint="props.session.is_joint"
          :detail="presenceDetail"
        />
        <div
          v-if="isLive"
          class="mt-3 h-1.5 overflow-hidden rounded-full bg-surface-sunken"
          role="progressbar"
          :aria-valuenow="progress.percent"
          aria-valuemin="0"
          aria-valuemax="100"
          :aria-label="t('admin.regie.now.progress')"
        >
          <i class="block h-full bg-live" :style="{ width: `${progress.percent}%` }" />
        </div>
      </div>
    </div>

    <div v-if="props.canSchedule || props.canPublishIncident" class="flex flex-wrap items-center gap-2.5 px-4 pb-4 sm:px-5 sm:pb-5">
      <template v-if="isLive">
        <UiButton
          v-if="props.session.meeting?.ready"
          variant="live"
          icon="video"
          :loading="props.busy === `host:${id}`"
          @click="emit('open-host')"
        >
          {{ t('admin.regie.now.openHost') }}
        </UiButton>
        <UiButton v-if="props.canSchedule" variant="secondary" @click="emit('end-live')">
          {{ t('admin.regie.now.endLive') }}
        </UiButton>
        <UiButton
          v-if="props.canPublishIncident"
          variant="ghost"
          icon="clock"
          :loading="props.busy === `overrun:${id}`"
          @click="emit('report-overrun')"
        >
          {{ t('admin.regie.now.overrun') }}
        </UiButton>
      </template>
      <template v-else-if="props.canSchedule">
        <UiButton
          variant="live"
          icon="broadcast"
          :disabled="props.startBlock !== null"
          :loading="props.busy === `start:${id}`"
          @click="emit('start-live')"
        >
          {{ t('admin.regie.slot.startLive') }}
        </UiButton>
        <UiButton
          v-if="needsMeeting && props.canManageMeeting"
          variant="secondary"
          icon="video"
          :loading="props.busy === `meeting:${id}`"
          @click="emit('create-meeting')"
        >
          {{ t('admin.regie.slot.createMeeting') }}
        </UiButton>
        <p v-if="blockText" class="w-full text-sm text-text-secondary">{{ blockText }}</p>
      </template>
    </div>

    <AdminRegieDocuments :documents="props.session.documents" :timezone="props.timezone" />
  </section>
</template>
