<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { AttachedImage } from '~/types/media'
import type { ActiveIncident } from '~/types/live'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'
import type { PublicScheduleTrack } from '~/types/programme/track-page'
import type { ComposedDay, ProgrammeTrackIndex } from '~/utils/programme-tracks'

/**
 * Le calendrier (maquette « direction condensée », semaine.html, validée le
 * 01/11) : tous les jours du programme côte à côte, sur toute la largeur de la
 * page, dans une rangée qui défile à l'horizontale. Les en-têtes sont une
 * rangée à part, collée sous la barre et tenue alignée sur les colonnes : une
 * rangée collée dans un bloc qui défile en largeur ne colle plus à la page.
 * Aucun ancêtre ne doit donc être en `overflow: hidden`.
 *
 * Un trait épais ouvre chaque semaine. Le premier dimanche de la conférence,
 * sans activité, est un jour de repos hachuré (`restDayOf`). Une journée
 * spéciale et l'atelier tiennent chacun en un bloc à leur couleur, à la place
 * de leurs activités (03/11) ; l'en-tête de la colonne dit « Journée spéciale ».
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

const trackTo = (slug: string) => localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: slug } })

const at = (day: IsoDate) => new Date(`${day}T12:00:00Z`)
const format = (day: IsoDate, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(intlLocale.value, { ...options, timeZone: 'UTC' }).format(at(day))

const stateLabel = (current: ProgrammeSessionState) =>
  current === 'live' ? t('programme.week.live') : t(`session-card.state.${current}`)

const allDays = computed(() => calendarDays(props.days, props.restDay))
const breaks = computed(() => weekBreaks(allDays.value))

function card(session: PublicScheduleRow, weekday: string) {
  const current = state(session)
  const page = link(props.editionSlug, session)
  const parts = partsOf(session)
  return {
    id: session.id,
    parts,
    slides: parts.length ? slidesOf(parts, session.cover, session.id) : [],
    jointNames: namesOf(parts),
    to: page,
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
    notice: incidentFor(props.incidents, session),
    host: session.organization_hosts_pavilion ?? false,
  }
}

/** Le bandeau d'en-tête d'un jour qui porte un fil : son titre (journée) ou « Atelier préparatoire ». */
function tagOf(day: ComposedDay) {
  const special = day.blocks.find((block) => block.track.kind === 'special_day')?.track
  const track = special ?? day.blocks[0]?.track
  if (!track) return null
  return {
    text: special ? tr(special.title) : t('programme.tracks.side.kicker'),
    icon: trackIcon(track.icon_code),
    to: trackTo(track.slug),
    isSide: !special,
    style: trackDayStyle(track.color_hex),
  }
}

const columns = computed(() =>
  allDays.value.map((day, index) => {
    const weekday = format(day, { weekday: 'long' })
    const isRest = day === props.restDay
    const isPast = !props.editionPast && props.today !== null && day < props.today
    const own = props.sessions.filter((session) => dayKeyInZone(session.starts_at, props.timezone) === day)
    const composed = composeDay(day, own, props.tracks, props.timezone, props.matches, props.matchesTrack)
    const tag = isRest ? null : tagOf(composed)
    // Toutes les activités publiées retenues, celles d'un fil et d'un moment comprises : le même compte que le bandeau.
    const activities = own.filter((session) => props.matches(session)).length
    // Une colonne qui ne porte que des blocs les garde en vue au défilement.
    const pinned = !isRest && composed.items.length > 0 && composed.items.every((item) => item.kind === 'block')
    // Le bloc qui ouvre la colonne pend à la barre de l'en-tête, qu'il reste en vue ou non.
    const hangs = !isRest && composed.items[0]?.kind === 'block'
    return {
      date: day,
      number: format(day, { day: 'numeric' }),
      weekday,
      label: format(day, { weekday: 'long', day: 'numeric', month: 'long' }),
      isToday: day === props.today,
      isPast,
      isRest,
      isAlt: index % 2 === 1,
      weekBreak: breaks.value.has(index),
      count: isRest ? t('programme.week.rest.closed') : t('programme.week.count', { count: activities }, activities),
      tag,
      pinned,
      hangs,
      items: composed.items.map((item) =>
        item.kind === 'block'
          ? { kind: 'block' as const, key: item.key, block: item.block, to: trackTo(item.block.track.slug) }
          : { kind: 'session' as const, key: item.key, card: card(item.session, weekday) },
      ),
      activities,
    }
  }),
)

const shown = computed(() => columns.value.reduce((sum, column) => sum + column.activities, 0))
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

/**
 * Le bloc d'une colonne qui n'a rien d'autre (journée, atelier, jour de repos)
 * suit le défilement sous les en-têtes, jusqu'au bas de sa colonne, comme dans
 * la v1. Pas de `position: sticky` : dans un défileur horizontal, il ne colle
 * plus à la page.
 */
let pinFrame = 0
function pin(): void {
  pinFrame = 0
  const root = body.value
  if (!root) return
  const bar = sticky.value?.getBoundingClientRect().bottom ?? 0
  // Une colonne qui a cessé d'être seule (recherche) rend sa carte à sa place.
  for (const card of root.querySelectorAll<HTMLElement>('.cal-body-col:not([data-pinned]) > [data-shift]')) {
    delete card.dataset.shift
    card.style.transform = ''
  }
  for (const column of root.querySelectorAll<HTMLElement>('[data-pinned]')) {
    const card = column.firstElementChild as HTMLElement | null
    if (!card) continue
    // Le bloc d'une journée pend à la barre de l'en-tête ; la note du jour de repos garde son retrait.
    const top = bar + (card.classList.contains('rest-note') ? 12 : 0)
    const shift = Number(card.dataset.shift ?? 0)
    const natural = card.getBoundingClientRect().top - shift
    const room = column.getBoundingClientRect().bottom - (natural + card.offsetHeight) - 16
    const next = Math.round(Math.max(0, Math.min(top - natural, room)))
    if (next === shift) continue
    card.dataset.shift = String(next)
    card.style.transform = next ? `translateY(${next}px)` : ''
  }
}
function schedulePin(): void {
  if (!pinFrame) pinFrame = requestAnimationFrame(pin)
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
  window.addEventListener('scroll', schedulePin, { passive: true })
  window.addEventListener('resize', bounds)
  window.addEventListener('resize', schedulePin)
  schedulePin()
  sticky.value?.addEventListener('wheel', onWheel, { passive: false })
})
onBeforeUnmount(() => {
  window.removeEventListener('scroll', measureStuck)
  window.removeEventListener('scroll', schedulePin)
  window.removeEventListener('resize', schedulePin)
  if (pinFrame) cancelAnimationFrame(pinFrame)
  window.removeEventListener('resize', bounds)
  sticky.value?.removeEventListener('wheel', onWheel)
})
watch(() => [props.selected, allDays.value.join()], () => nextTick(() => show(props.selected)))
watch(columns, () => nextTick(schedulePin))
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
            :class="{ 'week-start': column.weekBreak, 'is-tinted': column.tag, 'is-side': column.tag?.isSide, 'is-alt': column.isAlt, 'is-rest': column.isRest, 'is-past': column.isPast }"
            :style="column.tag?.style"
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
          :class="{ 'week-start': column.weekBreak, 'is-tinted': column.tag, 'is-alt': column.isAlt, 'is-rest': column.isRest }"
          :data-pinned="column.pinned || column.isRest ? '' : undefined"
          :data-hangs="column.hangs ? '' : undefined"
          :aria-label="column.label"
        >
          <p v-if="column.isRest" class="rest-note">
            {{ t('programme.week.rest.label') }}
            <span>{{ t('programme.week.rest.note') }}</span>
          </p>

          <ol v-else-if="column.items.length" class="wk-list">
            <template v-for="entry in column.items" :key="entry.key">
              <li v-if="entry.kind === 'block'">
                <EventProgrammeTrackBlock :block="entry.block" :to="entry.to" :timezone="props.timezone" :faded="column.isPast" />
              </li>
              <li
                v-else
                class="wk-card group"
                :class="{ 'is-live': entry.card.current === 'live', 'is-faded': entry.card.faded }"
              >
                <div class="wk-media" :class="{ 'wk-media-tile': !entry.card.cover && !entry.card.slides.length }">
                  <UiSlideshow
                    v-if="entry.card.slides.length"
                    :slides="entry.card.slides"
                    :interval="5000"
                    :image-class="['transition duration-200 motion-safe:group-hover:scale-[1.03]', { grayscale: entry.card.faded }]"
                    sizes="(min-width: 960px) 216px, 104px"
                  />
                  <UiImage
                    v-else-if="entry.card.cover"
                    :image="entry.card.cover"
                    ratio="auto"
                    frame-class="size-full"
                    class="absolute inset-0 transition duration-200 motion-safe:group-hover:scale-[1.03]"
                    :class="{ grayscale: entry.card.faded }"
                    sizes="(min-width: 960px) 216px, 104px"
                  />
                  <ActivityCoverFallback
                    v-else
                    :image="props.editionImage"
                    :label="entry.card.initials"
                    text-class="text-[1.6rem]"
                    :faded="entry.card.faded"
                    :image-class="['transition duration-200 motion-safe:group-hover:scale-[1.03]']"
                    sizes="(min-width: 960px) 216px, 104px"
                  >
                    <span aria-hidden="true">{{ entry.card.initials }}</span>
                  </ActivityCoverFallback>
                </div>
                <div class="wk-body">
                  <p class="wk-time">
                    <b>{{ entry.card.start }}</b>
                    <span><small>{{ entry.card.weekday }}</small>{{ entry.card.until }}</span>
                    <EventHostMark v-if="entry.card.host" class="ms-auto self-center" />
                  </p>
                  <h4 class="wk-title" :class="{ 'line-through': entry.card.current === 'cancelled' }">
                    <NuxtLink :to="entry.card.to" class="wk-link">{{ entry.card.title }}</NuxtLink>
                  </h4>
                  <ActivityIncidentNotice v-if="entry.card.notice" :incident="entry.card.notice" variant="flag" class="mt-1.5" />
                  <template v-if="entry.card.parts.length">
                    <p class="mt-2 flex items-center gap-2.5 text-[0.8rem]">
                      <span class="flex shrink-0">
                        <OrganizationLogoMark
                          v-for="(part, index) in entry.card.parts"
                          :key="part.id"
                          :name="part.name"
                          :acronym="part.acronym"
                          :logo="part.logo"
                          size="sm"
                          :class="{ '-ml-3 ring-2 ring-surface-raised': index > 0 }"
                        />
                      </span>
                      <b class="min-w-0 truncate font-bold text-text">{{ entry.card.jointNames }}</b>
                    </p>
                    <p class="wk-joint">
                      <UiIcon name="users" size="1.05em" :stroke-width="2" />{{ t('programme.joint.tag') }}
                    </p>
                  </template>
                  <p v-else-if="entry.card.organization" class="mt-2 flex items-center gap-2.5 text-[0.8rem]">
                    <OrganizationLogoMark :name="entry.card.organization" :acronym="entry.card.acronym" :logo="entry.card.logo" size="sm" />
                    <b class="min-w-0 truncate font-bold text-text">{{ entry.card.shortName }}</b>
                  </p>
                  <div class="wk-foot">
                    <UiStatusBadge :state="entry.card.current" size="sm" :label="entry.card.stateLabel" />
                    <span v-if="entry.card.visited" class="text-xs text-accent">{{ t('programme.week.visited') }}</span>
                    <UiButton v-if="entry.card.current === 'live'" variant="live" :to="entry.card.joinTo" icon="video" class="relative z-10">
                      {{ t('programme.week.join') }}
                    </UiButton>
                  </div>
                </div>
              </li>
            </template>
          </ol>

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

/* Le jour de repos : hachuré sur toute la hauteur, et dit. */
.cal-body-col.is-rest {
  background: var(--cal-hatch);
}

/* Le fond du corps s'arrête aux marges de la colonne, comme celui de l'en-tête : les bords tombent d'aplomb. */
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

/* Colonne d'une journée ou de l'atelier : sa carte pend à la barre de l'en-tête, qui la nomme déjà. */
.cal-body-col[data-hangs] .wk-list {
  padding-top: 0;
}

.cal-body-col[data-hangs] .wk-list > li:first-child :deep(.tb) {
  border-top: 0;
  border-top-left-radius: 0;
  border-top-right-radius: 0;
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
