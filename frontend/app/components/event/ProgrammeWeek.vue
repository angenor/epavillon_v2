<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'

/**
 * La semaine en colonnes : une par journée, les activités retenues par la
 * recherche empilées dans l'ordre. Les en-têtes de jour restent figés sous la
 * barre du site : aucun ancêtre ne doit donc être en `overflow: hidden`. Trop
 * étroit pour toutes les colonnes, le tableau défile à l'horizontale.
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
      label: format(day, { weekday: 'long', day: 'numeric', month: 'long' }),
      isToday: day === props.today,
      isPast: props.today !== null && day < props.today,
      count: t('programme.days.count', { count: kept.length }, kept.length),
      special: special ? tr(special.title) : '',
      cards: kept.map(card),
    }
  }),
)

const shown = computed(() => columns.value.reduce((sum, column) => sum + column.cards.length, 0))

const tint = (column: { isToday: boolean }, index: number) =>
  column.isToday ? 'bg-accent/6' : index % 2 ? 'bg-text/3' : ''

const root = useTemplateRef<HTMLElement>('root')
const head = useTemplateRef<HTMLElement>('head')
const body = useTemplateRef<HTMLElement>('body')

/** Les en-têtes restent figés au défilement vertical, donc hors du défileur horizontal : on les tient alignés. */
function sync(from: 'head' | 'body'): void {
  const [source, target] = from === 'head' ? [head.value, body.value] : [body.value, head.value]
  if (source && target && target.scrollLeft !== source.scrollLeft) target.scrollLeft = source.scrollLeft
}

/** Sur un écran étroit, la journée choisie arrive en premier à gauche. */
function reveal(): void {
  const index = props.days.indexOf(props.selected)
  const column = body.value?.querySelectorAll('section')[index]
  if (body.value && column) body.value.scrollLeft = column.offsetLeft
}

/** Une fois figée sous la barre, la rangée des jours se resserre : elle ne garde que le nécessaire. */
const sentinel = useTemplateRef<HTMLElement>('sentinel')
const stuck = ref(false)
const hasSpecial = computed(() => columns.value.some((column) => column.special))
let observer: IntersectionObserver | undefined

onMounted(() => {
  reveal()
  if (!sentinel.value) return
  const style = getComputedStyle(document.documentElement)
  const nav = Math.round(parseFloat(style.getPropertyValue('--nav-height')) * parseFloat(style.fontSize)) || 0
  observer = new IntersectionObserver(
    ([entry]) => {
      stuck.value = !!entry && !entry.isIntersecting && entry.boundingClientRect.top < nav
    },
    { rootMargin: `-${nav}px 0px 0px 0px` },
  )
  observer.observe(sentinel.value)
})
onBeforeUnmount(() => observer?.disconnect())
watch(() => [props.selected, props.days], () => nextTick(reveal))

async function next(): Promise<void> {
  emit('week', 1)
  await nextTick()
  const top = root.value?.getBoundingClientRect().top
  if (top === undefined) return
  const style = getComputedStyle(document.documentElement)
  const nav = parseFloat(style.getPropertyValue('--nav-height')) * parseFloat(style.fontSize)
  window.scrollTo({ top: top + window.scrollY - (Number.isFinite(nav) ? nav : 0), behavior: 'smooth' })
}
</script>

<template>
  <div ref="root">
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

    <div ref="sentinel" class="mt-7" aria-hidden="true" />
    <div class="sticky top-(--nav-height) z-20 bg-surface">
      <div ref="head" class="overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden" @scroll="sync('head')">
        <div class="grid grid-cols-[repeat(var(--week-columns),minmax(11rem,1fr))]" :style="{ '--week-columns': String(columns.length) }">
          <button
            v-for="(column, index) in columns"
            :key="column.date"
            type="button"
            class="group/day block min-w-0 cursor-pointer border-l border-border-subtle px-4 text-left first:border-l-0"
            :class="tint(column, index)"
            :aria-label="t('programme.week.openDay', { day: column.label })"
            :aria-current="column.isToday ? 'date' : undefined"
            @click="emit('day', column.date)"
          >
            <span
              class="block border-b-2 transition-[padding] duration-200"
              :class="[column.isToday ? 'border-accent' : 'border-text', stuck ? 'pt-2.5 pb-2' : 'py-3']"
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
              <span
                v-if="!stuck || hasSpecial"
                class="text-[11px] leading-4 font-bold tracking-caps text-accent uppercase"
                :class="stuck ? 'mt-1 line-clamp-1 h-4' : 'mt-2 line-clamp-2 h-8'"
              >
                {{ column.special }}
              </span>
            </span>
          </button>
        </div>
      </div>
    </div>

    <div ref="body" class="relative overflow-x-auto" @scroll="sync('body')">
      <div class="grid grid-cols-[repeat(var(--week-columns),minmax(11rem,1fr))]" :style="{ '--week-columns': String(columns.length) }">
        <section
          v-for="(column, index) in columns"
          :key="column.date"
          class="min-w-0 border-l border-border-subtle px-4 pb-3 first:border-l-0"
          :class="tint(column, index)"
          :aria-label="column.label"
        >
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

              <p class="mt-2.5 flex flex-wrap items-baseline gap-x-2">
                <span
                  class="text-[26px] leading-none font-light tabular-nums"
                  :class="item.current === 'live' ? 'text-live' : 'text-text'"
                >
                  {{ item.start }}
                </span>
                <span class="text-xs whitespace-nowrap text-text-muted tabular-nums">{{ item.until }}</span>
              </p>

              <NuxtLink
                :to="item.to"
                class="mt-1.5 line-clamp-4 text-[15px] leading-[1.3] text-pretty text-text no-underline after:absolute after:inset-0 group-hover:underline"
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

    <div v-if="props.hasNext" class="flex justify-end border-t border-border-subtle pt-5">
      <UiButton variant="secondary" icon-trailing="arrow-right" @click="next">{{ t('programme.week.next') }}</UiButton>
    </div>
  </div>
</template>
