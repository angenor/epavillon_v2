<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'

/**
 * Tout le programme en liste, un séparateur par jour. Au défilement, le jour
 * dont la section passe sous la bande figée est signalé (`reading`) : la page
 * l'allume dans la bande. `scrollToDay` sert le geste inverse.
 */

interface Props {
  days: IsoDate[]
  sessions: PublicScheduleRow[]
  matches: (session: PublicScheduleRow) => boolean
  timezone: TimeZoneName
  visitedId: string | null
  editionSlug: string
  /** Hauteur de ce qui reste figé au-dessus de la liste (barre du site + bande des jours), en pixels. */
  stickyOffset: number
}

const props = defineProps<Props>()
const emit = defineEmits<{ reading: [date: IsoDate]; reset: [] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { state, fill, link } = useProgrammeSession()

const MAX_THEMES = 3

const at = (day: IsoDate) => new Date(`${day}T12:00:00Z`)
const weekday = (day: IsoDate) =>
  new Intl.DateTimeFormat(locale.value, { weekday: 'long', timeZone: 'UTC' }).format(at(day))
const fullDate = (day: IsoDate) =>
  new Intl.DateTimeFormat(locale.value, { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' }).format(at(day))

function duration(session: PublicScheduleRow): string {
  const minutes = Math.round((Date.parse(session.ends_at) - Date.parse(session.starts_at)) / 60_000)
  const hours = Math.floor(minutes / 60)
  const rest = minutes % 60
  if (!hours) return t('programme.list.minutes', { minutes })
  return rest ? t('programme.list.hoursMinutes', { hours, minutes: String(rest).padStart(2, '0') }) : t('programme.list.hours', { hours })
}

function row(session: PublicScheduleRow) {
  return {
    id: session.id,
    to: link(props.editionSlug, session),
    start: time(session.starts_at, props.timezone),
    end: time(session.ends_at, props.timezone),
    duration: duration(session),
    title: tr(session.title),
    acronym: session.organization_acronym,
    organization: session.organization_name,
    country: session.organization_country ? tr(session.organization_country) : '',
    format: t(`session-card.format.${session.format}`),
    streamed: session.is_streamed,
    current: state(session),
    themes: session.themes.slice(0, MAX_THEMES).map((theme) => ({
      code: theme.code,
      label: tr(theme.label),
      background: fill(theme.color),
    })),
    moreThemes: Math.max(session.themes.length - MAX_THEMES, 0),
    visited: session.id === props.visitedId,
  }
}

const sections = computed(() =>
  props.days.map((day) => {
    const own = props.sessions.filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
    const rows = own.filter(props.matches).map(row)
    return { day, weekday: weekday(day), date: fullDate(day), rows, total: own.length }
  }),
)

const shown = computed(() => sections.value.reduce((sum, section) => sum + section.rows.length, 0))

const anchor = (day: IsoDate) => `programme-jour-${day}`

let locked = 0
let frame = 0

function spy(): void {
  cancelAnimationFrame(frame)
  frame = requestAnimationFrame(() => {
    if (Date.now() < locked) return
    let current: IsoDate | null = null
    for (const section of sections.value) {
      const element = document.getElementById(anchor(section.day))
      if (!element) continue
      if (element.getBoundingClientRect().top - props.stickyOffset <= 24) current = section.day
      else break
    }
    const atBottom = window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 4
    const last = sections.value[sections.value.length - 1]?.day
    const reading = (atBottom && last) || current || sections.value[0]?.day
    if (reading) emit('reading', reading)
  })
}

function scrollToDay(day: IsoDate, smooth = true): void {
  const element = document.getElementById(anchor(day))
  if (!element) return
  locked = Date.now() + (smooth ? 900 : 100)
  const top = element.getBoundingClientRect().top + window.scrollY - props.stickyOffset + 1
  window.scrollTo({ top, behavior: smooth ? 'smooth' : 'auto' })
}

defineExpose({ scrollToDay })

onMounted(() => window.addEventListener('scroll', spy, { passive: true }))
onBeforeUnmount(() => {
  window.removeEventListener('scroll', spy)
  cancelAnimationFrame(frame)
})
</script>

<template>
  <div>
    <section
      v-for="section in sections"
      :id="anchor(section.day)"
      :key="section.day"
      class="pb-6"
      :aria-labelledby="`${anchor(section.day)}-titre`"
    >
      <header class="flex flex-wrap items-end justify-between gap-x-4 gap-y-1 border-b-4 border-poster-ink pt-6 pb-2">
        <h3
          :id="`${anchor(section.day)}-titre`"
          class="font-poster text-[clamp(2rem,4.5vw,2.75rem)] leading-[0.9] font-black tracking-[-0.01em] uppercase font-stretch-[62%]"
        >
          {{ section.weekday }}
          <span class="ml-2 font-poster-mono text-sm font-semibold tracking-[0.08em] text-poster-ink-muted normal-case">
            {{ section.date }}
          </span>
        </h3>
        <p class="font-poster-mono text-xs text-poster-ink-muted">
          {{ t('programme.list.count', { count: section.rows.length, total: section.total }, section.total) }}
        </p>
      </header>

      <p v-if="!section.rows.length" class="border-b-2 border-poster-ink px-2 py-4 text-sm text-poster-ink-muted sm:px-4">
        {{ t('programme.list.emptyFiltered') }}
      </p>

      <ol>
        <li
          v-for="entry in section.rows"
          :key="entry.id"
          class="grid gap-3 border-b-2 border-poster-ink px-2 py-4 sm:px-4 md:grid-cols-[8.5rem_minmax(0,1fr)_11rem] md:items-center md:gap-6"
          :class="[entry.current === 'live' ? 'bg-poster-live-row' : '', entry.current === 'past' || entry.current === 'cancelled' ? 'opacity-60' : '']"
        >
          <div class="flex items-baseline gap-3 md:block">
            <p class="font-poster-mono text-[2rem] leading-none font-semibold tracking-[-0.03em]">{{ entry.start }}</p>
            <p class="font-poster-mono text-[0.8125rem] text-poster-ink-muted md:mt-1.5">
              {{ t('programme.list.until', { end: entry.end, duration: entry.duration }) }}
            </p>
          </div>

          <div class="flex min-w-0 flex-col gap-1.5">
            <div class="flex flex-wrap items-center gap-2">
              <span
                v-for="theme in entry.themes"
                :key="theme.code"
                class="inline-flex h-6 items-center rounded-full border-2 border-poster-ink px-2.5 text-xs font-semibold"
                :style="{ background: theme.background }"
              >
                {{ theme.label }}
              </span>
              <span v-if="entry.moreThemes" class="font-poster-mono text-xs">+{{ entry.moreThemes }}</span>
              <span class="font-poster-mono text-xs text-poster-ink-muted">
                {{ entry.format }}<template v-if="entry.streamed"> · {{ t('programme.list.streamed') }}</template>
              </span>
              <span
                v-if="entry.current === 'past' || entry.current === 'cancelled' || entry.current === 'postponed'"
                class="font-poster-mono text-xs font-semibold tracking-[0.08em] uppercase"
                :class="entry.current === 'postponed' ? 'text-postponed' : 'text-poster-ink-muted'"
              >
                {{ t(`session-card.state.${entry.current}`) }}
              </span>
            </div>
            <p
              class="font-poster text-[1.375rem] leading-[1.1] font-extrabold font-stretch-[85%]"
              :class="{ 'line-through': entry.current === 'cancelled' }"
            >
              {{ entry.title }}
            </p>
            <p v-if="entry.organization" class="text-sm text-poster-ink-muted">
              <strong v-if="entry.acronym" class="text-poster-ink">{{ entry.acronym }}</strong>
              <template v-if="entry.acronym"> — </template>{{ entry.organization }}<template v-if="entry.country"> · {{ entry.country }}</template>
            </p>
          </div>

          <div class="flex items-center gap-3 md:flex-col md:items-end">
            <span
              v-if="entry.current === 'live'"
              class="-rotate-6 rounded border-[3px] border-live bg-poster-paper px-2 py-0.5 font-poster text-lg font-black tracking-[0.06em] text-live font-stretch-[70%]"
            >
              {{ t('programme.list.live') }}
            </span>
            <NuxtLink
              :to="entry.to"
              class="inline-flex h-11 items-center gap-2 rounded-md border-2 border-poster-ink px-3.5 text-sm font-bold transition-transform"
              :class="entry.visited ? 'bg-poster-ink text-poster-on-ink-accent' : 'bg-poster-paper-raised text-poster-ink shadow-poster-sm hover:-translate-y-0.5'"
              :aria-label="`${t(entry.visited ? 'programme.list.visited' : 'programme.list.open')} : ${entry.title}`"
            >
              {{ t(entry.visited ? 'programme.list.visited' : 'programme.list.open') }}
              <UiIcon name="arrow-right" size="1rem" />
            </NuxtLink>
          </div>
        </li>
      </ol>
    </section>

    <div v-if="!shown" class="flex flex-col items-start gap-4 py-10">
      <p class="font-poster text-3xl font-extrabold font-stretch-[85%]">{{ t('programme.list.emptyAll') }}</p>
      <button
        type="button"
        class="h-11 cursor-pointer rounded-md border-2 border-poster-ink bg-poster-paper-raised px-4 font-bold text-poster-ink shadow-poster-sm"
        @click="emit('reset')"
      >
        {{ t('programme.filters.clear') }}
      </button>
    </div>
  </div>
</template>
