<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'

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
}

const props = defineProps<Props>()
const emit = defineEmits<{ reading: [date: IsoDate]; reset: [] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { state, link, duration, titleParts } = useProgrammeSession()

const at = (day: IsoDate) => new Date(`${day}T12:00:00Z`)
const format = (day: IsoDate, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(locale.value, { ...options, timeZone: 'UTC' }).format(at(day))



function row(session: PublicScheduleRow) {
  const current = state(session)
  const faded = current === 'past' || current === 'cancelled'
  return {
    id: session.id,
    to: link(props.editionSlug, session),
    start: time(session.starts_at, props.timezone),
    end: time(session.ends_at, props.timezone),
    duration: duration(session),
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
      specials: [...specials.values()].map((title) => t('programme.days.special', { title })),
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
          <p v-for="special in section.specials" :key="special">{{ special }}</p>
        </div>
      </header>

      <div>
        <p v-if="!section.rows.length && section.total" class="py-4 text-sm text-text-muted">
          {{ t('programme.list.emptyFiltered') }}
        </p>

        <ol>
          <li v-for="(entry, index) in section.rows" :key="entry.id">
            <article
              class="group relative -ml-3 grid grid-cols-[7rem_minmax(0,1fr)] gap-x-4 gap-y-1.5 rounded-md px-3 py-4 lg:grid-cols-[112px_208px_minmax(0,1fr)_172px_24px] lg:items-center lg:gap-x-7 lg:gap-y-0 lg:py-4.5"
              :class="entry.current === 'live' ? 'bg-live-surface' : index % 2 ? 'bg-text/3' : ''"
            >
              <p class="col-start-2 row-start-1 flex flex-wrap items-baseline gap-x-2 self-start lg:col-start-1 lg:block lg:pt-1">
                <span
                  class="block text-xl leading-none font-light tabular-nums lg:text-[38px]"
                  :class="entry.faded ? 'text-text-subtle' : entry.current === 'live' ? 'text-live' : 'text-text'"
                >
                  {{ entry.start }}
                </span>
                <span class="block text-[13px] text-text-muted tabular-nums lg:mt-1.5">
                  {{ t('programme.list.until', { end: entry.end, duration: entry.duration }) }}
                </span>
              </p>

              <div
                class="relative col-start-1 row-span-3 row-start-1 aspect-video self-start overflow-hidden rounded-lg bg-surface-inverse lg:col-start-2 lg:row-span-1 lg:w-52 lg:self-center"
              >
                <UiImage
                  v-if="entry.cover"
                  :image="entry.cover"
                  ratio="auto"
                  frame-class="size-full"
                  class="absolute inset-0 transition duration-200 motion-safe:group-hover:scale-[1.03]"
                  :class="{ grayscale: entry.faded }"
                  sizes="(min-width: 1024px) 208px, 7rem"
                />
                <span
                  v-else
                  class="absolute inset-0 flex items-center justify-center text-sm font-bold text-text-on-inverse/20 lg:text-[22px]"
                  aria-hidden="true"
                >
                  {{ entry.acronym }}
                </span>
              </div>

              <div class="col-start-2 min-w-0 lg:col-start-3 lg:row-start-1">
                <h4
                  class="line-clamp-3 font-sans text-base leading-[1.25] text-pretty sm:text-lg lg:text-[21px]"
                  :class="[entry.faded ? 'text-text-muted' : 'text-text', { 'line-through': entry.current === 'cancelled' }]"
                >
                  <!-- Lien couvrant : toute la ligne mène à l'activité, sans second lien à tabuler. -->
                  <NuxtLink :to="entry.to" class="font-normal text-inherit no-underline after:absolute after:inset-0 hover:underline">
                    <b class="font-bold">{{ entry.title.lead }}</b>{{ entry.title.rest }}
                  </NuxtLink>
                </h4>
                <p v-if="entry.organization || entry.acronym" class="mt-1.5 text-sm text-text-muted">
                  <b v-if="entry.acronym" class="font-bold text-text">{{ entry.acronym }}</b>
                  <template v-if="entry.acronym && entry.organization"> — </template>{{ entry.organization }}<template v-if="entry.country"> · {{ entry.country }}</template>
                </p>
              </div>

              <div class="col-start-2 justify-self-start lg:col-start-4 lg:row-start-1">
                <UiStatusBadge :state="entry.current" size="sm" :label="entry.stateLabel" />
              </div>

              <span class="hidden text-accent lg:col-start-5 lg:row-start-1 lg:block" aria-hidden="true">
                <UiIcon
                  name="arrow-right"
                  size="22px"
                  class="transition-transform duration-200 motion-safe:group-hover:translate-x-1"
                />
              </span>
            </article>
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
