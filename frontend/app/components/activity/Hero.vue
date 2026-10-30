<script setup lang="ts">
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'

interface Props {
  session: PublicScheduleRow
  edition: PublicEditionRow
  state: ProgrammeSessionState
  backTo: string
  trail: string
  cancelledReason?: string | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { intlLocale } = useDateTime()
const { titleParts, showFormat } = useProgrammeSession()
const { image } = useEditionSummary(() => props.edition)
const localePath = useLocalePath()

const title = computed(() => tr(props.session.title))
const parts = computed(() => titleParts(title.value))
const titleSize = computed(() =>
  title.value.length > 90
    ? 'text-[1.875rem] sm:text-[2.25rem] lg:text-[2.625rem]'
    : 'text-[2rem] sm:text-[2.625rem] lg:text-[3.25rem]',
)

const stateLabel = computed(() =>
  props.state === 'live' ? t('activity.state.live') : t(`session-card.state.${props.state}`),
)
const stateDot: Record<ProgrammeSessionState, string> = {
  live: 'bg-live',
  upcoming: 'bg-accent-on-inverse',
  ongoing: 'bg-warning-solid',
  postponed: 'bg-postponed-border',
  past: 'bg-text-on-inverse-muted',
  cancelled: 'bg-text-on-inverse-muted',
}

const specialDays = computed(() => props.session.tracks.filter((track) => track.kind === 'special_day'))
const trackTo = (slug: string) =>
  localePath({ name: 'track-edition-track', params: { edition: props.edition.slug, track: slug } })

const place = computed(() =>
  [props.session.room_name ? tr(props.session.room_name) : '', props.edition.city ?? ''].filter(Boolean).join(', '),
)
const day = computed(() =>
  formatDate(props.session.starts_at, { locale: intlLocale.value, timeZone: props.session.timezone, dateStyle: 'full' }),
)

const logo = computed(() => {
  const asset = props.session.organization_logo
  return asset ? (asset.sources.thumb?.url ?? asset.url) : null
})

/** Le pays ne dit quelque chose que d'une institution publique nationale : il la désigne. */
const nationalCountry = computed(() =>
  props.session.organization_type_code === 'public_national_institution' && props.session.organization_country
    ? tr(props.session.organization_country)
    : '',
)
</script>

<template>
  <UiMediaBanner :image="image" :back-to="props.backTo" :back-label="t('activity.back')" :trail="props.trail">
    <div class="mt-8 flex flex-wrap gap-2 sm:mt-10">
      <UiBannerPill :tone="props.state === 'live' ? 'live' : 'neutral'" :dot="stateDot[props.state]">
        {{ stateLabel }}
      </UiBannerPill>
      <UiBannerPill v-if="showFormat(props.session)">
        <UiIcon name="globe" size="0.9375rem" :stroke-width="2" />
        {{ t(`session-card.format.${props.session.format}`) }}
      </UiBannerPill>
      <UiBannerPill v-for="track in specialDays" :key="track.slug" tone="accent" :to="trackTo(track.slug)">
        {{ tr(track.title) }}
      </UiBannerPill>
    </div>

    <h1
      class="mt-5.5 max-w-280 font-sans leading-[1.1] font-light text-balance text-text-on-inverse"
      :class="[titleSize, { 'line-through decoration-2': props.state === 'cancelled' }]"
    >
      <b class="font-bold">{{ parts.lead }}</b>{{ parts.rest }}
    </h1>

    <p v-if="props.state === 'cancelled' && props.cancelledReason" class="mt-4 max-w-200 text-base">
      <b class="font-bold">{{ t('activity.hero.cancelledReason') }}</b>
      {{ props.cancelledReason }}
    </p>

    <ul class="mt-5.5 flex flex-wrap items-center gap-x-8 gap-y-3 text-base font-bold">
      <li v-if="place" class="inline-flex items-center gap-2.5">
        <UiIcon name="map-pin" size="1.125rem" :stroke-width="2" />
        {{ place }}
      </li>
      <li class="inline-flex items-center gap-2.5">
        <UiIcon name="calendar" size="1.125rem" :stroke-width="2" />
        <span class="inline-block first-letter:uppercase">{{ day }}</span>
      </li>
    </ul>

    <div v-if="props.session.organization_name" class="mt-6 flex items-center gap-4">
      <span
        class="flex h-16 max-w-[45%] min-w-16 shrink-0 items-center justify-center rounded-lg bg-surface-inverse-selected px-4 shadow-glass"
      >
        <img v-if="logo" :src="logo" alt="" class="max-h-11 max-w-full object-contain sm:max-w-44">
        <span v-else class="text-lg font-bold tracking-[0.02em] text-text-on-inverse-selected" aria-hidden="true">
          {{ props.session.organization_acronym ?? initialsOf(props.session.organization_name) }}
        </span>
      </span>
      <p class="min-w-0">
        <span class="block text-xs font-bold tracking-caps text-text-on-inverse-muted uppercase">
          {{ t('activity.hero.ledBy') }}
        </span>
        <b class="mt-0.5 block text-[1.0625rem]">{{ props.session.organization_name }}</b>
        <span v-if="nationalCountry" class="mt-1 inline-flex items-center gap-1.5 text-sm text-text-on-inverse-muted">
          <UiCountryFlag
            v-if="props.session.organization_country_code"
            :code="props.session.organization_country_code"
            :label="nationalCountry"
            aria-hidden="true"
          />
          {{ nationalCountry }}
        </span>
      </p>
    </div>
  </UiMediaBanner>
</template>
