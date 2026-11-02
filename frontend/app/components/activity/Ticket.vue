<script setup lang="ts">
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { RegistrationResult } from '~/types/programme/registration'
import type { PublicSessionMeeting } from '~/types/live'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'
import type { TicketCallToAction } from './TicketAction.vue'
import type { PublicTrackPage } from '~/types/programme/track-page'

/**
 * La carte de l'activité : l'heure, le compte à rebours, les places quand elles
 * sont comptées, et LE bouton qui compte à ce moment-là. Les heures d'ouverture
 * des inscriptions se comparent à l'horloge du navigateur, une fois monté : le
 * serveur tranche de toute façon.
 *
 * Dès que ce bouton sort de l'écran, une barre fixée en bas le reprend : on doit
 * pouvoir s'inscrire sans remonter, et sur mobile sans descendre.
 *
 * Activité d'une journée spéciale ou de l'atelier (`day`) : on s'inscrit à la
 * journée, qui se suit par un seul direct. « S'inscrire » ouvre la fenêtre de
 * la journée, cette activité en avant ; ni jauge ni fenêtre d'inscription
 * propres ; « Rejoindre » mène à la salle de la journée.
 */

interface Props {
  session: PublicScheduleRow
  edition: PublicEditionRow
  state: ProgrammeSessionState
  languages: string[]
  now: number | null
  homeTime: { range: string; zone: string } | null
  meeting?: PublicSessionMeeting | null
  /** La salle Zoom tient la scène (même règle que `ActivityStage`) : l'action devient « Rejoindre ». */
  zoomOpen?: boolean
  roomTo?: string
  /** La journée spéciale dont l'activité fait partie : son nom sous la date, et la bande y mène. */
  specialDay?: { title: string; to: string } | null
  /** La journée dont l'activité fait partie, et qui se suit d'un seul direct ; nulle sinon. */
  day?: { page: PublicTrackPage } | null
}

const props = withDefaults(defineProps<Props>(), { meeting: null, zoomOpen: false, roomTo: undefined, specialDay: null, day: null })
const emit = defineEmits<{ started: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime, dayLong, time, timeWithZone, zoneOffsetShort, zoneLabel } = useDateTime()
const { duration } = useProgrammeSession()
const { mine, refresh, cancel } = useSessionRegistration(computed(() => (props.day ? '' : props.session.id)))
const dayRegistration = useDayRegistration(() => props.day?.page.track.id ?? null)
const { mine: dayMine, dialogOpen: dayDialogOpen, highlight: dayHighlight } = dayRegistration
const isRegistered = computed(() => (props.day ? dayRegistration.registered.value : mine.value !== null))
const registrationRequired = computed(() => Boolean(props.day) || props.session.registration_required !== false)
const dayTarget = computed(() =>
  props.day ? { page: props.day.page, registration: dayMine.value, highlight: dayHighlight.value } : null,
)
const joinTo = computed(() => props.roomTo ?? '#direct')

const dialogOpen = ref(false)
const cancelling = ref(false)
const { copied: shared, share, url: requestUrl } = useShareLink(() => tr(props.session.title))

/** Nombre d'inscrits caché au public pour le moment (demande du 30/10) : repasser à `true` pour le rendre. */
const isAttendanceShown = false

const capacity = computed(() => props.session.capacity)
const registered = computed(() => props.session.registered_count)
const full = computed(() => !props.day && capacity.value !== null && registered.value >= capacity.value)
const pct = computed(() => (capacity.value ? Math.min(100, Math.round((100 * registered.value) / capacity.value)) : 0))

const opensAt = computed(() => props.session.registration_opens_at ?? null)
const closesAt = computed(() => props.session.registration_closes_at ?? null)
const notOpenYet = computed(() => !props.day && props.now !== null && opensAt.value !== null && Date.parse(opensAt.value) > props.now)
const closed = computed(() => !props.day && props.now !== null && closesAt.value !== null && Date.parse(closesAt.value) <= props.now)

type Action = 'registered' | 'register' | 'waitlist' | 'full' | 'notOpen' | 'closed' | 'free' | 'watch' | 'join' | 'replay' | 'none'

const action = computed<Action>(() => {
  const state = props.state
  if (props.zoomOpen) {
    if (!registrationRequired.value || isRegistered.value) return 'join'
    if (!notOpenYet.value && !closed.value) return full.value ? (props.session.waitlist_enabled ? 'waitlist' : 'full') : 'register'
  }
  if (state === 'live' || state === 'ongoing') return props.session.is_streamed ? 'watch' : 'none'
  if (state === 'past') return props.session.replay_url ? 'replay' : 'none'
  if (state !== 'upcoming') return 'none'
  if (isRegistered.value) return 'registered'
  if (!registrationRequired.value) return 'free'
  if (notOpenYet.value) return 'notOpen'
  if (closed.value) return 'closed'
  if (full.value) return props.session.waitlist_enabled ? 'waitlist' : 'full'
  return 'register'
})

const note = computed(() => {
  switch (action.value) {
    case 'registered':
      if (props.day) return dayNote.value
      return mine.value?.status === 'waitlisted' ? t('activity.ticket.note.waitlisted') : t('activity.ticket.note.registered')
    case 'join':
      return props.day ? joinLead.value : [joinLead.value, meetingWhen.value].filter(Boolean).join(' ')
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
      if (props.day) return t('activity.ticket.day.registerNote', { day: tr(props.day.page.track.title) })
      return closesAt.value ? t('activity.ticket.note.until', { date: dateTime(closesAt.value, props.session.timezone) }) : ''
    case 'watch':
      return t('activity.ticket.note.live')
    default:
      return props.state === 'past' && isAttendanceShown ? t('activity.ticket.note.past', { count: registered.value }) : ''
  }
})

/** « Vous suivez 6 activités sur 8. Celle-ci est dans votre programme. » */
const dayNote = computed(() => {
  const value = dayMine.value
  const page = props.day?.page
  if (!value || !page) return t('activity.ticket.day.guest')
  const count = keptIds(value.session_ids, page).length
  const following = t('activity.ticket.day.following', { count, total: value.activity_count }, count)
  return `${following} ${t(value.session_ids.includes(props.session.id) ? 'activity.ticket.day.kept' : 'activity.ticket.day.notKept')}`
})

/** Ce qui précède l'horaire : la liste d'attente, l'entrée libre, ou la journée. */
const joinLead = computed(() => {
  if (props.day) return t('activity.ticket.day.joinLead')
  if (props.session.registration_required === false) return t('activity.ticket.note.freeMeeting')
  return mine.value?.status === 'waitlisted' ? t('zoom-room.waitlisted') : ''
})

const meetingWhen = computed(() => {
  if (props.state === 'upcoming') {
    return t('activity.ticket.note.meetingStarts', {
      date: dayLong(props.session.starts_at, props.session.timezone),
      time: timeWithZone(props.session.starts_at, props.session.timezone, props.edition.city ?? undefined),
    })
  }
  return props.state === 'past' ? t('activity.ticket.note.meetingAfter') : t('activity.ticket.note.meeting')
})

const callToAction = computed<TicketCallToAction | null>(() => {
  const value = action.value
  if (value === 'register' || value === 'waitlist' || value === 'watch' || value === 'join') return value
  return value === 'replay' && props.session.replay_url ? 'replay' : null
})

const hours = computed(() =>
  t('activity.ticket.hours', {
    start: time(props.session.starts_at, props.session.timezone),
    end: time(props.session.ends_at, props.session.timezone),
  }),
)
const barWhen = computed(
  () =>
    `${dayLong(props.session.starts_at, props.session.timezone)}, ${zoneLabel(props.session.timezone, props.edition.city ?? undefined)}`,
)
const isLiveNow = computed(() => props.state === 'live' || props.state === 'ongoing')
const liveSince = computed(() => t('activity.ticket.liveSince', { time: time(props.session.starts_at, props.session.timezone) }))

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

function openRegistration(): void {
  if (props.day) dayRegistration.openDialog(props.session.id)
  else dialogOpen.value = true
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
  downloadIcs(
    {
      uid: `${props.session.id}@epavillon`,
      title: tr(props.session.title),
      startsAt: props.session.starts_at,
      endsAt: props.session.ends_at,
      location: place.value,
      url: requestUrl.href,
    },
    props.session.slug,
  )
}
</script>

<template>
  <UiTicket :label="t('activity.ticket.label')">
    <template #band>
      <NuxtLink
        v-if="props.specialDay"
        :to="props.specialDay.to"
        class="block no-underline hover:opacity-95"
        :aria-label="t('activity.ticket.toDay', { day: props.specialDay.title })"
      >
        <EventTicketDate :date="props.session.starts_at" :timezone="props.session.timezone" :caption="props.specialDay.title">
          <template #icon>
            <UiIcon name="arrow-right" size="1.75rem" :stroke-width="1.8" />
          </template>
        </EventTicketDate>
      </NuxtLink>
      <EventTicketDate v-else :date="props.session.starts_at" :timezone="props.session.timezone" />
    </template>

    <!-- Le corps défile quand la page borne la hauteur du billet collé ; il couvre le rembourrage du billet. -->
    <div class="-m-6 min-h-0 overflow-y-auto px-6 pt-4.5 pb-5.5">
      <p class="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
        <span class="font-condensed text-[2.4rem] leading-none font-bold tabular-nums" :class="isLiveNow ? 'text-live' : 'text-heading'">
          {{ hours }}
        </span>
        <span class="font-condensed text-[1.1rem] font-semibold text-text-secondary">{{ duration(props.session) }}</span>
      </p>
      <p class="mt-1.5 text-sm text-text-secondary first-letter:uppercase">{{ zone }}</p>
      <p v-if="props.homeTime" class="mt-1 text-sm text-text-secondary">
        {{ t('activity.hero.yourTimeAt') }}
        <b class="font-semibold text-text tabular-nums">{{ props.homeTime.range }}</b>
        ({{ props.homeTime.zone }})
      </p>

      <div
        v-if="props.state === 'upcoming'"
        class="mt-4.5 flex flex-col gap-5 border-t border-border-subtle pt-3.5"
      >
        <ActivityCountdown :starts-at="props.session.starts_at" @elapsed="emit('started')" />
        <div v-if="isAttendanceShown && capacity !== null">
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

      <div v-show="hasAction" ref="actions" class="mt-4 flex flex-col gap-2">
        <!-- Le direct a commencé : l'encart rouge porte l'heure de début et le bouton. -->
        <div
          v-if="isLiveNow && (callToAction === 'watch' || callToAction === 'join')"
          class="flex flex-col gap-2.5 rounded-md border border-live-border bg-live-surface p-3"
        >
          <UiStatusBadge state="live" :label="liveSince" size="sm" class="self-start" />
          <p v-if="note" class="text-sm leading-relaxed text-text">{{ note }}</p>
          <ActivityTicketAction :action="callToAction" :replay-url="props.session.replay_url" :join-to="joinTo" :day="Boolean(props.day)" />
        </div>
        <template v-else-if="callToAction">
          <ActivityTicketAction
            :action="callToAction"
            :replay-url="props.session.replay_url"
            :join-to="joinTo"
            :day="Boolean(props.day)"
            @register="openRegistration"
          />
          <UiButton v-if="action === 'join' && mine && !props.day && props.state === 'upcoming'" variant="link" class="self-center" :loading="cancelling" @click="onCancel">
            {{ t('activity.ticket.cancel') }}
          </UiButton>
          <p v-if="note" class="text-[0.8125rem] leading-relaxed text-text-muted">{{ note }}</p>
        </template>
        <div v-else-if="action === 'registered'" class="flex flex-col items-start gap-2 rounded-md border border-success-border bg-success-surface p-3">
          <span class="inline-flex items-center gap-1.5 rounded-full bg-success-solid px-2.5 py-1 text-xs font-bold tracking-[0.05em] text-success-contrast uppercase">
            <UiIcon name="check" size="0.875rem" :stroke-width="2.5" />
            {{ t(props.day ? 'activity.ticket.day.registered' : mine?.status === 'waitlisted' ? 'activity.ticket.onWaitlist' : 'activity.ticket.registered') }}
          </span>
          <p v-if="note" class="text-sm leading-relaxed text-text">{{ note }}</p>
          <UiButton v-if="props.day && dayMine" variant="link" @click="dayRegistration.openDialog(props.session.id)">
            {{ t('activity.ticket.day.edit') }}
          </UiButton>
          <UiButton v-else-if="!props.day" variant="link" :loading="cancelling" @click="onCancel">
            {{ t('activity.ticket.cancel') }}
          </UiButton>
        </div>
        <p v-else-if="action === 'free'" class="rounded-md bg-info-surface px-3 py-2.5 text-sm font-semibold text-info">{{ note }}</p>
        <p v-else-if="note" class="text-[0.8125rem] leading-relaxed text-text-muted">{{ note }}</p>
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

      <div class="mt-4 grid grid-cols-2 gap-2.5">
        <UiButton variant="secondary" icon="calendar" block @click="downloadCalendar">
          {{ t('activity.ticket.calendar') }}
        </UiButton>
        <UiButton variant="secondary" :icon="shared ? 'check' : 'copy'" block @click="share">
          {{ t(shared ? 'activity.ticket.copied' : 'activity.ticket.share') }}
        </UiButton>
      </div>
    </div>

    <template #outside>
      <ActivityRegistrationDialog
        v-if="dayTarget"
        v-model:open="dayDialogOpen"
        :day="dayTarget"
        :timezone="props.session.timezone"
        :timezone-label="props.edition.city ?? undefined"
        @day-registered="dayRegistration.registeredWith"
        @day-updated="dayRegistration.updated"
      />
      <ActivityRegistrationDialog
        v-else
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
            <!-- Téléportée hors de la page : elle reprend elle-même la direction condensée. -->
            <div
              v-if="barShown && callToAction"
              data-direction="condensee"
              class="fixed inset-x-0 bottom-0 z-40 border-t border-border bg-surface-raised pb-[env(safe-area-inset-bottom)] text-text shadow-lg"
              role="region"
              :aria-label="t('activity.ticket.bar')"
            >
              <div class="mx-auto flex max-w-360 items-center justify-between gap-3 px-4 py-2.5 sm:px-6 lg:px-12">
                <div class="min-w-0">
                  <p class="font-condensed text-[1.3rem] leading-tight font-bold whitespace-nowrap text-heading tabular-nums">{{ hours }}</p>
                  <p class="truncate text-xs text-text-muted">{{ barWhen }}</p>
                </div>
                <ActivityTicketAction
                  compact
                  class="shrink-0 px-5!"
                  :action="callToAction"
                  :replay-url="props.session.replay_url"
                  :join-to="joinTo"
            :day="Boolean(props.day)"
                  @register="openRegistration"
                />
              </div>
            </div>
          </Transition>
        </Teleport>
      </ClientOnly>
    </template>
  </UiTicket>
</template>
