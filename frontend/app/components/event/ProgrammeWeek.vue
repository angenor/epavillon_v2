<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { AttachedImage } from '~/types/media'
import type { ActiveIncident } from '~/types/live'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'
import type { PublicScheduleTrack } from '~/types/programme/track-page'
import type { ProgrammeTrackIndex } from '~/utils/programme-tracks'

/**
 * Le calendrier (maquette « direction condensée », semaine.html, validée le
 * 01/11) : tous les jours du programme côte à côte, sur toute la largeur de la
 * page, dans une rangée qui défile à l'horizontale. Les en-têtes sont une
 * rangée à part, collée sous la barre et tenue alignée sur les colonnes : une
 * rangée collée dans un bloc qui défile en largeur ne colle plus à la page.
 * Aucun ancêtre ne doit donc être en `overflow: hidden`.
 *
 * Un trait épais ouvre chaque semaine. Le premier dimanche de la conférence,
 * sans activité, est un jour de repos hachuré (`restDayOf`). Le jour d'une
 * journée spéciale est teinté de sa couleur, en-tête compris, et ses activités
 * mènent à leur place dans le déroulé de la journée.
 */

interface Props {
  days: IsoDate[]
  restDay: IsoDate | null
  sessions: PublicScheduleRow[]
  matches: (session: PublicScheduleRow) => boolean
  tracks: ProgrammeTrackIndex
  matchesTrack: (track: PublicScheduleTrack) => boolean
  timezone: TimeZoneName
  selected: IsoDate
  today: IsoDate | null
  visitedId: string | null
  editionSlug: string
  /** Édition passée : ses activités gardent leurs couleurs, sinon plus rien ne donne envie de cliquer. */
  editionPast: boolean
  /** La photo de l'édition : la vignette des activités sans couverture. */
  editionImage?: AttachedImage | null
  /** Messages actifs de l'édition : celui d'une activité se pose sur sa carte. */
  incidents?: ActiveIncident[]
}

const props = withDefaults(defineProps<Props>(), { editionImage: null, incidents: () => [] })

const { t } = useI18n()
const { tr } = useI18nText()
const { time, intlLocale } = useDateTime()
const { state, link } = useProgrammeSession()
const { partsOf, slidesOf, namesOf } = useJointSession()
const localePath = useLocalePath()

const trackTo = (slug: string, hash?: string) =>
  localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: slug }, hash })

const at = (day: IsoDate) => new Date(`${day}T12:00:00Z`)
const format = (day: IsoDate, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(intlLocale.value, { ...options, timeZone: 'UTC' }).format(at(day))

const stateLabel = (current: ProgrammeSessionState) =>
  current === 'live' ? t('programme.week.live') : t(`session-card.state.${current}`)

const allDays = computed(() => calendarDays(props.days, props.restDay))
const breaks = computed(() => weekBreaks(allDays.value))

function card(session: PublicScheduleRow, weekday: string) {
  const current = state(session)
  const special = specialOf(session, props.tracks)
  const page = link(props.editionSlug, session)
  const parts = partsOf(session)
  return {
    id: session.id,
    parts,
    slides: parts.length ? slidesOf(parts, session.cover, session.id) : [],
    jointNames: namesOf(parts),
    to: special ? trackTo(special.slug, `#${trackActivityAnchor(session.slug)}`) : page,
    joinTo: `${page}#direct`,
    current,
    stateLabel: stateLabel(current),
    faded: current === 'cancelled' || current === 'postponed' || (current === 'past' && !props.editionPast),
    cover: session.cover,
    start: time(session.starts_at, props.timezone),
    weekday,
    until: t('programme.week.until', { end: time(session.ends_at, props.timezone) }),
    title: tr(session.title),
    organization: session.organization_name ?? session.organization_acronym,
    acronym: session.organization_acronym,
    shortName: scheduleOrganizationLabel(session),
    initials: (session.organization_acronym ?? initialsOf(session.organization_name ?? '')).slice(0, 8),
    logo: session.organization_logo ?? null,
    visited: session.id === props.visitedId,
    inDay: !!special,
    notice: incidentFor(props.incidents, session),
    host: session.organization_hosts_pavilion ?? false,
  }
}

// Comme dans la liste, le fond de la journée s'arrête à sa dernière activité : les suivantes reprennent le fond ordinaire.
function groupsOf(cards: ReturnType<typeof card>[]) {
  const first = cards.findIndex((item) => item.inDay)
  if (first < 0) return [{ key: 'all', band: false, cards }]
  const last = cards.findLastIndex((item) => item.inDay)
  return [
    { key: 'before', band: false, cards: cards.slice(0, first) },
    { key: 'band', band: true, cards: cards.slice(first, last + 1) },
    { key: 'after', band: false, cards: cards.slice(last + 1) },
  ].filter((group) => group.cards.length)
}

const columns = computed(() =>
  allDays.value.map((day, index) => {
    const weekday = format(day, { weekday: 'long' })
    const isRest = day === props.restDay
    const isPast = !props.editionPast && props.today !== null && day < props.today
    const own = props.sessions
      .filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
      .sort((a, b) => Date.parse(a.starts_at) - Date.parse(b.starts_at))
    const kept = own.filter((session) => props.matches(session) && !sideOf(session, props.tracks))
    const sides = props.tracks.sides
      .filter((side) => trackDayOf(side, props.timezone) === day)
      .filter((side) => props.matchesTrack(side) || own.some((session) => props.matches(session) && sideOf(session, props.tracks)?.slug === side.slug))
    const special = own.map((session) => specialOf(session, props.tracks)).find((track) => track !== null) ?? null
    const side = sides[0] ?? null
    const tag = special
      ? { text: tr(special.title), icon: trackIcon(special.icon_code), to: trackTo(special.slug) }
      : side
        ? { text: t('programme.tracks.side.kicker'), icon: trackIcon(side.icon_code), to: trackTo(side.slug) }
        : null
    return {
      date: day,
      number: format(day, { day: 'numeric' }),
      weekday,
      label: format(day, { weekday: 'long', day: 'numeric', month: 'long' }),
      isToday: day === props.today,
      isPast,
      isRest,
      isTinted: !!tag,
      isSide: !special && !!side,
      isAlt: index % 2 === 1,
      weekBreak: breaks.value.has(index),
      count: isRest ? t('programme.week.rest.closed') : t('programme.week.count', { count: kept.length + sides.length }, kept.length + sides.length),
      tag,
      dayStyle: special ? trackDayStyle(special.color_hex) : side ? trackDayStyle(side.color_hex) : undefined,
      sides: sides.map((item) => ({
        slug: item.slug,
        title: tr(item.title),
        to: trackTo(item.slug),
        icon: trackIcon(item.icon_code),
        start: item.starts_at ? time(item.starts_at, props.timezone) : '',
        until: item.ends_at ? t('programme.week.until', { end: time(item.ends_at, props.timezone) }) : '',
        venue: item.venue_name ? tr(item.venue_name) : '',
        weekday,
        past: isPast,
      })),
      groups: groupsOf(kept.map((session) => card(session, weekday))),
      hasCards: kept.length > 0,
    }
  }),
)

const shown = computed(() => columns.value.reduce((sum, column) => sum + column.groups.reduce((n, group) => n + group.cards.length, 0) + column.sides.length, 0))
const template = computed(() => allDays.value.map((day) => (day === props.restDay ? 'var(--cal-rest)' : 'var(--cal-col)')).join(' '))

const heading = computed(() => {
  const first = allDays.value[0]
  const last = allDays.value[allDays.value.length - 1]
  if (!first || !last) return { title: '', span: '' }
  const count = t('programme.week.count', { count: shown.value }, shown.value)
  const sameYear = first.slice(0, 4) === last.slice(0, 4)
  const title =
    first.slice(0, 7) === last.slice(0, 7)
      ? format(first, { month: 'long', year: 'numeric' })
      : t('programme.week.monthRange', {
          from: format(first, sameYear ? { month: 'long' } : { month: 'long', year: 'numeric' }),
          to: format(last, { month: 'long', year: 'numeric' }),
        })
  if (first === last) return { title, span: t('programme.week.single', { day: format(first, { weekday: 'long', day: 'numeric' }), count }) }
  const options: Intl.DateTimeFormatOptions =
    first.slice(0, 7) === last.slice(0, 7) ? { weekday: 'long', day: 'numeric' } : { weekday: 'long', day: 'numeric', month: 'long' }
  return { title, span: t('programme.week.span', { from: format(first, options), to: format(last, options), count }) }
})

const showToday = computed(() => {
  const now = props.today
  const first = allDays.value[0]
  const last = allDays.value[allDays.value.length - 1]
  return now !== null && !!first && !!last && now >= first && now <= last
})

const head = useTemplateRef<HTMLElement>('head')
const body = useTemplateRef<HTMLElement>('body')
const sticky = useTemplateRef<HTMLElement>('sticky')
const sentinel = useTemplateRef<HTMLElement>('sentinel')
const stuck = ref(false)
const canPrevious = ref(false)
const canNext = ref(false)

const isPhone = () => window.matchMedia('(max-width: 959.98px)').matches
const motion = (): ScrollBehavior => (window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth')
const cells = (): HTMLElement[] => Array.from(body.value?.firstElementChild?.children ?? []) as HTMLElement[]
const gutter = () => (body.value ? parseFloat(getComputedStyle(body.value).paddingLeft) || 0 : 0)
const leftOf = (cell: HTMLElement) => cell.offsetLeft - gutter()

/** Les en-têtes, collés au défilement vertical, vivent hors du défileur horizontal : on les tient alignés, dans les deux sens. */
let echo: { target: HTMLElement; left: number } | null = null

function sync(from: 'head' | 'body'): void {
  const [source, target] = from === 'head' ? [head.value, body.value] : [body.value, head.value]
  if (from === 'body') bounds()
  if (!source || !target) return
  // L'écho de notre propre alignement ne doit pas ramener l'autre rangée en arrière : il couperait un défilement animé.
  if (echo?.target === source && Math.abs(source.scrollLeft - echo.left) <= 1) return
  if (Math.abs(target.scrollLeft - source.scrollLeft) <= 1) return
  target.scrollLeft = source.scrollLeft
  echo = { target, left: target.scrollLeft }
}

function bounds(): void {
  const scroller = body.value
  if (!scroller) return
  canPrevious.value = scroller.scrollLeft > 4
  canNext.value = scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 4
}

/** Suivant : la première colonne coupée au bord droit passe à gauche. Précédent : un écran de colonnes entières en arrière. */
function step(direction: -1 | 1): void {
  const scroller = body.value
  if (!scroller) return
  const visible = scroller.clientWidth - 2 * gutter()
  const list = cells()
  if (direction === 1) {
    const edge = scroller.scrollLeft + visible
    const cell = list.find((item) => leftOf(item) + item.offsetWidth > edge + 1)
    if (cell) scroller.scrollTo({ left: leftOf(cell), behavior: motion() })
    return
  }
  const target = scroller.scrollLeft - visible
  const cell = list.find((item) => leftOf(item) >= target - 1)
  scroller.scrollTo({ left: cell ? leftOf(cell) : 0, behavior: motion() })
}

/** Sur grand écran, la semaine du jour depuis son lundi ; sur téléphone, le jour lui-même. */
function show(day: IsoDate, behavior: ScrollBehavior = 'auto'): void {
  const scroller = body.value
  if (!scroller) return
  const from = isPhone() ? day : weekStart(day)
  const index = allDays.value.findIndex((date) => date >= from)
  const cell = cells()[index === -1 ? 0 : index]
  if (cell) scroller.scrollTo({ left: leftOf(cell), behavior })
  sync('body')
}

function goToday(): void {
  if (props.today) show(props.today, motion())
}

/** Le repère précède la rangée : il ne bouge pas quand elle se réduit, donc pas d'aller-retour. */
function measureStuck(): void {
  if (!sentinel.value || !sticky.value) return
  const top = parseFloat(getComputedStyle(sticky.value).top) || 0
  stuck.value = sentinel.value.getBoundingClientRect().top < top - 1
}

/** Une molette ou un pavé tactile qui glisse en largeur sur les en-têtes fait défiler les colonnes. */
function onWheel(event: WheelEvent): void {
  if (!body.value || Math.abs(event.deltaX) <= Math.abs(event.deltaY)) return
  body.value.scrollLeft += event.deltaX
  event.preventDefault()
}

onMounted(() => {
  show(props.selected)
  measureStuck()
  window.addEventListener('scroll', measureStuck, { passive: true })
  window.addEventListener('resize', bounds)
  sticky.value?.addEventListener('wheel', onWheel, { passive: false })
})
onBeforeUnmount(() => {
  window.removeEventListener('scroll', measureStuck)
  window.removeEventListener('resize', bounds)
  sticky.value?.removeEventListener('wheel', onWheel)
})
watch(() => [props.selected, allDays.value.join()], () => nextTick(() => show(props.selected)))
</script>

<template>
  <div class="calendar full-bleed">
    <div class="cal-bar flex flex-wrap items-center justify-between gap-x-6 gap-y-3 pt-6 pb-4">
      <div class="min-w-0">
        <h3 class="heading-condensed text-[clamp(1.8rem,3.4vw,2.3rem)] text-heading">{{ heading.title }}</h3>
        <p class="mt-0.5 text-sm text-text-muted">{{ heading.span }}</p>
      </div>
      <UiButton v-if="showToday" variant="secondary" @click="goToday">{{ t('programme.week.today') }}</UiButton>
    </div>

    <div ref="sentinel" aria-hidden="true" />
    <div ref="sticky" class="cal-sticky" :class="{ 'is-stuck': stuck }">
      <button
        v-show="canPrevious"
        type="button"
        class="cal-arrow left-2"
        :aria-label="t('programme.week.previous')"
        @click="step(-1)"
      >
        <UiIcon name="chevron-left" size="22px" :stroke-width="2" />
      </button>
      <div ref="head" class="cal-head-scroll" @scroll="sync('head')">
        <div class="cal-grid" :style="{ gridTemplateColumns: template }">
          <div
            v-for="column in columns"
            :key="column.date"
            class="cal-col cal-head-col"
            :class="{ 'week-start': column.weekBreak, 'is-tinted': column.isTinted, 'is-side': column.isSide, 'is-alt': column.isAlt, 'is-rest': column.isRest, 'is-past': column.isPast }"
            :style="column.dayStyle"
          >
            <NuxtLink v-if="column.tag" :to="column.tag.to" class="rule-label wk-tag">
              <UiIcon v-if="column.tag.icon" :name="column.tag.icon" size="1.05em" :stroke-width="2" />
              <span class="truncate">{{ column.tag.text }}</span>
            </NuxtLink>
            <span v-else-if="column.isRest" class="rule-label wk-tag wk-tag-rest">{{ t('programme.week.rest.label') }}</span>
            <span v-else class="rule-label wk-tag invisible" aria-hidden="true">&nbsp;</span>
            <div class="wk-head">
              <b class="wk-number">{{ column.number }}</b>
              <div class="min-w-0">
                <p class="wk-weekday heading-condensed">{{ column.weekday }}</p>
                <p class="wk-count">{{ column.count }}</p>
              </div>
              <UiStatusBadge v-if="column.isToday" class="wk-today" state="ongoing" size="sm" :label="t('programme.week.today')" />
            </div>
          </div>
        </div>
      </div>
      <button
        v-show="canNext"
        type="button"
        class="cal-arrow right-2"
        :aria-label="t('programme.week.next')"
        @click="step(1)"
      >
        <UiIcon name="chevron-right" size="22px" :stroke-width="2" />
      </button>
    </div>

    <div ref="body" class="cal-body-scroll" @scroll="sync('body')">
      <div class="cal-grid" :style="{ gridTemplateColumns: template }">
        <section
          v-for="column in columns"
          :key="column.date"
          class="cal-col cal-body-col"
          :class="{ 'week-start': column.weekBreak, 'is-tinted': column.isTinted, 'is-side': column.isSide, 'is-alt': column.isAlt, 'is-rest': column.isRest }"
          :style="column.dayStyle"
          :aria-label="column.label"
        >
          <p v-if="column.isRest" class="rest-note">
            {{ t('programme.week.rest.label') }}
            <span>{{ t('programme.week.rest.note') }}</span>
          </p>

          <div v-else-if="column.hasCards || column.sides.length">
            <ol v-if="column.sides.length" class="wk-list">
              <li v-for="side in column.sides" :key="side.slug" class="wk-card group" :class="{ 'is-faded': side.past }">
                <div class="wk-media wk-media-tile">
                  <UiIcon v-if="side.icon" :name="side.icon" size="2rem" :stroke-width="1.6" />
                </div>
                <div class="wk-body">
                  <p v-if="side.start" class="wk-time">
                    <b>{{ side.start }}</b>
                    <span><small>{{ side.weekday }}</small>{{ side.until }}</span>
                  </p>
                  <h4 class="wk-title">
                    <NuxtLink :to="side.to" class="wk-link">{{ side.title }}</NuxtLink>
                  </h4>
                  <p v-if="side.venue" class="mt-2 text-[0.8rem] text-text-muted">{{ side.venue }}</p>
                  <div v-if="side.past" class="wk-foot">
                    <UiStatusBadge state="past" size="sm" :label="stateLabel('past')" />
                  </div>
                </div>
              </li>
            </ol>

            <ol v-for="group in column.groups" :key="group.key" class="wk-list" :class="{ 'wk-band': group.band }">
              <li
                v-for="item in group.cards"
                :key="item.id"
                class="wk-card group"
                :class="{ 'is-live': item.current === 'live', 'is-faded': item.faded }"
              >
                <div class="wk-media" :class="{ 'wk-media-tile': !item.cover && !item.slides.length }">
                  <UiSlideshow
                    v-if="item.slides.length"
                    :slides="item.slides"
                    :interval="5000"
                    :image-class="['transition duration-200 motion-safe:group-hover:scale-[1.03]', { grayscale: item.faded }]"
                    sizes="(min-width: 960px) 216px, 104px"
                  />
                  <UiImage
                    v-else-if="item.cover"
                    :image="item.cover"
                    ratio="auto"
                    frame-class="size-full"
                    class="absolute inset-0 transition duration-200 motion-safe:group-hover:scale-[1.03]"
                    :class="{ grayscale: item.faded }"
                    sizes="(min-width: 960px) 216px, 104px"
                  />
                  <ActivityCoverFallback
                    v-else
                    :image="props.editionImage"
                    :label="item.initials"
                    text-class="text-[1.6rem]"
                    :faded="item.faded"
                    :image-class="['transition duration-200 motion-safe:group-hover:scale-[1.03]']"
                    sizes="(min-width: 960px) 216px, 104px"
                  >
                    <span aria-hidden="true">{{ item.initials }}</span>
                  </ActivityCoverFallback>
                </div>
                <div class="wk-body">
                  <p class="wk-time">
                    <b>{{ item.start }}</b>
                    <span><small>{{ item.weekday }}</small>{{ item.until }}</span>
                    <EventHostMark v-if="item.host" class="ms-auto self-center" />
                  </p>
                  <h4 class="wk-title" :class="{ 'line-through': item.current === 'cancelled' }">
                    <NuxtLink :to="item.to" class="wk-link">{{ item.title }}</NuxtLink>
                  </h4>
                  <ActivityIncidentNotice v-if="item.notice" :incident="item.notice" variant="flag" class="mt-1.5" />
                  <template v-if="item.parts.length">
                    <p class="mt-2 flex items-center gap-2.5 text-[0.8rem]">
                      <span class="flex shrink-0">
                        <OrganizationLogoMark
                          v-for="(part, index) in item.parts"
                          :key="part.id"
                          :name="part.name"
                          :acronym="part.acronym"
                          :logo="part.logo"
                          size="sm"
                          :class="{ '-ml-3 ring-2 ring-surface-raised': index > 0 }"
                        />
                      </span>
                      <b class="min-w-0 truncate font-bold text-text">{{ item.jointNames }}</b>
                    </p>
                    <p class="wk-joint">
                      <UiIcon name="users" size="1.05em" :stroke-width="2" />{{ t('programme.joint.tag') }}
                    </p>
                  </template>
                  <p v-else-if="item.organization" class="mt-2 flex items-center gap-2.5 text-[0.8rem]">
                    <OrganizationLogoMark :name="item.organization" :acronym="item.acronym" :logo="item.logo" size="sm" />
                    <b class="min-w-0 truncate font-bold text-text">{{ item.shortName }}</b>
                  </p>
                  <div class="wk-foot">
                    <UiStatusBadge :state="item.current" size="sm" :label="item.stateLabel" />
                    <span v-if="item.visited" class="text-xs text-accent">{{ t('programme.week.visited') }}</span>
                    <UiButton v-if="item.current === 'live'" variant="live" :to="item.joinTo" icon="video" class="relative z-10">
                      {{ t('programme.week.join') }}
                    </UiButton>
                  </div>
                </div>
              </li>
            </ol>
          </div>

          <p v-else class="px-1 py-3.5 text-[13px] text-text-muted">{{ t('programme.week.emptyDay') }}</p>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.calendar {
  --cal-col: 228px;
  --cal-rest: 128px;
  --cal-gutter: 60px;
  --cal-hatch: repeating-linear-gradient(135deg, var(--color-border) 0 2px, transparent 2px 10px);
}

.cal-bar {
  padding-inline: var(--cal-gutter);
}

.cal-sticky {
  position: sticky;
  /* Une barre d'outils collée au-dessus peut déclarer sa hauteur ici. */
  top: var(--programme-sticky-top, var(--nav-height));
  z-index: 20;
  background: var(--color-surface);
  border-bottom: 2px solid var(--color-text);
}

.cal-grid {
  display: grid;
  width: max-content;
}

.cal-head-scroll {
  overflow: hidden;
  padding-inline: var(--cal-gutter);
}

.cal-body-scroll {
  position: relative;
  overflow-x: auto;
  padding: 0 var(--cal-gutter) 4rem;
  scrollbar-width: thin;
}

.cal-col {
  min-width: 0;
  padding-inline: 6px;
}

/* Un trait épais ouvre chaque semaine, en en-tête comme dans les colonnes. */
.cal-col.week-start {
  border-left: 3px solid var(--color-text);
  padding-left: 9px;
}

/* Un jour ordinaire sur deux, un gris à peine posé, de l'en-tête au bas de la colonne. */
.cal-col.is-alt:not(.is-tinted, .is-rest) {
  background: color-mix(in srgb, var(--color-text) 4.5%, transparent);
}

/* Une journée spéciale se détache plus que l'atelier, qui garde le fond léger. */
.cal-col.is-tinted:not(.is-side) {
  --cal-day-bg: color-mix(in srgb, var(--color-day-line) 24%, var(--color-surface-raised));
}

.cal-col.is-tinted.is-side {
  --cal-day-bg: var(--color-day-surface);
}

.cal-head-col {
  display: flex;
  flex-direction: column;
  padding-top: 10px;
}

.wk-tag {
  align-self: stretch;
  border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  padding: 6px 10px 5px;
  font-size: 1rem;
  white-space: nowrap;
  overflow: hidden;
  text-decoration: none;
  --rule-solid: var(--color-day-solid);
  --rule-on: var(--color-day-on-solid);
}

a.wk-tag:hover {
  text-decoration: underline;
}

.wk-tag-rest {
  --rule-solid: var(--color-neutral-surface);
  --rule-on: var(--color-neutral);
}

/* L'en-tête descend jusqu'au trait, même quand un voisin (« Aujourd'hui ») est plus haut. */
.wk-head {
  flex: 1;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  align-content: flex-start;
  gap: 4px 10px;
  padding: 8px 8px 10px;
}

.is-tinted .wk-head {
  background: var(--cal-day-bg);
}

.is-rest .wk-head {
  background: var(--cal-hatch);
}

.wk-number {
  font-family: var(--font-condensed);
  font-size: 2.6rem;
  font-weight: 700;
  line-height: 0.85;
  font-variant-numeric: tabular-nums;
  color: var(--color-heading);
}

.is-tinted .wk-number {
  color: var(--color-day);
}

.is-past .wk-number,
.is-rest .wk-number {
  color: var(--color-text-muted);
}

.wk-weekday {
  font-size: 1.1rem;
  color: var(--color-heading);
}

.wk-count {
  font-size: 0.76rem;
  color: var(--color-text-muted);
}

/* Collée en haut, la rangée se réduit à une ligne : étiquette fine, numéro et jour. */
.wk-tag,
.wk-head,
.wk-number,
.wk-weekday,
.cal-head-col {
  transition: font-size 180ms ease, padding 180ms ease;
}

.is-stuck .cal-head-col {
  padding-top: 4px;
}

.is-stuck .wk-tag {
  font-size: 0.78rem;
  padding: 3px 8px 2px;
}

.is-stuck .wk-head {
  flex-wrap: nowrap;
  padding: 3px 8px 5px;
}

.is-stuck .wk-number {
  font-size: 1.45rem;
}

.is-stuck .wk-weekday {
  font-size: 0.95rem;
}

.is-stuck .wk-count,
.is-stuck .wk-today {
  display: none;
}

@media (prefers-reduced-motion: reduce) {
  .wk-tag,
  .wk-head,
  .wk-number,
  .wk-weekday,
  .cal-head-col {
    transition: none;
  }
}

.cal-arrow {
  position: absolute;
  top: 50%;
  z-index: 2;
  display: grid;
  place-items: center;
  width: var(--target-min);
  height: var(--target-min);
  transform: translateY(-50%);
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-full);
  background: var(--color-surface-raised);
  color: var(--color-heading);
  box-shadow: var(--shadow-md);
  cursor: pointer;
}

.cal-arrow:hover {
  background: var(--color-surface-hover);
}

.cal-body-col.is-side,
.wk-band {
  background: var(--cal-day-bg);
  border-radius: 0 0 var(--radius-xl) var(--radius-xl);
}

.wk-band .wk-card {
  border-color: color-mix(in srgb, var(--color-day-line) 30%, transparent);
}

/* Le jour de repos : hachuré sur toute la hauteur, et dit. */
.cal-body-col.is-rest {
  background: var(--cal-hatch);
}

/* Le fond du corps s'arrête aux marges de la colonne, comme celui de l'en-tête : les bords tombent d'aplomb. */
.cal-body-col.is-side,
.cal-body-col.is-rest {
  background-clip: content-box;
}

.rest-note {
  margin-top: 24px;
  padding: 12px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-surface);
  text-align: center;
  font-family: var(--font-condensed);
  font-size: 1.05rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.rest-note span {
  display: block;
  margin-top: 6px;
  font-family: var(--font-text);
  font-size: 0.76rem;
  font-weight: 400;
  letter-spacing: 0;
  text-transform: none;
  color: var(--color-text-muted);
}

.wk-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 10px 0;
}

.wk-list + .wk-list {
  padding-top: 0;
}

.is-side .wk-list,
.wk-list.wk-band {
  padding-inline: 2px;
}

.wk-list.wk-band,
.wk-band + .wk-list {
  padding-top: 10px;
}

.wk-card {
  position: relative;
  overflow: hidden;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  background: var(--color-surface-raised);
  box-shadow: var(--shadow-sm);
  transition: box-shadow 300ms ease, transform 300ms ease;
}

.wk-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

@media (prefers-reduced-motion: reduce) {
  .wk-card,
  .wk-card:hover {
    transition: none;
    transform: none;
  }
}

.wk-card.is-faded {
  opacity: 0.72;
}

.wk-card.is-live {
  border: 2px solid var(--color-live);
}

.wk-media {
  position: relative;
  overflow: hidden;
  aspect-ratio: 16 / 9;
  background: var(--color-surface-sunken);
}

.wk-media-tile {
  display: grid;
  place-items: center;
  font-family: var(--font-condensed);
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--color-text-muted);
}

:is(.is-side, .wk-band) .wk-media-tile {
  background: var(--color-day-surface);
  color: var(--color-day);
}

.wk-body {
  padding: 10px 12px 12px;
}

.wk-time {
  display: flex;
  align-items: flex-end;
  gap: 8px;
}

.wk-time b {
  font-family: var(--font-condensed);
  font-size: 1.7rem;
  font-weight: 700;
  line-height: 0.9;
  font-variant-numeric: tabular-nums;
  color: var(--color-heading);
}

:is(.is-side, .wk-band) .wk-time b {
  color: var(--color-day);
}

.is-live .wk-time b {
  color: var(--color-live);
}

/* Le jour en tout petit au-dessus de l'heure de fin : on le garde en tête en faisant défiler. */
.wk-time span {
  display: flex;
  flex-direction: column;
  font-size: 0.76rem;
  line-height: 1.15;
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
}

.wk-time small {
  font-size: 0.62rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-text-secondary);
}

.wk-title {
  margin-top: 8px;
  font-family: var(--font-condensed);
  font-size: 1.08rem;
  font-weight: 600;
  line-height: 1.15;
  letter-spacing: 0.005em;
  color: var(--color-heading);
  text-wrap: pretty;
}

/* Lien couvrant : toute la carte mène à l'activité, sans second lien à tabuler. */
.wk-link {
  color: inherit;
  text-decoration: none;
}

.wk-link::after {
  content: "";
  position: absolute;
  inset: 0;
}

.wk-card:hover .wk-link {
  color: var(--color-accent);
  text-decoration: underline;
}

.wk-joint {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-accent);
}

:is(.is-side, .wk-band) .wk-joint {
  color: var(--color-day);
}

.wk-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}

/* Téléphone : un jour par écran ; la rangée des en-têtes reste collée et se fait
   glisser comme les colonnes, les deux se suivent dans les deux sens. */
@media (max-width: 959.98px) {
  .calendar {
    --cal-col: 86vw;
    --cal-rest: 60vw;
    --cal-gutter: 16px;
  }

  .cal-arrow {
    display: none;
  }

  .cal-head-scroll,
  .cal-body-scroll {
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    scroll-padding-inline: var(--cal-gutter);
    scrollbar-width: none;
  }

  .cal-head-scroll::-webkit-scrollbar,
  .cal-body-scroll::-webkit-scrollbar {
    display: none;
  }

  .cal-col {
    scroll-snap-align: start;
  }

  .wk-card {
    display: grid;
    grid-template-columns: 104px minmax(0, 1fr);
  }

  .wk-media {
    aspect-ratio: auto;
    min-height: 100%;
  }
}
</style>
