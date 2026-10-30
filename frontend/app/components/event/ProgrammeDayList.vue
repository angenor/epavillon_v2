<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { ProgrammeRowData } from './ProgrammeRow.vue'

/**
 * Tout le programme en liste, une section par journée. Au défilement, la journée
 * dont la section passe sous la bande figée est signalée (`reading`) : la page
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
  today: IsoDate | null
  /** Tait les journées strictement antérieures à `today`. */
  hidePast: boolean
  /** Édition passée : ses activités gardent leurs couleurs, sinon plus rien ne donne envie de cliquer. */
  editionPast: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ reading: [date: IsoDate]; reset: [] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { state, link, duration, titleParts } = useProgrammeSession()
const localePath = useLocalePath()

const at = (day: IsoDate) => new Date(`${day}T12:00:00Z`)
const format = (day: IsoDate, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(locale.value, { ...options, timeZone: 'UTC' }).format(at(day))



function row(session: PublicScheduleRow): ProgrammeRowData {
  const current = state(session)
  const faded = current === 'cancelled' || (current === 'past' && !props.editionPast)
  return {
    id: session.id,
    to: link(props.editionSlug, session),
    start: time(session.starts_at, props.timezone),
    until: t('programme.list.until', { end: time(session.ends_at, props.timezone), duration: duration(session) }),
    title: titleParts(tr(session.title)),
    acronym: session.organization_acronym,
    organization: session.organization_name,
    country: session.organization_country ? tr(session.organization_country) : '',
    cover: session.cover,
    current,
    faded,
    stateLabel: current === 'live' ? t('programme.list.live') : t(`session-card.state.${current}`),
  }
}

const visibleDays = computed(() => {
  const today = props.today
  return props.hidePast && today ? props.days.filter((day) => day >= today) : props.days
})

const sections = computed(() =>
  visibleDays.value.map((day) => {
    const own = props.sessions.filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
    const rows = own.filter(props.matches).map(row)
    const specials = new Map<string, string>()
    for (const session of own) {
      for (const track of session.tracks) {
        if (track.kind === 'special_day' && !specials.has(track.slug)) specials.set(track.slug, tr(track.title))
      }
    }
    return {
      day,
      number: format(day, { day: 'numeric' }),
      weekday: format(day, { weekday: 'long' }),
      date: format(day, { day: 'numeric', month: 'long', year: 'numeric' }),
      month: t('programme.days.monthYear', {
        month: format(day, { month: 'long', year: 'numeric' }),
        count: t('programme.days.count', { count: rows.length }, rows.length),
      }),
      specials: [...specials].map(([slug, title]) => ({
        slug,
        title,
        to: localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: slug } }),
      })),
      rows,
      total: own.length,
    }
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
  <div class="font-sans">
    <section
      v-for="section in sections"
      :id="anchor(section.day)"
      :key="section.day"
      class="grid gap-y-4 border-b border-text pt-8 pb-3 lg:grid-cols-[184px_minmax(0,1fr)] lg:gap-x-8"
      :aria-labelledby="`${anchor(section.day)}-titre`"
    >
      <header class="flex flex-wrap items-baseline gap-x-3 self-start lg:block">
        <p class="text-[56px] leading-[0.9] font-light tabular-nums lg:text-[72px]" aria-hidden="true">
          {{ section.number }}
        </p>
        <h3 :id="`${anchor(section.day)}-titre`" class="font-sans text-lg font-bold text-text capitalize lg:mt-2.5">
          {{ section.weekday }}<span class="sr-only"> {{ section.date }}</span>
        </h3>
        <p class="basis-full text-sm text-text-muted lg:mt-0.5">{{ section.month }}</p>
        <div
          v-if="section.specials.length"
          class="mt-3.5 basis-full border-t-2 border-accent pt-2.5 text-xs font-bold text-accent uppercase lg:max-w-42"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          <p v-for="special in section.specials" :key="special.slug">
            <NuxtLink :to="special.to" class="inline-flex min-h-11 items-center text-accent no-underline hover:underline">
              {{ special.title }}
            </NuxtLink>
          </p>
        </div>
      </header>

      <div>
        <p v-if="!section.rows.length && section.total" class="py-4 text-sm text-text-muted">
          {{ t('programme.list.emptyFiltered') }}
        </p>

        <ol>
          <li v-for="(entry, index) in section.rows" :key="entry.id">
            <EventProgrammeRow :entry="entry" :striped="index % 2 === 1" />
          </li>
        </ol>
      </div>
    </section>

    <div v-if="!shown" class="flex flex-col items-start gap-4 py-10">
      <p class="text-2xl font-bold">{{ t('programme.list.emptyAll') }}</p>
      <UiButton variant="secondary" @click="emit('reset')">
        {{ t('programme.filters.clear') }}
      </UiButton>
    </div>
  </div>
</template>
