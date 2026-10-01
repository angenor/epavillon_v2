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
const { state, fill, link, themeColor } = useProgrammeSession()

const MAX_THEMES = 3
const NATIONAL = 'public_national_institution'

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
    cover: session.cover,
    logo: session.organization_logo ? (session.organization_logo.sources.thumb?.url ?? session.organization_logo.url) : null,
    color: themeColor(session),
    flag:
      session.organization_type_code === NATIONAL && session.organization_country_code
        ? { code: session.organization_country_code, label: session.organization_country ? tr(session.organization_country) : session.organization_country_code }
        : null,
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

      <p v-if="!section.rows.length" class="border-b border-poster-line px-2 py-4 text-sm text-poster-ink-muted sm:px-4">
        {{ t('programme.list.emptyFiltered') }}
      </p>

      <ol>
        <li
          v-for="entry in section.rows"
          :key="entry.id"
          class="grid gap-4 border-b border-poster-line px-2 py-5 sm:px-4 md:grid-cols-[7.5rem_16.5rem_minmax(0,1fr)_10rem] md:items-center md:gap-7"
          :class="[entry.current === 'live' ? 'bg-poster-live-row' : '', entry.current === 'past' || entry.current === 'cancelled' ? 'opacity-60' : '']"
        >
          <div class="flex items-baseline gap-3 md:block">
            <p class="font-poster-mono text-[2rem] leading-none font-semibold tracking-[-0.03em]">{{ entry.start }}</p>
            <p class="font-poster-mono text-[0.8125rem] text-poster-ink-muted md:mt-1.5">
              {{ t('programme.list.until', { end: entry.end, duration: entry.duration }) }}
            </p>
          </div>

          <NuxtLink
            :to="entry.to"
            class="relative block aspect-video overflow-hidden rounded-md border-2 md:aspect-auto md:h-37"
            :class="entry.current === 'live' ? 'border-live' : 'border-poster-ink'"
            :style="{
              background: fill(entry.color),
              boxShadow: entry.current === 'past' || entry.current === 'cancelled' ? undefined : `5px 5px 0 ${entry.current === 'live' ? 'var(--color-live)' : (entry.color ?? 'var(--color-poster-ink)')}`,
            }"
            tabindex="-1"
            aria-hidden="true"
          >
            <UiImage
              v-if="entry.cover"
              :image="entry.cover"
              ratio="auto"
              frame-class="size-full"
              class="size-full"
              :class="{ grayscale: entry.current === 'past' || entry.current === 'cancelled' }"
              sizes="(min-width: 768px) 264px, 100vw"
            />
            <span
              v-if="entry.current === 'live'"
              class="absolute top-2.5 left-2.5 rounded-sm bg-live px-2 py-0.5 font-poster-mono text-[0.6875rem] font-semibold tracking-[0.08em] text-live-contrast uppercase"
            >
              ● {{ t('programme.list.live') }}
            </span>
            <span class="absolute bottom-2.5 left-2.5 flex max-w-[calc(100%-5.5rem)] flex-wrap gap-1">
              <span
                v-for="theme in entry.themes.slice(0, 1)"
                :key="theme.code"
                class="inline-flex h-6 items-center truncate rounded-full border-2 border-poster-ink px-2.5 text-[0.6875rem] font-bold text-poster-ink"
                :style="{ background: theme.background }"
              >
                {{ theme.label }}
              </span>
              <span
                v-if="entry.themes.length + entry.moreThemes > 1"
                class="inline-flex h-6 items-center rounded-full border-2 border-poster-ink bg-poster-paper-raised px-2 font-poster-mono text-[0.6875rem] font-semibold text-poster-ink"
              >
                +{{ entry.themes.length + entry.moreThemes - 1 }}
              </span>
            </span>
            <span
              class="absolute right-2.5 bottom-2.5 flex h-13 max-w-32 min-w-13 items-center justify-center rounded-lg border-2 border-poster-ink bg-poster-paper-raised px-1.5 py-1"
            >
              <img v-if="entry.logo" :src="entry.logo" alt="" class="max-h-9.5 max-w-28 object-contain" loading="lazy">
              <span v-else class="font-poster text-[0.8125rem] font-black text-poster-ink font-stretch-[80%]">
                {{ entry.acronym ?? '—' }}
              </span>
            </span>
          </NuxtLink>

          <div class="flex min-w-0 flex-col gap-1.5">
            <p class="font-poster-mono text-xs text-poster-ink-muted">
              {{ entry.format }}<template v-if="entry.streamed"> · {{ t('programme.list.streamed') }}</template>
              <span
                v-if="entry.current === 'past' || entry.current === 'cancelled' || entry.current === 'postponed'"
                class="ml-1.5 font-semibold tracking-[0.08em] uppercase"
                :class="{ 'text-postponed': entry.current === 'postponed' }"
              >
                {{ t(`session-card.state.${entry.current}`) }}
              </span>
            </p>
            <p
              class="font-poster text-[1.5rem] leading-[1.1] font-extrabold font-stretch-[85%]"
              :class="{ 'line-through': entry.current === 'cancelled' }"
            >
              {{ entry.title }}
            </p>
            <p v-if="entry.organization" class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-poster-ink-muted">
              <UiCountryFlag v-if="entry.flag" :code="entry.flag.code" :label="entry.flag.label" />
              <span>
                <strong v-if="entry.acronym" class="text-poster-ink">{{ entry.acronym }}</strong>
                <template v-if="entry.acronym"> — </template>{{ entry.organization }}<template v-if="entry.country"> · {{ entry.country }}</template>
              </span>
              <span
                v-if="entry.flag"
                class="inline-flex h-5.5 items-center rounded-sm border-[1.5px] border-poster-ink px-2 font-poster-mono text-[0.625rem] font-semibold tracking-[0.06em] text-poster-ink uppercase"
              >
                {{ t('programme.list.national') }}
              </span>
            </p>
          </div>

          <div class="flex items-center gap-3 md:justify-end">
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
