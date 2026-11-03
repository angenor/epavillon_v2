<script setup lang="ts">
import type { AttachedImage } from '~/types/media'
import type { PublicTrackPage } from '~/types/programme/track-page'
import type { TrackDayPhase } from '~/composables/useTrackDay'

/**
 * Le bandeau d'une journée spéciale : celui d'une activité (`UiMediaBanner`),
 * avec le pavé de date à la couleur de la journée et le titre en condensé
 * (direction condensée). L'atelier, sans photo, garde l'aplat bleu nuit et dit
 * qu'il se tient ailleurs qu'au pavillon.
 */

interface Props {
  page: PublicTrackPage
  phase: TrackDayPhase
  backTo: string
  /** « Veille de la CdP31 » : le fil se tient avant l'ouverture de l'édition. */
  eve: string
  /** Une activité du fil est en direct. */
  live?: boolean
  /** Logo clair de l'édition, lu dans la liste publique des éditions. */
  editionLogo?: AttachedImage | null
}

const props = withDefaults(defineProps<Props>(), { live: false, editionLogo: null })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { intlLocale } = useDateTime()

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

const title = computed(() => tr(props.page.track.title))
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
  <!-- Sur téléphone, le billet ne remonte pas sur le bandeau : son bas se resserre. -->
  <UiMediaBanner
    :image="props.page.cover"
    :back-to="props.backTo"
    :back-label="t('track.back', { edition })"
    :trail="trail"
    :logo="props.editionLogo"
    :logo-alt="t('track.editionLogo', { edition })"
    class="max-lg:pb-9"
  >
    <div class="mt-4 flex flex-wrap gap-2">
      <UiBannerPill :tone="stateTone" :dot="stateDot">{{ stateLabel }}</UiBannerPill>
      <UiBannerPill v-if="props.eve">{{ props.eve }}</UiBannerPill>
      <UiBannerPill v-else>{{ t(`track.kind.${props.page.track.kind}`) }}</UiBannerPill>
      <UiBannerPill v-if="props.live" tone="live" dot="bg-text-on-inverse motion-safe:animate-pulse">{{ t('track.hero.live') }}</UiBannerPill>
      <UiBannerPill v-if="props.page.track.tagline">{{ tr(props.page.track.tagline) }}</UiBannerPill>
      <UiBannerPill v-if="props.page.is_off_site">{{ t('track.offSite') }}</UiBannerPill>
    </div>

    <div class="mt-5 flex flex-col items-start gap-3.5 sm:flex-row sm:gap-5">
      <p
        v-if="block.day"
        class="flex shrink-0 items-baseline gap-2 rounded-lg bg-(--color-day-mark) px-3 py-1.5 font-condensed font-bold text-(--color-day-on-mark) uppercase leading-none sm:w-19 sm:flex-col sm:items-center sm:gap-0 sm:px-0 sm:py-2"
        aria-hidden="true"
      >
        <span class="text-sm tracking-[0.06em]">{{ block.weekday }}</span>
        <b class="text-[1.6rem] font-bold tabular-nums sm:my-0.5 sm:text-[2.6rem]">{{ block.day }}</b>
        <span class="text-sm tracking-[0.06em]">{{ block.month }}</span>
      </p>
      <h1 class="heading-condensed max-w-[30ch] text-[clamp(2.2rem,4.4vw,3.5rem)] leading-none text-balance text-text-on-inverse">
        {{ title }}
      </h1>
    </div>

    <div :class="block.day ? 'sm:pl-24' : ''">
      <p v-if="props.page.track.subtitle" class="mt-3 max-w-[58ch] text-lg leading-snug text-text-on-inverse/90">
        {{ tr(props.page.track.subtitle) }}
      </p>
      <ul class="mt-5 flex flex-wrap items-center gap-x-6.5 gap-y-2.5 text-[0.95rem] text-text-on-inverse/90">
        <li v-if="place" class="inline-flex items-center gap-2">
          <UiIcon name="map-pin" size="1rem" :stroke-width="2" class="text-accent-on-inverse" />
          {{ place }}
        </li>
        <li v-if="fullDate" class="inline-flex items-center gap-2">
          <UiIcon name="calendar" size="1rem" :stroke-width="2" class="text-accent-on-inverse" />
          <span class="inline-block first-letter:uppercase">{{ fullDate }}</span>
        </li>
      </ul>
    </div>
  </UiMediaBanner>
</template>
