<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { AttachedImage } from '~/types/media'
import type { ActiveIncident } from '~/types/live'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { ProgrammeRowData } from './ProgrammeRow.vue'
import type { PublicScheduleTrack } from '~/types/programme/track-page'
import type { ProgrammeTrackIndex, TrackBlock } from '~/utils/programme-tracks'

/**
 * Tout le programme en liste, une section par journée : le jour à gauche, figé
 * sur grand écran, les activités à droite (direction condensée, 01/11). Une
 * journée spéciale publiée et l'atelier y tiennent chacun en un bloc à leur
 * couleur (03/11) ; les autres activités du jour restent des lignes, à leur
 * heure, avant ou après le bloc. Au défilement, la journée dont la section passe sous ce qui est figé est
 * signalée (`reading`) : la page l'allume dans la bande. `scrollToDay` sert le
 * geste inverse.
 */

interface Props {
  days: IsoDate[]
  sessions: PublicScheduleRow[]
  matches: (session: PublicScheduleRow) => boolean
  /** Fils publiés : un bloc par journée spéciale et pour l'atelier. */
  tracks: ProgrammeTrackIndex
  matchesTrack: (track: PublicScheduleTrack) => boolean
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



const trackTo = (slug: string) => localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: slug } })

function row(session: PublicScheduleRow): ProgrammeRowData {
  const current = state(session)
  const faded = current === 'cancelled' || (current === 'past' && !props.editionPast)
  const page = link(props.editionSlug, session)
  return {
    id: session.id,
    notice: incidentFor(props.incidents, session),
    to: page,
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
    host: session.organization_hosts_pavilion ?? false,
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

type Run = { kind: 'block'; key: string; block: TrackBlock; to: string } | { kind: 'rows'; key: string; rows: ProgrammeRowData[] }

const sections = computed(() =>
  visibleDays.value.map((day) => {
    const own = props.sessions.filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
    const composed = composeDay(day, own, props.tracks, props.timezone, props.matches, props.matchesTrack)

    // Les lignes qui se suivent forment une liste ; un bloc la coupe.
    const runs: Run[] = []
    for (const item of composed.items) {
      if (item.kind === 'block') {
        runs.push({ kind: 'block', key: item.key, block: item.block, to: trackTo(item.block.track.slug) })
        continue
      }
      const last = runs.at(-1)
      if (last?.kind === 'rows') last.rows.push(row(item.session))
      else runs.push({ kind: 'rows', key: item.key, rows: [row(item.session)] })
    }

    // Toutes les activités publiées retenues, celles d'un fil et d'un moment comprises : le même compte que le bandeau.
    const activities = own.filter((session) => props.matches(session)).length
    const special = composed.blocks.find((block) => block.track.kind === 'special_day')?.track ?? null
    const side = composed.blocks.find((block) => block.track.kind !== 'special_day')?.track ?? null
    const eve = side ? eveLabel(day) : ''
    const count = eve || t('programme.days.count', { count: activities }, activities)
    // Le trait au-dessus du numéro : la couleur de la journée spéciale, à défaut celle de l'atelier.
    const marked = special ?? side
    return {
      day,
      number: format(day, { day: 'numeric' }),
      weekday: format(day, { weekday: 'long' }),
      date: format(day, { day: 'numeric', month: 'long', year: 'numeric' }),
      month: t('programme.days.monthCount', { month: format(day, { month: 'long' }), count }),
      style: marked ? trackDayStyle(marked.color_hex) : undefined,
      past: props.today !== null && day < props.today,
      isToday: day === props.today,
      runs,
      shown: composed.items.length,
      total: composed.total,
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

        <template v-for="run in section.runs" :key="run.key">
          <EventProgrammeTrackBlock
            v-if="run.kind === 'block'"
            :block="run.block"
            :to="run.to"
            :timezone="props.timezone"
            :faded="section.past && !props.editionPast"
            layout="row"
          />
          <ol v-else>
            <li v-for="entry in run.rows" :key="entry.id" class="border-b border-border-subtle last:border-b-0">
              <EventProgrammeRow :entry="entry" :edition-image="props.editionImage" />
            </li>
          </ol>
        </template>
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
