<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'

/** La liste d'un jour, écrite comme une affiche : le jour en très grand, une ligne par activité. */

interface Props {
  day: IsoDate
  sessions: PublicScheduleRow[]
  matches: (session: PublicScheduleRow) => boolean
  timezone: TimeZoneName
  zone: string
  previous: IsoDate | null
  next: IsoDate | null
  visitedId: string | null
  editionSlug: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ select: [date: IsoDate]; reset: [] }>()

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
const shortDay = (day: IsoDate) =>
  new Intl.DateTimeFormat(locale.value, { weekday: 'long', day: 'numeric', timeZone: 'UTC' }).format(at(day))

function duration(session: PublicScheduleRow): string {
  const minutes = Math.round((Date.parse(session.ends_at) - Date.parse(session.starts_at)) / 60_000)
  const hours = Math.floor(minutes / 60)
  const rest = minutes % 60
  if (!hours) return t('programme.list.minutes', { minutes })
  return rest ? t('programme.list.hoursMinutes', { hours, minutes: String(rest).padStart(2, '0') }) : t('programme.list.hours', { hours })
}

const rows = computed(() =>
  props.sessions.filter(props.matches).map((session) => {
    const current = state(session)
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
      current,
      themes: session.themes.slice(0, MAX_THEMES).map((theme) => ({
        code: theme.code,
        label: tr(theme.label),
        background: fill(theme.color),
      })),
      moreThemes: Math.max(session.themes.length - MAX_THEMES, 0),
      visited: session.id === props.visitedId,
    }
  }),
)
</script>

<template>
  <div>
    <header class="flex flex-wrap items-end justify-between gap-4 border-b-4 border-poster-ink pb-3.5">
      <div class="min-w-0">
        <p class="font-poster-mono text-sm font-semibold tracking-[0.08em] text-poster-ink-muted uppercase">
          {{ fullDate(props.day) }}
        </p>
        <h3
          class="mt-1.5 font-poster text-[clamp(4rem,11vw,8.25rem)] leading-[0.82] font-black tracking-[-0.02em] uppercase font-stretch-[62%]"
        >
          {{ weekday(props.day) }}
        </h3>
      </div>
      <p class="font-poster-mono text-sm leading-relaxed text-poster-ink-muted sm:text-right">
        {{ t('programme.list.count', { count: rows.length, total: props.sessions.length }) }}<br>
        {{ props.zone }}
      </p>
    </header>

    <ol>
      <li
        v-for="row in rows"
        :key="row.id"
        class="grid gap-4 border-b-2 border-poster-ink px-2 py-5 sm:px-4 md:grid-cols-[9.5rem_minmax(0,1fr)_11rem] md:items-center md:gap-6"
        :class="[row.current === 'live' ? 'bg-poster-live-row' : '', row.current === 'past' || row.current === 'cancelled' ? 'opacity-60' : '']"
      >
        <div class="flex items-baseline gap-3 md:block">
          <p class="font-poster-mono text-[2.5rem] leading-none font-semibold tracking-[-0.03em]">{{ row.start }}</p>
          <p class="font-poster-mono text-sm text-poster-ink-muted md:mt-2">
            {{ t('programme.list.until', { end: row.end, duration: row.duration }) }}
          </p>
        </div>

        <div class="flex min-w-0 flex-col gap-2">
          <div class="flex flex-wrap items-center gap-2">
            <span
              v-for="theme in row.themes"
              :key="theme.code"
              class="inline-flex h-6.5 items-center rounded-full border-2 border-poster-ink px-2.5 text-xs font-semibold"
              :style="{ background: theme.background }"
            >
              {{ theme.label }}
            </span>
            <span v-if="row.moreThemes" class="font-poster-mono text-xs">+{{ row.moreThemes }}</span>
            <span class="font-poster-mono text-xs text-poster-ink-muted">
              {{ row.format }}<template v-if="row.streamed"> · {{ t('programme.list.streamed') }}</template>
            </span>
            <span
              v-if="row.current === 'past' || row.current === 'cancelled' || row.current === 'postponed'"
              class="font-poster-mono text-xs font-semibold tracking-[0.08em] uppercase"
              :class="row.current === 'postponed' ? 'text-postponed' : 'text-poster-ink-muted'"
            >
              {{ t(`session-card.state.${row.current}`) }}
            </span>
          </div>
          <p
            class="font-poster text-2xl leading-[1.1] font-extrabold font-stretch-[85%] sm:text-[1.625rem]"
            :class="{ 'line-through': row.current === 'cancelled' }"
          >
            {{ row.title }}
          </p>
          <p v-if="row.organization" class="text-sm text-poster-ink-muted">
            <strong v-if="row.acronym" class="text-poster-ink">{{ row.acronym }}</strong>
            <template v-if="row.acronym"> — </template>{{ row.organization }}<template v-if="row.country"> · {{ row.country }}</template>
          </p>
        </div>

        <div class="flex items-center gap-3 md:flex-col md:items-end">
          <span
            v-if="row.current === 'live'"
            class="-rotate-6 rounded border-[3px] border-live bg-poster-paper px-2.5 py-0.5 font-poster text-xl font-black tracking-[0.06em] text-live font-stretch-[70%]"
          >
            {{ t('programme.list.live') }}
          </span>
          <NuxtLink
            :to="row.to"
            class="inline-flex h-12 items-center gap-2 rounded-md border-2 border-poster-ink px-4 font-bold transition-transform"
            :class="row.visited ? 'bg-poster-ink text-poster-on-ink-accent' : 'bg-poster-paper-raised text-poster-ink shadow-poster-sm hover:-translate-y-0.5'"
            :aria-label="`${t(row.visited ? 'programme.list.visited' : 'programme.list.open')} : ${row.title}`"
          >
            {{ t(row.visited ? 'programme.list.visited' : 'programme.list.open') }}
            <UiIcon name="arrow-right" size="1rem" />
          </NuxtLink>
        </div>
      </li>
    </ol>

    <div v-if="!rows.length" class="flex flex-col items-start gap-4 border-b-2 border-poster-ink py-14">
      <p class="font-poster text-3xl font-extrabold font-stretch-[85%]">
        {{ t(props.sessions.length ? 'programme.list.emptyFiltered' : 'programme.list.empty') }}
      </p>
      <button
        v-if="props.sessions.length"
        type="button"
        class="h-12 cursor-pointer rounded-md border-2 border-poster-ink bg-poster-paper-raised px-4 font-bold text-poster-ink shadow-poster-sm"
        @click="emit('reset')"
      >
        {{ t('programme.filters.clear') }}
      </button>
    </div>

    <nav class="mt-6 flex flex-wrap justify-between gap-3" :aria-label="t('programme.list.navigation')">
      <button
        v-if="props.previous"
        type="button"
        class="inline-flex h-12 cursor-pointer items-center gap-2 rounded-md border-2 border-poster-ink px-4 font-bold text-poster-ink"
        @click="emit('select', props.previous)"
      >
        <UiIcon name="arrow-left" size="1rem" />
        {{ shortDay(props.previous) }}
      </button>
      <span v-else />
      <button
        v-if="props.next"
        type="button"
        class="inline-flex h-12 cursor-pointer items-center gap-2 rounded-md border-2 border-poster-ink bg-poster-ink px-4 font-bold text-poster-on-ink-accent"
        @click="emit('select', props.next)"
      >
        {{ shortDay(props.next) }}
        <UiIcon name="arrow-right" size="1rem" />
      </button>
    </nav>
  </div>
</template>
