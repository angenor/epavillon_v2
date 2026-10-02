<script setup lang="ts">
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { RegistrationResult } from '~/types/programme/registration'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'
import type { TicketCallToAction } from './TicketAction.vue'

/**
 * Le billet : l'image, les places, LE bouton qui compte à ce moment-là, et les
 * informations pratiques. Les heures d'ouverture des inscriptions se comparent
 * à l'horloge du navigateur, une fois monté : le serveur tranche de toute façon.
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
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime, dayLong, timeRange } = useDateTime()
const requestUrl = useRequestURL()
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
  if (capacity.value === null) return t('activity.ticket.unlimited')
  if (full.value) return props.session.waitlist_enabled ? t('session-card.capacity.waitlist') : t('session-card.capacity.full')
  return t('session-card.capacity.remaining', capacity.value - registered.value)
})

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
  <aside
    class="overflow-hidden rounded-lg bg-poster-ink text-poster-on-ink"
    :style="{ boxShadow: `8px 8px 0 ${props.session.themes[0]?.color ?? 'var(--color-poster-line)'}` }"
    :aria-label="t('activity.ticket.label')"
  >
    <div v-if="props.session.cover" class="relative border-b-2 border-poster-ink">
      <UiImage :image="props.session.cover" ratio="16 / 9" loading="eager" sizes="(min-width: 1024px) 26.5rem, 100vw" />
      <span
        v-if="props.state === 'live'"
        class="absolute top-3 left-3 -rotate-[6deg] rounded border-[3px] border-live bg-poster-paper px-2.5 py-0.5 font-poster text-xl font-black tracking-[0.06em] text-live font-stretch-[70%]"
      >
        {{ t('activity.state.live') }}
      </span>
    </div>

    <div class="p-6">
      <p class="font-poster-mono text-xs font-semibold tracking-[0.08em] text-poster-on-ink-muted uppercase">
        {{ t('activity.ticket.places') }}
      </p>
      <template v-if="capacity !== null">
        <p class="mt-1.5 flex items-baseline justify-between gap-3">
          <span class="flex items-baseline gap-2">
            <span class="font-poster text-[2.75rem] leading-[0.9] font-black text-poster-on-ink-accent font-stretch-[62%]">{{ registered }}</span>
            <span class="font-poster-mono text-sm text-poster-on-ink-muted">{{ t('activity.ticket.ofCapacity', { capacity }) }}</span>
          </span>
          <span class="text-right text-sm text-poster-on-ink-muted">{{ remaining }}</span>
        </p>
        <div
          class="mt-2.5 h-2 overflow-hidden rounded-full bg-poster-on-ink-muted/30"
          role="meter"
          :aria-valuenow="registered"
          :aria-valuemax="capacity"
          aria-valuemin="0"
          :aria-label="t('session-card.capacity.label')"
        >
          <div class="h-full bg-poster-on-ink-accent" :style="{ width: `${pct}%` }" />
        </div>
      </template>
      <p v-else class="mt-1.5 font-poster text-[1.75rem] leading-none font-black text-poster-on-ink-accent font-stretch-[68%]">
        {{ remaining }}
      </p>

      <div ref="actions" class="mt-5 flex flex-col gap-2.5">
        <ActivityTicketAction
          v-if="callToAction"
          :action="callToAction"
          :replay-url="props.session.replay_url"
          @register="dialogOpen = true"
        />
        <template v-else-if="action === 'registered'">
          <p class="flex h-13.5 items-center justify-center gap-2 rounded-md bg-poster-on-ink/10 font-bold">
            <UiIcon name="check-circle" size="1.25rem" />
            {{ t(mine?.status === 'waitlisted' ? 'activity.ticket.onWaitlist' : 'activity.ticket.registered') }}
          </p>
          <button
            type="button"
            class="h-11 cursor-pointer text-sm font-semibold underline underline-offset-4 disabled:opacity-60"
            :disabled="cancelling"
            @click="onCancel"
          >
            {{ t('activity.ticket.cancel') }}
          </button>
        </template>
        <p v-if="note" class="text-xs leading-relaxed text-poster-on-ink-muted">{{ note }}</p>
      </div>
    </div>
    <div class="relative flex h-6 items-center" aria-hidden="true">
      <span class="absolute -left-3 size-6 rounded-full bg-poster-paper" />
      <span class="mx-5.5 flex-1 border-t-2 border-dashed border-poster-on-ink-muted/50" />
      <span class="absolute -right-3 size-6 rounded-full bg-poster-paper" />
    </div>

    <div class="px-6 pt-2 pb-6">
      <dl class="grid grid-cols-[5.75rem_minmax(0,1fr)] gap-3 text-sm leading-snug">
        <template v-if="place">
          <dt class="pt-0.5 font-poster-mono text-[0.6875rem] tracking-[0.08em] text-poster-on-ink-muted uppercase">{{ t('activity.ticket.place') }}</dt>
          <dd>{{ place }}</dd>
        </template>
        <dt class="pt-0.5 font-poster-mono text-[0.6875rem] tracking-[0.08em] text-poster-on-ink-muted uppercase">{{ t('activity.ticket.format') }}</dt>
        <dd>{{ t(`session-card.format.${props.session.format}`) }}</dd>
        <template v-if="props.languages.length">
          <dt class="pt-0.5 font-poster-mono text-[0.6875rem] tracking-[0.08em] text-poster-on-ink-muted uppercase">{{ t('activity.ticket.languages') }}</dt>
          <dd>{{ props.languages.join(', ') }}</dd>
        </template>
        <dt class="pt-0.5 font-poster-mono text-[0.6875rem] tracking-[0.08em] text-poster-on-ink-muted uppercase">{{ t('activity.ticket.broadcast') }}</dt>
        <dd>{{ t(props.session.is_streamed ? 'activity.ticket.streamed' : 'activity.ticket.onSite') }}</dd>
      </dl>

      <div class="mt-5 flex gap-2.5">
        <button
          type="button"
          class="inline-flex h-11.5 flex-1 cursor-pointer items-center justify-center gap-2 rounded-md border-2 border-poster-on-ink-muted/60 text-sm font-semibold"
          @click="downloadCalendar"
        >
          <UiIcon name="calendar" size="1.0625rem" />
          {{ t('activity.ticket.calendar') }}
        </button>
        <button
          type="button"
          class="inline-flex h-11.5 flex-1 cursor-pointer items-center justify-center gap-2 rounded-md border-2 border-poster-on-ink-muted/60 text-sm font-semibold"
          @click="share"
        >
          <UiIcon :name="shared ? 'check' : 'copy'" size="1.0625rem" />
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
            class="fixed inset-x-0 bottom-0 z-40 border-t-2 border-poster-ink bg-poster-ink pb-[env(safe-area-inset-bottom)] text-poster-on-ink"
            role="region"
            :aria-label="t('activity.ticket.bar')"
          >
            <div class="mx-auto flex max-w-[1440px] items-center gap-4 px-4 py-2.5 sm:px-6 lg:px-12">
              <div class="min-w-0 flex-1">
                <p class="truncate font-bold">{{ tr(props.session.title) }}</p>
                <p class="truncate font-poster-mono text-xs text-poster-on-ink-muted">{{ when }}</p>
              </div>
              <ActivityTicketAction
                class="h-12! shrink-0 px-5!"
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
