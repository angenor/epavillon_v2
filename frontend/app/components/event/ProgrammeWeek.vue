<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'

/**
 * La semaine en colonnes : une par journée, les activités retenues par la
 * recherche empilées dans l'ordre. Sous `md`, seule la journée choisie s'affiche
 * et la rangée d'onglets, figée sous la barre du site, permet d'en changer :
 * aucun ancêtre ne doit donc être en `overflow: hidden`.
 */

interface Props {
  days: IsoDate[]
  sessions: PublicScheduleRow[]
  matches: (session: PublicScheduleRow) => boolean
  timezone: TimeZoneName
  selected: IsoDate
  today: IsoDate | null
  visitedId: string | null
  editionSlug: string
  /** Rang de la semaine affichée, à partir de 1. */
  week: number
  weeks: number
  hasPrevious: boolean
  hasNext: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ week: [step: -1 | 1]; day: [date: IsoDate] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time, intlLocale } = useDateTime()
const { state, link, duration, titleParts } = useProgrammeSession()

const NOTE_TONES: Partial<Record<ProgrammeSessionState, string>> = {
  ongoing: 'text-warning',
  past: 'text-text-muted',
  postponed: 'text-postponed',
  cancelled: 'text-text-muted',
}

const at = (day: IsoDate) => new Date(`${day}T12:00:00Z`)
const format = (day: IsoDate, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(intlLocale.value, { ...options, timeZone: 'UTC' }).format(at(day))

const heading = computed(() => {
  const first = props.days[0]
  const last = props.days[props.days.length - 1]
  if (!first || !last) return { lead: '', month: '' }
  const month = format(first, { month: 'long' })
  if (first === last) return { lead: t('programme.week.single', { day: format(first, { day: 'numeric' }) }), month }
  if (first.slice(0, 7) === last.slice(0, 7)) {
    return {
      lead: t('programme.week.range', { from: format(first, { day: 'numeric' }), to: format(last, { day: 'numeric' }) }),
      month,
    }
  }
  return {
    lead: t('programme.week.range', {
      from: format(first, { day: 'numeric', month: 'long' }),
      to: format(last, { day: 'numeric', month: 'long' }),
    }),
    month: '',
  }
})


function card(session: PublicScheduleRow) {
  const current = state(session)
  const { lead, rest } = titleParts(tr(session.title))
  return {
    id: session.id,
    to: link(props.editionSlug, session),
    current,
    faded: current === 'past' || current === 'cancelled',
    cover: session.cover,
    start: time(session.starts_at, props.timezone),
    until: t('programme.list.until', { end: time(session.ends_at, props.timezone), duration: duration(session) }),
    lead,
    rest,
    acronym: session.organization_acronym ?? session.organization_name ?? '',
    note: NOTE_TONES[current] ? t(`session-card.state.${current}`).toLocaleLowerCase(locale.value) : '',
    noteTone: NOTE_TONES[current] ?? '',
    visited: session.id === props.visitedId,
  }
}

const columns = computed(() =>
  props.days.map((day) => {
    const own = props.sessions
      .filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
      .sort((a, b) => Date.parse(a.starts_at) - Date.parse(b.starts_at))
    const kept = own.filter(props.matches)
    const special = own.flatMap((session) => session.tracks).find((track) => track.kind === 'special_day')
    return {
      date: day,
      number: format(day, { day: 'numeric' }),
      weekday: format(day, { weekday: 'long' }),
      weekdayShort: format(day, { weekday: 'short' }),
      label: format(day, { weekday: 'long', day: 'numeric', month: 'long' }),
      isToday: day === props.today,
      isPast: props.today !== null && day < props.today,
      selected: day === props.selected,
      count: t('programme.days.count', { count: kept.length }, kept.length),
      special: special ? tr(special.title) : '',
      cards: kept.map(card),
    }
  }),
)

const shown = computed(() => columns.value.reduce((sum, column) => sum + column.cards.length, 0))
</script>

<template>
  <div>
    <div class="flex flex-wrap items-center justify-between gap-4 border-b border-border-subtle py-4">
      <p class="text-[24px] leading-tight font-light text-text">
        <b class="font-bold">{{ heading.lead }}</b> {{ heading.month }}<span class="mt-1 block text-sm text-text-muted sm:mt-0 sm:ml-3.5 sm:inline">{{
          t('programme.week.position', {
            week: props.week,
            weeks: props.weeks,
            count: t('programme.days.count', { count: shown }, shown),
          })
        }}</span>
      </p>
      <div role="group" class="flex gap-2" :aria-label="t('programme.week.navigation')">
        <UiButton
          variant="secondary"
          icon="arrow-left"
          icon-only
          :label="t('programme.week.previous')"
          :disabled="!props.hasPrevious"
          @click="emit('week', -1)"
        />
        <UiButton variant="secondary" icon-trailing="arrow-right" :disabled="!props.hasNext" @click="emit('week', 1)">
          {{ t('programme.week.next') }}
        </UiButton>
      </div>
    </div>

    <div class="sticky top-(--nav-height) z-20 flex overflow-x-auto border-b border-border-subtle bg-surface md:hidden">
      <button
        v-for="column in columns"
        :key="column.date"
        type="button"
        class="-mb-px flex min-h-11 shrink-0 cursor-pointer flex-col items-center gap-1 border-b-2 px-3 py-2"
        :class="[
          column.selected ? (column.isToday ? 'border-accent' : 'border-text') : 'border-transparent',
          column.isPast ? 'text-text-subtle' : 'text-text',
        ]"
        :aria-pressed="column.selected"
        :aria-current="column.isToday ? 'date' : undefined"
        :aria-label="column.label"
        @click="emit('day', column.date)"
      >
        <span class="text-[26px] leading-none tabular-nums" :class="column.isToday ? 'font-bold text-accent' : 'font-light'">
          {{ column.number }}
        </span>
        <span class="text-[11px] font-bold tracking-[0.06em] uppercase">{{ column.weekdayShort }}</span>
      </button>
    </div>

    <div
      class="mt-7 grid grid-cols-1 md:grid-cols-[repeat(var(--week-columns),minmax(0,1fr))] md:gap-x-8"
      :style="{ '--week-columns': String(columns.length) }"
    >
      <section
        v-for="column in columns"
        :key="column.date"
        class="relative isolate min-w-0 pb-3 md:block md:before:absolute md:before:inset-y-0 md:before:-left-4 md:before:border-l md:before:border-border-subtle md:first:before:hidden"
        :class="[column.selected ? 'block' : 'hidden', column.isToday ? 'md:after:absolute md:after:inset-y-0 md:after:-inset-x-4 md:after:-z-10 md:after:bg-accent/6' : '']"
        :aria-label="column.label"
      >
        <button
          type="button"
          class="group/day block w-full cursor-pointer border-b-2 py-3 text-left"
          :class="column.isToday ? 'border-accent' : 'border-text'"
          :aria-label="t('programme.week.openDay', { day: column.label })"
          :aria-current="column.isToday ? 'date' : undefined"
          @click="emit('day', column.date)"
        >
          <span class="flex items-end gap-2.5" :class="column.isPast ? 'text-text-subtle' : 'text-text'">
            <span class="text-[48px] leading-[0.9] tabular-nums" :class="column.isToday ? 'font-bold' : 'font-light'">
              {{ column.number }}
            </span>
            <span class="min-w-0 pb-0.5">
              <span class="block text-sm font-bold first-letter:uppercase group-hover/day:underline">{{ column.weekday }}</span>
              <span class="block text-xs text-text-muted">{{ column.count }}</span>
            </span>
          </span>
          <span class="mt-2 line-clamp-2 h-8 text-[11px] leading-4 font-bold tracking-caps text-accent uppercase">
            {{ column.special }}
          </span>
        </button>

        <ol v-if="column.cards.length">
          <li v-for="item in column.cards" :key="item.id" class="group relative border-b border-border-subtle py-3.5">
            <div :class="{ 'opacity-60': item.faded }">
              <div
                class="relative aspect-video overflow-hidden rounded-[6px] bg-surface-inverse"
                :class="{ 'outline-2 outline-offset-2 outline-live': item.current === 'live' }"
              >
                <UiImage
                  v-if="item.cover"
                  :image="item.cover"
                  ratio="auto"
                  frame-class="size-full"
                  class="absolute inset-0 transition duration-200 motion-safe:group-hover:scale-[1.03]"
                  :class="{ grayscale: item.faded }"
                  sizes="(min-width: 768px) 16rem, 100vw"
                />
                <span
                  v-else
                  class="absolute inset-0 flex items-center justify-center text-[18px] font-bold text-text-on-inverse/20"
                  aria-hidden="true"
                >
                  {{ item.acronym }}
                </span>
                <UiStatusBadge
                  v-if="item.current === 'live'"
                  state="live"
                  size="sm"
                  :label="t('programme.list.live')"
                  class="absolute top-2 left-2"
                />
              </div>

              <p class="mt-2.5 flex items-baseline gap-2">
                <span
                  class="text-[26px] leading-none font-light tabular-nums"
                  :class="item.current === 'live' ? 'text-live' : 'text-text'"
                >
                  {{ item.start }}
                </span>
                <span class="text-xs text-text-muted tabular-nums">{{ item.until }}</span>
              </p>

              <NuxtLink
                :to="item.to"
                class="mt-1.5 block text-[15px] leading-[1.3] text-pretty text-text no-underline after:absolute after:inset-0 group-hover:underline"
                :class="{ 'line-through': item.current === 'cancelled' }"
              >
                <b class="font-bold">{{ item.lead }}</b>{{ item.rest }}
              </NuxtLink>

              <p class="mt-1.5 text-[11px] font-bold tracking-[0.06em] text-text-muted uppercase">
                {{ item.acronym }}<span v-if="item.note" class="font-normal" :class="item.noteTone"> · {{ item.note }}</span><span
                  v-if="item.visited"
                  class="font-normal text-accent"
                > · {{ t('programme.list.visited') }}</span>
              </p>
            </div>
          </li>
        </ol>
        <p v-else class="py-3.5 text-[13px] text-text-muted">{{ t('programme.week.emptyDay') }}</p>
      </section>
    </div>
  </div>
</template>
