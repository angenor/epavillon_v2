<script setup lang="ts">
import type { PublicTrackPage } from '~/types/programme/track-page'
import type { TrackDayPhase } from '~/composables/useTrackDay'

/**
 * Le bandeau d'une journée spéciale : celui d'une activité (`UiMediaBanner`),
 * avec le bloc date à la couleur de la journée. L'atelier, sans photo, garde
 * l'aplat bleu nuit et dit qu'il se tient ailleurs qu'au pavillon.
 */

interface Props {
  page: PublicTrackPage
  phase: TrackDayPhase
  backTo: string
  /** « Veille de la CdP31 » : le fil se tient avant l'ouverture de l'édition. */
  eve: string
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { intlLocale } = useDateTime()
const { titleParts } = useProgrammeSession()

const timezone = computed(() => props.page.edition.timezone)
const edition = computed(() => props.page.edition.acronym ?? tr(props.page.edition.title))
const isDay = computed(() => props.page.track.kind === 'special_day')

/** Le premier jour : heure de la première ligne, à défaut la date annoncée. */
const firstInstant = computed(() => {
  const start = props.page.opens_at ?? props.page.starts_at
  return start ? Date.parse(start) : props.page.track.starts_on ? Date.parse(`${props.page.track.starts_on}T12:00:00Z`) : null
})
const zoneOfDate = computed(() => (props.page.opens_at ?? props.page.starts_at ? timezone.value : 'UTC'))
const part = (options: Intl.DateTimeFormatOptions) =>
  firstInstant.value === null
    ? ''
    : new Intl.DateTimeFormat(locale.value, { ...options, timeZone: zoneOfDate.value }).format(firstInstant.value)

const block = computed(() => ({ weekday: part({ weekday: 'short' }), day: part({ day: 'numeric' }), month: part({ month: 'short' }) }))
const fullDate = computed(() =>
  firstInstant.value === null ? '' : formatDate(firstInstant.value, { locale: intlLocale.value, timeZone: zoneOfDate.value, dateStyle: 'full' }),
)

const title = computed(() => titleParts(tr(props.page.track.title)))
const trail = computed(() => (props.eve ? t('track.trail.before') : t(`track.trail.${props.page.track.kind}`)))

const place = computed(() => {
  const venue = props.page.venue
  if (!venue) return props.page.edition.city ?? ''
  const area = venue.area ? tr(venue.area).toLocaleLowerCase(locale.value) : ''
  return [tr(venue.name), props.page.is_off_site ? '' : area, props.page.edition.city ?? ''].filter(Boolean).join(', ')
})

const stateTone = computed(() => (props.phase === 'during' ? 'ongoing' : 'neutral'))
const stateDot = computed(() => (props.phase === 'before' ? 'bg-accent-on-inverse' : props.phase === 'during' ? 'bg-text-on-warning-inverse' : 'bg-text-on-inverse-muted'))
const stateLabel = computed(() => t(`track.state.${isDay.value ? 'day' : 'other'}.${props.phase}`))
</script>

<template>
  <UiMediaBanner :image="props.page.cover" :back-to="props.backTo" :back-label="t('track.back', { edition })" :trail="trail">
    <div class="mt-8 flex flex-wrap gap-2 sm:mt-10">
      <UiBannerPill :tone="stateTone" :dot="stateDot">{{ stateLabel }}</UiBannerPill>
      <UiBannerPill v-if="props.eve">{{ props.eve }}</UiBannerPill>
      <UiBannerPill v-else>{{ t(`track.kind.${props.page.track.kind}`) }}</UiBannerPill>
      <UiBannerPill v-if="props.page.track.tagline">{{ tr(props.page.track.tagline) }}</UiBannerPill>
      <UiBannerPill v-if="props.page.is_off_site">{{ t('track.offSite') }}</UiBannerPill>
    </div>

    <div class="mt-6 grid gap-4 sm:grid-cols-[auto_minmax(0,1fr)] sm:items-start sm:gap-5">
      <p
        v-if="block.day"
        class="flex items-baseline gap-2 justify-self-start rounded-lg bg-(--color-day-mark) px-3 py-2 text-sm font-bold text-(--color-day-on-mark) uppercase sm:w-22 sm:flex-col sm:items-center sm:gap-0 sm:px-0 sm:py-3 sm:text-xs sm:leading-[1.1]"
        :style="{ letterSpacing: 'var(--tracking-caps)' }"
        aria-hidden="true"
      >
        <span>{{ block.weekday }}</span>
        <b class="text-xl leading-none font-bold tracking-normal tabular-nums sm:text-[2.75rem] sm:font-light">{{ block.day }}</b>
        <span>{{ block.month }}</span>
      </p>
      <div class="min-w-0">
        <h1 class="max-w-240 font-sans text-[2rem] leading-[1.08] font-light text-balance text-text-on-inverse sm:text-[2.625rem] lg:text-[3.25rem]">
          <b class="font-bold">{{ title.lead }}</b>{{ title.rest }}
        </h1>
        <p v-if="props.page.track.subtitle" class="mt-4 max-w-3xl text-lg leading-snug sm:text-xl">
          {{ tr(props.page.track.subtitle) }}
        </p>
      </div>
    </div>

    <ul class="mt-6 flex flex-wrap items-center gap-x-8 gap-y-3 text-base font-bold">
      <li v-if="place" class="inline-flex items-center gap-2.5">
        <UiIcon name="map-pin" size="1.125rem" :stroke-width="2" />
        {{ place }}
      </li>
      <li v-if="fullDate" class="inline-flex items-center gap-2.5">
        <UiIcon name="calendar" size="1.125rem" :stroke-width="2" />
        <span class="inline-block first-letter:uppercase">{{ fullDate }}</span>
      </li>
    </ul>
  </UiMediaBanner>
</template>
