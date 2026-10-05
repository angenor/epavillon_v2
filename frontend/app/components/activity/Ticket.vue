<script setup lang="ts">
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { RegistrationResult } from '~/types/programme/registration'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'
import type { TicketCallToAction } from './TicketAction.vue'

/**
 * La carte de l'activité : l'heure, le compte à rebours, les places quand elles
 * sont comptées, et LE bouton qui compte à ce moment-là. Les heures d'ouverture
 * des inscriptions se comparent à l'horloge du navigateur, une fois monté : le
 * serveur tranche de toute façon.
 *
 * Dès que ce bouton sort de l'écran, une barre fixée en bas le reprend : on doit
 * pouvoir s'inscrire sans remonter, et sur mobile sans descendre.
 */

interface Props {
  session: PublicScheduleRow
  edition: PublicEditionRow
  state: ProgrammeSessionState
  languages: string[]
  now: number | null
  homeTime: { range: string; zone: string } | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ started: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime, dayLong, time, timeRange, zoneOffsetShort, zoneLabel } = useDateTime()
const requestUrl = useRequestURL()
const { duration } = useProgrammeSession()
const { mine, refresh, cancel } = useSessionRegistration(computed(() => props.session.id))

const dialogOpen = ref(false)
const cancelling = ref(false)
const shared = ref(false)

const capacity = computed(() => props.session.capacity)
const registered = computed(() => props.session.registered_count)
const full = computed(() => capacity.value !== null && registered.value >= capacity.value)
const pct = computed(() => (capacity.value ? Math.min(100, Math.round((100 * registered.value) / capacity.value)) : 0))

const opensAt = computed(() => props.session.registration_opens_at ?? null)
const closesAt = computed(() => props.session.registration_closes_at ?? null)
const notOpenYet = computed(() => props.now !== null && opensAt.value !== null && Date.parse(opensAt.value) > props.now)
const closed = computed(() => props.now !== null && closesAt.value !== null && Date.parse(closesAt.value) <= props.now)

type Action = 'registered' | 'register' | 'waitlist' | 'full' | 'notOpen' | 'closed' | 'free' | 'watch' | 'replay' | 'none'

const action = computed<Action>(() => {
  const state = props.state
  if (state === 'live' || state === 'ongoing') return props.session.is_streamed ? 'watch' : 'none'
  if (state === 'past') return props.session.replay_url ? 'replay' : 'none'
  if (state !== 'upcoming') return 'none'
  if (mine.value) return 'registered'
  if (props.session.registration_required === false) return 'free'
  if (notOpenYet.value) return 'notOpen'
  if (closed.value) return 'closed'
  if (full.value) return props.session.waitlist_enabled ? 'waitlist' : 'full'
  return 'register'
})

const note = computed(() => {
  switch (action.value) {
    case 'registered':
      return mine.value?.status === 'waitlisted' ? t('activity.ticket.note.waitlisted') : t('activity.ticket.note.registered')
    case 'free':
      return t('activity.ticket.note.free')
    case 'notOpen':
      return t('activity.ticket.note.notOpen', { date: dateTime(opensAt.value, props.session.timezone) })
    case 'closed':
      return t('activity.ticket.note.closed')
    case 'full':
      return t('activity.ticket.note.full')
    case 'register':
    case 'waitlist':
      return closesAt.value ? t('activity.ticket.note.until', { date: dateTime(closesAt.value, props.session.timezone) }) : ''
    case 'watch':
      return t('activity.ticket.note.live')
    default:
      return props.state === 'past' ? t('activity.ticket.note.past', { count: registered.value }) : ''
  }
})

const callToAction = computed<TicketCallToAction | null>(() => {
  const value = action.value
  if (value === 'register' || value === 'waitlist' || value === 'watch') return value
  return value === 'replay' && props.session.replay_url ? 'replay' : null
})

const when = computed(
  () =>
    `${dayLong(props.session.starts_at, props.session.timezone)} · ${timeRange(props.session.starts_at, props.session.ends_at, props.session.timezone, props.edition.city ?? undefined)}`,
)

const actions = useTemplateRef<HTMLElement>('actions')
const actionsInView = ref(true)
const footerInView = ref(false)
const barShown = computed(() => callToAction.value !== null && !actionsInView.value && !footerInView.value)
let observer: IntersectionObserver | undefined

onMounted(() => {
  const footer = [...document.querySelectorAll('footer')].at(-1)
  observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (entry.target === actions.value) actionsInView.value = entry.isIntersecting
      else footerInView.value = entry.isIntersecting
    }
  })
  if (actions.value) observer.observe(actions.value)
  if (footer) observer.observe(footer)
})
onBeforeUnmount(() => observer?.disconnect())

const remaining = computed(() => {
  if (capacity.value === null) return ''
  if (full.value) return props.session.waitlist_enabled ? t('session-card.capacity.waitlist') : t('session-card.capacity.full')
  return t('activity.ticket.places', capacity.value - registered.value)
})

const zone = computed(() =>
  t('activity.hero.zone', {
    zone: zoneLabel(props.session.timezone, props.edition.city ?? undefined),
    offset: zoneOffsetShort(props.session.timezone, props.session.starts_at),
  }),
)
const until = computed(() =>
  t('programme.list.until', { end: time(props.session.ends_at, props.session.timezone), duration: duration(props.session) }),
)
const streamedHere = computed(() => props.session.is_streamed && props.state !== 'past' && props.state !== 'cancelled')
const hasAction = computed(() => callToAction.value !== null || action.value === 'registered' || note.value !== '')

const place = computed(() =>
  [props.session.room_name ? tr(props.session.room_name) : '', props.edition.city, tr(props.edition.country_name)]
    .filter(Boolean)
    .join(' · '),
)

async function onDone(_result: RegistrationResult): Promise<void> {
  await refresh()
}

async function onCancel(): Promise<void> {
  cancelling.value = true
  try {
    await cancel()
  } finally {
    cancelling.value = false
  }
}

function downloadCalendar(): void {
  const content = icsCalendar({
    uid: `${props.session.id}@epavillon`,
    title: tr(props.session.title),
    startsAt: props.session.starts_at,
    endsAt: props.session.ends_at,
    location: place.value,
    url: requestUrl.href,
  })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(new Blob([content], { type: 'text/calendar;charset=utf-8' }))
  link.download = `${props.session.slug}.ics`
  link.click()
  URL.revokeObjectURL(link.href)
}

async function share(): Promise<void> {
  const url = requestUrl.href
  if (navigator.share) {
    await navigator.share({ title: tr(props.session.title), url }).catch(() => undefined)
    return
  }
  await navigator.clipboard.writeText(url)
  shared.value = true
  setTimeout(() => (shared.value = false), 2500)
}
</script>

<template>
  <aside class="overflow-hidden rounded-lg bg-surface-raised text-text shadow-lg" :aria-label="t('activity.ticket.label')">
    <div class="h-1 bg-accent" aria-hidden="true" />
    <div class="flex flex-col p-6">
      <p class="text-xs text-text-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">{{ zone }}</p>
      <p class="mt-2.5 flex flex-wrap items-end gap-x-3 gap-y-1">
        <span class="text-[3.25rem] leading-[0.9] font-light tabular-nums" :class="props.state === 'live' ? 'text-live' : 'text-text'">
          {{ time(props.session.starts_at, props.session.timezone) }}
        </span>
        <span class="pb-0.5 text-sm text-text-muted tabular-nums">{{ until }}</span>
      </p>
      <p v-if="props.homeTime" class="mt-3 text-sm text-text-muted">
        {{ t('activity.hero.yourTimeAt') }}
        <b class="font-bold text-text tabular-nums">{{ props.homeTime.range }}</b>
        ({{ props.homeTime.zone }})
      </p>

      <div
        v-if="props.state === 'upcoming'"
        class="mt-5 flex flex-col gap-5 border-t border-border-subtle pt-5"
      >
        <ActivityCountdown :starts-at="props.session.starts_at" @elapsed="emit('started')" />
        <div v-if="capacity !== null">
          <p class="flex flex-wrap items-baseline justify-between gap-x-3 text-sm">
            <span><b class="text-lg font-bold tabular-nums">{{ registered }}</b> {{ t('activity.ticket.ofCapacity', { capacity }) }}</span>
            <span class="text-text-muted">{{ remaining }}</span>
          </p>
          <div
            class="mt-2 h-1 overflow-hidden rounded-full bg-border-subtle"
            role="meter"
            :aria-valuenow="registered"
            :aria-valuemax="capacity"
            aria-valuemin="0"
            :aria-label="t('session-card.capacity.label')"
          >
            <div class="h-full rounded-full bg-accent" :style="{ width: `${pct}%` }" />
          </div>
        </div>
      </div>

      <div v-show="hasAction" ref="actions" class="mt-5 flex flex-col gap-2.5">
        <ActivityTicketAction
          v-if="callToAction"
          :action="callToAction"
          :replay-url="props.session.replay_url"
          @register="dialogOpen = true"
        />
        <template v-else-if="action === 'registered'">
          <p class="flex min-h-12 items-center justify-center gap-2 rounded-md bg-success-surface px-4 font-bold text-success">
            <UiIcon name="check-circle" size="1.25rem" />
            {{ t(mine?.status === 'waitlisted' ? 'activity.ticket.onWaitlist' : 'activity.ticket.registered') }}
          </p>
          <UiButton variant="link" class="self-center" :loading="cancelling" @click="onCancel">
            {{ t('activity.ticket.cancel') }}
          </UiButton>
        </template>
        <p v-if="note" class="text-[0.8125rem] leading-relaxed text-text-muted">{{ note }}</p>
      </div>

      <div v-if="props.languages.length || streamedHere" class="mt-4 flex flex-col gap-1.5 text-[0.8125rem] text-text-muted">
        <p v-if="props.languages.length">
          {{ t('activity.ticket.languages') }} · <span class="text-text">{{ props.languages.join(', ') }}</span>
        </p>
        <p v-if="streamedHere" class="flex items-center gap-2">
          <span class="relative flex size-2 shrink-0" aria-hidden="true">
            <span v-if="props.state === 'live'" class="absolute inset-0 animate-ping rounded-full bg-live motion-reduce:animate-none" />
            <span class="relative size-2 rounded-full bg-live" />
          </span>
          {{ t('activity.ticket.live') }}
        </p>
      </div>

      <div class="mt-4 grid grid-cols-2 gap-2">
        <button
          type="button"
          class="inline-flex min-h-(--target-min) cursor-pointer items-center justify-center gap-2 rounded-md border border-border text-sm font-bold text-text transition-colors hover:bg-surface-sunken"
          @click="downloadCalendar"
        >
          <UiIcon name="calendar" size="1rem" />
          {{ t('activity.ticket.calendar') }}
        </button>
        <button
          type="button"
          class="inline-flex min-h-(--target-min) cursor-pointer items-center justify-center gap-2 rounded-md border border-border text-sm font-bold text-text transition-colors hover:bg-surface-sunken"
          @click="share"
        >
          <UiIcon :name="shared ? 'check' : 'copy'" size="1rem" />
          {{ t(shared ? 'activity.ticket.copied' : 'activity.ticket.share') }}
        </button>
      </div>
    </div>

    <ActivityRegistrationDialog
      v-model:open="dialogOpen"
      :session-id="props.session.id"
      :session-title="tr(props.session.title)"
      :timezone="props.session.timezone"
      :timezone-label="props.edition.city ?? undefined"
      @done="onDone"
    />

    <ClientOnly>
      <Teleport to="body">
        <Transition
          enter-active-class="transition-transform duration-200 ease-out"
          leave-active-class="transition-transform duration-150 ease-in"
          enter-from-class="translate-y-full"
          leave-to-class="translate-y-full"
        >
          <div
            v-if="barShown && callToAction"
            class="fixed inset-x-0 bottom-0 z-40 border-t border-border-subtle bg-surface-raised pb-[env(safe-area-inset-bottom)] text-text shadow-lg"
            role="region"
            :aria-label="t('activity.ticket.bar')"
          >
            <div class="mx-auto flex max-w-[1440px] items-center gap-4 px-4 py-2.5 sm:px-6 lg:px-12">
              <div class="min-w-0 flex-1">
                <p class="truncate font-bold">{{ tr(props.session.title) }}</p>
                <p class="truncate text-xs text-text-muted tabular-nums">{{ when }}</p>
              </div>
              <ActivityTicketAction
                class="shrink-0 px-5!"
                :action="callToAction"
                :replay-url="props.session.replay_url"
                @register="dialogOpen = true"
              />
            </div>
          </div>
        </Transition>
      </Teleport>
    </ClientOnly>
  </aside>
</template>
