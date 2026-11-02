<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { AttachedImage } from '~/types/media'
import type { ActiveIncident } from '~/types/live'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { ProgrammeRowData } from './ProgrammeRow.vue'
import type { PublicScheduleTrack } from '~/types/programme/track-page'
import type { DayFrame, ProgrammeTrackIndex } from '~/utils/programme-tracks'

/**
 * Tout le programme en liste, une section par journée : le jour à gauche, figé
 * sur grand écran, les activités à droite (direction condensée, 01/11). Les
 * activités d'une journée spéciale publiée y sont réunies dans son cadre, les
 * autres suivent dessous ; le jour de l'atelier porte son seul encadré.
 * Au défilement, la journée dont la section passe sous ce qui est figé est
 * signalée (`reading`) : la page l'allume dans la bande. `scrollToDay` sert le
 * geste inverse.
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
  /** Hauteur de ce qui reste figé au-dessus de la liste (barre du site, bande des jours ou barre d'outils), en pixels. */
  stickyOffset: number
  today: IsoDate | null
  /** Tait les journées strictement antérieures à `today`. */
  hidePast: boolean
  /** Édition passée : ses activités gardent leurs couleurs, sinon plus rien ne donne envie de cliquer. */
  editionPast: boolean
  /** La photo de l'édition : la vignette des activités sans couverture. */
  editionImage?: AttachedImage | null
  /** Messages actifs de l'édition : celui d'une activité se pose sur sa carte. */
  incidents?: ActiveIncident[]
}

const props = withDefaults(defineProps<Props>(), { editionImage: null, incidents: () => [] })
const emit = defineEmits<{ reading: [date: IsoDate]; reset: [] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { state, link, duration, titleParts } = useProgrammeSession()
const { partsOf } = useJointSession()
const localePath = useLocalePath()

const at = (day: IsoDate) => new Date(`${day}T12:00:00Z`)
const format = (day: IsoDate, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(locale.value, { ...options, timeZone: 'UTC' }).format(at(day))



const trackTo = (slug: string, hash?: string) =>
  localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: slug }, hash })

function row(session: PublicScheduleRow, track: PublicScheduleTrack | null = null): ProgrammeRowData {
  const current = state(session)
  const faded = current === 'cancelled' || (current === 'past' && !props.editionPast)
  const page = link(props.editionSlug, session)
  return {
    id: session.id,
    notice: incidentFor(props.incidents, session),
    // Dans un cadre, la ligne mène au déroulé de la journée, à sa place ; la fiche y garde son lien.
    to: track ? trackTo(track.slug, `#${trackActivityAnchor(session.slug)}`) : page,
    start: time(session.starts_at, props.timezone),
    until: t('programme.list.untilShort', { end: time(session.ends_at, props.timezone), duration: duration(session) }),
    title: titleParts(tr(session.title)),
    acronym: session.organization_acronym,
    organization: session.organization_name,
    country: session.organization_country ? tr(session.organization_country) : '',
    cover: session.cover,
    logo: session.organization_logo ?? null,
    room: session.room_name ? tr(session.room_name) : null,
    joinTo: current === 'live' ? `${page}#direct` : null,
    current,
    faded,
    stateLabel: current === 'live' ? t('programme.list.live') : t(`session-card.state.${current}`),
    parts: partsOf(session),
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

/** L'atelier tient en un jour : son état se lit sur la date, sans l'heure. Nul avant l'horloge du navigateur. */
function sideState(day: IsoDate): 'upcoming' | 'ongoing' | 'past' | null {
  if (props.today === null) return null
  if (day < props.today) return 'past'
  return day === props.today ? 'ongoing' : 'upcoming'
}

const sections = computed(() =>
  visibleDays.value.map((day) => {
    const own = props.sessions.filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
    const kept = new Set(own.filter(props.matches).map((session) => session.id))

    const sides = props.tracks.sides
      .filter((side) => trackDayOf(side, props.timezone) === day)
      .filter((side) => props.matchesTrack(side) || own.some((session) => kept.has(session.id) && sideOf(session, props.tracks)?.slug === side.slug))
      .map((side) => ({ track: side, to: trackTo(side.slug), state: sideState(day) }))

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
    // Le trait au-dessus du numéro : la couleur de la première journée spéciale, le bleu nuit de l'atelier.
    const marked = frames[0]?.frame.track ?? sides[0]?.track ?? null
    return {
      day,
      number: format(day, { day: 'numeric' }),
      weekday: format(day, { weekday: 'long' }),
      date: format(day, { day: 'numeric', month: 'long', year: 'numeric' }),
      month: t('programme.days.monthCount', {
        month: format(day, { month: 'long' }),
        count: eve || t('programme.days.count', { count: activities }, activities),
      }),
      style: marked ? trackDayStyle(marked.color_hex) : undefined,
      past: props.today !== null && day < props.today,
      isToday: day === props.today,
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
  <div>
    <section
      v-for="section in sections"
      :id="anchor(section.day)"
      :key="section.day"
      class="grid gap-x-8 gap-y-4 border-b-2 border-text pt-9 pb-7 min-[960px]:grid-cols-[184px_minmax(0,1fr)]"
      :aria-labelledby="`${anchor(section.day)}-titre`"
    >
      <header
        class="flex flex-wrap items-baseline gap-x-3.5 gap-y-1 self-start min-[960px]:sticky min-[960px]:block"
        :style="[section.style, { top: `${props.stickyOffset + 16}px` }]"
      >
        <span
          v-if="section.style"
          class="mb-3.5 hidden h-1.5 w-12 bg-(--color-day-solid) min-[960px]:block"
          aria-hidden="true"
        />
        <p
          class="font-condensed text-[3.6rem] leading-[0.85] font-bold tabular-nums min-[960px]:text-[5.2rem]"
          :class="section.past ? 'text-text-muted' : section.style ? 'text-(--color-day)' : 'text-heading'"
          aria-hidden="true"
        >
          {{ section.number }}
        </p>
        <h3 :id="`${anchor(section.day)}-titre`" class="heading-condensed text-2xl text-heading min-[960px]:mt-2.5">
          {{ section.weekday }}<span class="sr-only"> {{ section.date }}</span>
        </h3>
        <p class="text-sm text-text-muted min-[960px]:mt-0.5">{{ section.month }}</p>
        <UiStatusBadge
          v-if="section.isToday"
          state="ongoing"
          size="sm"
          :label="t('programme.days.today')"
          class="self-center min-[960px]:mt-3"
        />
      </header>

      <div class="flex min-w-0 flex-col gap-6">
        <p v-if="!section.shown && section.total" class="py-4 text-sm text-text-muted">
          {{ t('programme.list.emptyFiltered') }}
        </p>

        <EventProgrammeSideDay
          v-for="side in section.sides"
          :key="side.track.slug"
          :track="side.track"
          :to="side.to"
          :state="side.state"
          :timezone="props.timezone"
        />

        <EventProgrammeDayFrame
          v-for="entry in section.frames"
          :key="entry.frame.track.slug"
          :frame="entry.frame"
          :to="entry.to"
          :timezone="props.timezone"
          :edition-image="props.editionImage"
        />

        <div v-if="section.rows.length">
          <p
            v-if="section.frames.length"
            class="font-condensed text-[1.05rem] font-semibold tracking-[0.05em] text-text-muted uppercase"
          >
            {{ t('programme.tracks.others') }}
          </p>
          <ol>
            <li v-for="entry in section.rows" :key="entry.id" class="border-b border-border-subtle last:border-b-0">
              <EventProgrammeRow :entry="entry" :edition-image="props.editionImage" />
            </li>
          </ol>
        </div>
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
