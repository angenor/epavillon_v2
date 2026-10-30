<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { ProgrammeRowData } from './ProgrammeRow.vue'
import type { PublicScheduleTrack } from '~/types/programme/track-page'
import type { DayFrame, ProgrammeTrackIndex } from '~/utils/programme-tracks'

/**
 * Tout le programme en liste, une section par journée. Les activités d'une
 * journée spéciale publiée y sont réunies dans son cadre (direction F, 30/10),
 * les autres suivent dessous ; le jour de l'atelier porte son seul encadré.
 * Au défilement, la journée
 * dont la section passe sous la bande figée est signalée (`reading`) : la page
 * l'allume dans la bande. `scrollToDay` sert le geste inverse.
 */

interface Props {
  days: IsoDate[]
  sessions: PublicScheduleRow[]
  matches: (session: PublicScheduleRow) => boolean
  /** Fils publiés : un cadre par journée spéciale, un encadré pour l'atelier. */
  tracks: ProgrammeTrackIndex
  matchesTrack: (track: PublicScheduleTrack) => boolean
  /** Une recherche est en cours : les cadres taisent leurs moments. */
  searching: boolean
  /** Jour d'ouverture de l'édition : l'atelier de la veille se dit « la veille ». */
  editionStart: IsoDate
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



const trackTo = (slug: string, hash?: string) =>
  localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: slug }, hash })

function row(session: PublicScheduleRow, track: PublicScheduleTrack | null = null): ProgrammeRowData {
  const current = state(session)
  const faded = current === 'cancelled' || (current === 'past' && !props.editionPast)
  return {
    id: session.id,
    // Dans un cadre, la ligne mène au déroulé de la journée, à sa place ; la fiche y garde son lien.
    to: track ? trackTo(track.slug, `#${trackActivityAnchor(session.slug)}`) : link(props.editionSlug, session),
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

function eveLabel(day: IsoDate): string {
  if (day >= props.editionStart) return ''
  return t(daysBetween(day, props.editionStart) === 1 ? 'programme.tracks.eve' : 'programme.tracks.before')
}

const sections = computed(() =>
  visibleDays.value.map((day) => {
    const own = props.sessions.filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
    const kept = new Set(own.filter(props.matches).map((session) => session.id))

    const sides = props.tracks.sides
      .filter((side) => trackDayOf(side, props.timezone) === day)
      .filter((side) => props.matchesTrack(side) || own.some((session) => kept.has(session.id) && sideOf(session, props.tracks)?.slug === side.slug))
      .map((side) => ({ track: side, to: trackTo(side.slug) }))

    const bySpecial = new Map<string, PublicScheduleRow[]>()
    const loose: PublicScheduleRow[] = []
    for (const session of own) {
      if (sideOf(session, props.tracks)) continue
      const special = specialOf(session, props.tracks)
      if (special) bySpecial.set(special.slug, [...(bySpecial.get(special.slug) ?? []), session])
      else if (kept.has(session.id)) loose.push(session)
    }
    const frames: { frame: DayFrame<ProgrammeRowData>; to: string }[] = []
    for (const [slug, sessions] of bySpecial) {
      const track = props.tracks.specials.get(slug)!
      const frame = composeFrame(track, sessions, kept, day, props.timezone, props.searching, (session) => row(session, track))
      if (frame) frames.push({ frame, to: trackTo(slug) })
    }
    frames.sort((a, b) => (a.frame.start ?? '').localeCompare(b.frame.start ?? ''))

    const rows = loose.map((session) => row(session))
    const activities = rows.length + frames.reduce((sum, entry) => sum + entry.frame.items.filter((item) => item.kind === 'activity').length, 0)
    const eve = sides.length ? eveLabel(day) : ''
    return {
      day,
      number: format(day, { day: 'numeric' }),
      weekday: format(day, { weekday: 'long' }),
      date: format(day, { day: 'numeric', month: 'long', year: 'numeric' }),
      month: t('programme.days.monthYear', {
        month: format(day, { month: 'long', year: 'numeric' }),
        count: eve || t('programme.days.count', { count: activities }, activities),
      }),
      sides,
      frames,
      rows,
      shown: activities + sides.length,
      total: own.length + props.tracks.sides.filter((side) => trackDayOf(side, props.timezone) === day).length,
    }
  }),
)

const shown = computed(() => sections.value.reduce((sum, section) => sum + section.shown, 0))

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
      </header>

      <div class="flex min-w-0 flex-col gap-4">
        <p v-if="!section.shown && section.total" class="py-4 text-sm text-text-muted">
          {{ t('programme.list.emptyFiltered') }}
        </p>

        <EventProgrammeSideDay
          v-for="side in section.sides"
          :key="side.track.slug"
          :track="side.track"
          :to="side.to"
          :timezone="props.timezone"
        />

        <EventProgrammeDayFrame
          v-for="entry in section.frames"
          :key="entry.frame.track.slug"
          :frame="entry.frame"
          :to="entry.to"
          :timezone="props.timezone"
        />

        <ol v-if="section.rows.length">
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
