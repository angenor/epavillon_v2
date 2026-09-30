<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { PlacedSession } from '~/utils/programme-week'

/**
 * La semaine : une colonne par jour de programme, les activités posées à leur
 * heure dans le fuseau de l'ÉDITION. Un clic ouvre la page de l'activité.
 *
 * Une activité écartée par les filtres reste en creux, sans texte : le rythme
 * de la semaine se lit toujours. La rangée des jours reste figée sous la barre
 * de navigation au défilement ; elle ne peut donc avoir aucun ancêtre en
 * `overflow: hidden`. Sous `md`, seule la colonne du jour choisi s'affiche et
 * la rangée sert d'onglets.
 */

interface Props {
  days: IsoDate[]
  sessions: PublicScheduleRow[]
  matches: (session: PublicScheduleRow) => boolean
  timezone: TimeZoneName
  range: [number, number]
  selected: IsoDate
  today: IsoDate | null
  /** Minutes écoulées aujourd'hui dans le fuseau de l'édition ; `null` avant le montage. */
  nowMinutes: number | null
  visitedId: string | null
  editionSlug: string
  /** « Semaine du 9 au 14 novembre ». */
  label: string
  /** « 3 sur 25 activités ». */
  summary: string
  zone: string
  hasPrevious: boolean
  hasNext: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ week: [step: -1 | 1]; day: [date: IsoDate] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { dayLong, zoneOffsetShort } = useDateTime()
const { state, themeColor, fill, link } = useProgrammeSession()

const HOUR = 80
const zoneOffset = computed(() => zoneOffsetShort(props.timezone, props.days[0] ? `${props.days[0]}T12:00:00Z` : undefined))
const pixels = (minutes: number) => ((minutes - props.range[0] * 60) * HOUR) / 60

const hours = computed(() =>
  Array.from({ length: props.range[1] - props.range[0] }, (_, index) => {
    const hour = props.range[0] + index
    return { hour, top: index * HOUR, label: `${String(hour).padStart(2, '0')}:00` }
  }),
)

const clock = (minutes: number) =>
  `${String(Math.floor(minutes / 60) % 24).padStart(2, '0')}:${String(minutes % 60).padStart(2, '0')}`

const columns = computed(() =>
  props.days.map((day) => {
    const own = props.sessions.filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
    const placed = layoutDay(own.map((session) => spanOf(session, props.timezone)))
    const kept = own.filter(props.matches).length
    const isToday = day === props.today
    return {
      date: day,
      isToday,
      selected: day === props.selected,
      number: day.slice(8),
      weekday: new Intl.DateTimeFormat(locale.value, { weekday: 'short', timeZone: 'UTC' }).format(new Date(`${day}T12:00:00Z`)),
      label: dayLong(`${day}T12:00:00Z`, props.timezone),
      sub: isToday ? t('programme.week.today') : t('programme.days.count', kept),
      blocks: placed.map((item) => block(item)),
    }
  }),
)

function block(item: PlacedSession) {
  const session = item.session
  const kept = props.matches(session)
  const current = state(session)
  const color = themeColor(session)
  const height = pixels(item.end) - pixels(item.start) - 4
  const compact = height < 90
  const visited = session.id === props.visitedId
  const time = `${clock(item.start)}–${clock(item.end)}`

  let look = 'border-2 border-poster-ink text-poster-ink shadow-poster hover:-translate-y-0.5'
  let background = fill(color)
  let shadow: string | undefined
  if (current === 'past' || current === 'cancelled') {
    look = 'border-2 border-poster-line text-poster-ink-muted'
    background = 'var(--color-poster-past)'
  } else if (current === 'postponed') {
    look = 'border-2 border-dashed border-postponed-border text-poster-ink'
  } else if (current === 'live') {
    look = 'border-[3px] border-live text-poster-ink hover:-translate-y-0.5'
    shadow = '4px 4px 0 var(--color-live)'
  }
  if (visited && kept) {
    look = 'border-2 border-poster-ink text-poster-on-ink'
    background = 'var(--color-poster-ink)'
    shadow = `5px 5px 0 ${color ?? 'var(--color-poster-line)'}`
  }
  if (!kept) {
    look = 'border-2 border-dashed border-poster-line text-poster-ink-muted'
    background = 'transparent'
    shadow = undefined
  }

  return {
    id: session.id,
    to: link(props.editionSlug, session),
    kept,
    current,
    time,
    title: tr(session.title),
    acronym: session.organization_acronym ?? session.organization_name ?? '',
    streamed: session.is_streamed,
    showAcronym: kept && !compact,
    lines: Math.max(1, Math.floor((height - (compact ? 30 : 46)) / 17)),
    look,
    style: {
      top: `${pixels(item.start) + 2}px`,
      height: `${height}px`,
      left: `calc(${(item.lane / item.lanes) * 100}% + 5px)`,
      width: `calc(${100 / item.lanes}% - 10px)`,
      background,
      boxShadow: shadow,
      zIndex: visited ? 3 : 1,
    },
    label: [tr(session.title), time, session.organization_name, t(`session-card.state.${current === 'live' ? 'ongoing' : current}`)]
      .filter(Boolean)
      .join(', '),
  }
}

const nowTop = computed(() => {
  const now = props.nowMinutes
  if (now === null || now < props.range[0] * 60 || now > props.range[1] * 60) return null
  return pixels(now)
})
</script>

<template>
  <div>
    <nav
      class="flex flex-wrap items-center gap-x-3 gap-y-2 rounded-t-md border-2 border-poster-ink bg-poster-ink px-3 py-2 text-poster-on-ink"
      :aria-label="t('programme.week.navigation')"
    >
      <p class="font-poster text-xl font-extrabold uppercase font-stretch-[75%]">{{ props.label }}</p>
      <p class="font-poster-mono text-xs text-poster-on-ink-muted">{{ props.summary }} · {{ props.zone }}</p>
      <div class="ml-auto flex gap-2">
        <button
          v-if="props.hasPrevious"
          type="button"
          class="inline-flex h-11 cursor-pointer items-center gap-2 rounded-md border-2 border-poster-on-ink-muted px-3 text-sm font-bold"
          @click="emit('week', -1)"
        >
          <UiIcon name="arrow-left" size="1rem" />
          <span class="hidden sm:inline">{{ t('programme.week.previous') }}</span>
          <span class="sr-only sm:hidden">{{ t('programme.week.previous') }}</span>
        </button>
        <button
          v-if="props.hasNext"
          type="button"
          class="inline-flex h-11 cursor-pointer items-center gap-2 rounded-md border-2 border-poster-today-strong bg-poster-today-strong px-3.5 text-sm font-bold text-poster-on-today"
          @click="emit('week', 1)"
        >
          {{ t('programme.week.next') }}
          <UiIcon name="arrow-right" size="1rem" />
        </button>
      </div>
    </nav>

    <div class="rounded-b-md border-2 border-t-0 border-poster-ink bg-poster-paper-raised">
      <div
        class="sticky top-(--nav-height) z-20 flex border-b-2 border-poster-ink bg-poster-paper-raised"
      >
        <div class="flex w-14 shrink-0 items-end justify-end px-1.5 pb-1 font-poster-mono text-[0.625rem] text-poster-ink-muted">
          {{ zoneOffset }}
        </div>
        <button
          v-for="column in columns"
          :key="column.date"
          type="button"
          class="flex min-w-0 flex-1 cursor-pointer flex-col items-start gap-0.5 border-l border-poster-line px-2 py-2 text-left md:flex-row md:items-center md:gap-2.5 md:px-3 md:py-2.5"
          :class="
            column.isToday
              ? 'bg-poster-today-strong text-poster-on-today'
              : column.selected
                ? 'bg-poster-paper-sunken text-poster-ink'
                : 'text-poster-ink hover:bg-poster-paper-sunken'
          "
          :aria-current="column.selected ? 'date' : undefined"
          :title="t('programme.week.openDay', { day: column.label })"
          @click="emit('day', column.date)"
        >
          <span class="font-poster text-3xl leading-none font-black font-stretch-[62%] md:text-4xl">{{ column.number }}</span>
          <span class="flex min-w-0 flex-col gap-0.5 font-poster-mono text-[0.6875rem]">
            <span class="font-semibold tracking-[0.08em] uppercase">{{ column.weekday }}</span>
            <span class="hidden truncate md:block">{{ column.sub }}</span>
          </span>
        </button>
      </div>

      <div class="relative flex" :style="{ height: `${hours.length * HOUR}px` }">
        <div class="relative w-14 shrink-0" aria-hidden="true">
          <span
            v-for="hour in hours"
            :key="hour.hour"
            class="absolute right-2 font-poster-mono text-[0.6875rem] text-poster-ink-muted"
            :style="{ top: `${Math.max(4, hour.top - 7)}px` }"
          >
            {{ hour.label }}
          </span>
          <span
            v-if="nowTop !== null && columns.some((column) => column.isToday)"
            class="absolute right-1 rounded-sm bg-live px-1 py-px font-poster-mono text-[0.6875rem] font-semibold text-live-contrast"
            :style="{ top: `${nowTop - 9}px` }"
          >
            {{ clock(props.nowMinutes ?? 0) }}
          </span>
        </div>

        <div class="pointer-events-none absolute inset-y-0 right-0 left-14" aria-hidden="true">
          <template v-for="hour in hours" :key="hour.hour">
            <div class="absolute inset-x-0 border-t border-poster-line" :style="{ top: `${hour.top}px` }" />
            <div class="absolute inset-x-0 border-t border-dashed border-poster-line/50" :style="{ top: `${hour.top + HOUR / 2}px` }" />
          </template>
        </div>

        <section
          v-for="column in columns"
          :key="column.date"
          class="relative min-w-0 flex-1 border-l border-poster-line"
          :class="[column.selected ? 'block' : 'hidden md:block', column.isToday ? 'bg-poster-today' : '']"
          :aria-label="column.label"
        >
          <ol class="contents">
            <li v-for="item in column.blocks" :key="item.id" class="contents">
              <NuxtLink
                :to="item.to"
                class="absolute flex flex-col gap-0.5 overflow-hidden rounded-[5px] px-2 py-1.5 transition-transform focus-visible:z-10"
                :class="item.look"
                :style="item.style"
                :aria-label="item.label"
              >
                <span class="flex w-full items-center justify-between gap-1 font-poster-mono text-[0.6875rem] font-semibold">
                  <span class="truncate">{{ item.time }}</span>
                  <span
                    v-if="item.kept && item.current === 'live'"
                    class="shrink-0 rounded-sm bg-live px-1 text-[0.625rem] tracking-[0.06em] text-live-contrast uppercase"
                  >
                    {{ t('programme.week.live') }}
                  </span>
                  <UiIcon v-else-if="item.kept && item.streamed" name="broadcast" size="0.8125rem" class="shrink-0" />
                </span>
                <template v-if="item.kept">
                  <span
                    class="overflow-hidden font-poster text-[0.9375rem] leading-[1.12] font-bold font-stretch-[78%] [display:-webkit-box] [-webkit-box-orient:vertical]"
                    :class="{ 'line-through': item.current === 'cancelled' }"
                    :style="{ WebkitLineClamp: item.lines }"
                  >
                    {{ item.title }}
                  </span>
                  <span
                    v-if="item.current === 'postponed' || item.current === 'cancelled'"
                    class="text-[0.625rem] font-bold tracking-[0.08em] uppercase"
                    :class="item.current === 'postponed' ? 'text-postponed' : ''"
                  >
                    {{ t(`session-card.state.${item.current}`) }}
                  </span>
                  <span
                    v-else-if="item.showAcronym"
                    class="mt-auto truncate text-[0.625rem] font-bold tracking-[0.08em] uppercase"
                  >
                    {{ item.acronym }}
                  </span>
                </template>
              </NuxtLink>
            </li>
          </ol>

          <div
            v-if="column.isToday && nowTop !== null"
            class="pointer-events-none absolute inset-x-0 z-5 border-t-2 border-live"
            :style="{ top: `${nowTop}px` }"
            aria-hidden="true"
          >
            <span class="absolute -top-[7px] -left-1.5 size-3 rounded-full bg-live" />
          </div>
        </section>
      </div>
    </div>

    <div class="mt-3 flex flex-wrap items-center justify-between gap-3">
      <ul class="flex flex-wrap items-center gap-x-5 gap-y-2 font-poster-mono text-xs text-poster-ink-muted">
        <li class="inline-flex items-center gap-2">
          <span class="h-3.5 w-5.5 rounded-sm border-2 border-poster-ink bg-poster-paper-sunken" />{{ t('programme.week.legend.activity') }}
        </li>
        <li class="inline-flex items-center gap-2">
          <span class="h-3.5 w-5.5 rounded-sm border-2 border-dashed border-poster-line" />{{ t('programme.week.legend.filtered') }}
        </li>
        <li class="inline-flex items-center gap-2">
          <span class="h-3.5 w-5.5 rounded-sm border-2 border-poster-line bg-poster-past" />{{ t('programme.week.legend.past') }}
        </li>
        <li class="inline-flex items-center gap-2">
          <span class="h-3.5 w-5.5 rounded-sm border-[3px] border-live" />{{ t('programme.week.legend.live') }}
        </li>
      </ul>
      <div class="flex gap-2">
        <button
          v-if="props.hasPrevious"
          type="button"
          class="inline-flex h-11 cursor-pointer items-center gap-2 rounded-md border-2 border-poster-ink bg-poster-paper-raised px-3.5 text-sm font-bold text-poster-ink"
          @click="emit('week', -1)"
        >
          <UiIcon name="arrow-left" size="1rem" />
          {{ t('programme.week.previous') }}
        </button>
        <button
          v-if="props.hasNext"
          type="button"
          class="inline-flex h-11 cursor-pointer items-center gap-2 rounded-md border-2 border-poster-ink bg-poster-today-strong px-4 text-sm font-bold text-poster-on-today shadow-poster-sm"
          @click="emit('week', 1)"
        >
          {{ t('programme.week.next') }}
          <UiIcon name="arrow-right" size="1rem" />
        </button>
      </div>
    </div>
  </div>
</template>
