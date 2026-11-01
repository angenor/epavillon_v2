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
const { showFormat } = useProgrammeSession()
const { image } = useEditionSummary(() => props.edition)
const localePath = useLocalePath()

const title = computed(() => tr(props.session.title))
// Un titre très long garde trois ou quatre lignes au plus.
const titleSize = computed(() =>
  title.value.length > 90 ? 'text-[clamp(2rem,3.6vw,3rem)]' : 'text-[clamp(2.2rem,4.6vw,3.8rem)]',
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

const specialDays = computed(() =>
  props.session.tracks
    .filter((track) => track.kind === 'special_day')
    .map((track) => ({
      slug: track.slug,
      title: tr(track.title),
      style: trackDayStyle(track.color),
      to: localePath({ name: 'track-edition-track', params: { edition: props.edition.slug, track: track.slug } }),
    })),
)

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
  <!-- Bas réduit sur une colonne : rien n'y remonte (le billet et l'image ne débordent qu'à partir de `lg`). -->
  <UiMediaBanner class="max-lg:pb-9!" :image="image" :back-to="props.backTo" :back-label="t('activity.back')" :trail="props.trail">
    <div class="mt-5 flex flex-wrap gap-2">
      <UiBannerPill :tone="props.state === 'live' ? 'live' : 'neutral'" :dot="stateDot[props.state]">
        {{ stateLabel }}
      </UiBannerPill>
      <UiBannerPill v-if="showFormat(props.session)">
        <UiIcon name="globe" size="0.9375rem" :stroke-width="2" />
        {{ t(`session-card.format.${props.session.format}`) }}
      </UiBannerPill>
      <NuxtLink
        v-for="track in specialDays"
        :key="track.slug"
        :to="track.to"
        :style="track.style"
        class="inline-flex min-h-11 items-center gap-2 rounded-full bg-(--color-day-solid) px-3.5 text-sm font-bold text-(--color-day-on-solid) no-underline hover:text-(--color-day-on-solid) hover:underline"
      >
        <UiIcon name="calendar" size="0.9375rem" :stroke-width="2" />
        {{ track.title }}
      </NuxtLink>
    </div>

    <h1
      class="mt-4 max-w-[30ch] font-condensed leading-[1.02] font-semibold tracking-[0.005em] text-balance text-text-on-inverse"
      :class="[titleSize, { 'line-through decoration-2': props.state === 'cancelled' }]"
    >
      {{ title }}
    </h1>

    <p v-if="props.state === 'cancelled' && props.cancelledReason" class="mt-4 max-w-200 text-base">
      <b class="font-bold">{{ t('activity.hero.cancelledReason') }}</b>
      {{ props.cancelledReason }}
    </p>

    <ul class="mt-4.5 flex flex-wrap items-center gap-x-6.5 gap-y-2.5 text-[0.95rem] text-text-on-inverse">
      <li v-if="place" class="inline-flex items-center gap-2">
        <UiIcon name="map-pin" size="1rem" :stroke-width="2" class="text-accent-on-inverse" />
        {{ place }}
      </li>
      <li class="inline-flex items-center gap-2">
        <UiIcon name="calendar" size="1rem" :stroke-width="2" class="text-accent-on-inverse" />
        <span class="inline-block first-letter:uppercase">{{ day }}</span>
      </li>
    </ul>

    <div v-if="props.session.organization_name" class="mt-5.5 flex items-center gap-3.5">
      <span
        class="flex h-16 max-w-[45%] min-w-16 shrink-0 items-center justify-center rounded-lg bg-surface-inverse-selected p-1.5 shadow-glass"
      >
        <img v-if="logo" :src="logo" alt="" class="max-h-13 max-w-full object-contain sm:max-w-44">
        <span v-else class="px-2 font-condensed text-lg font-bold tracking-[0.02em] text-text-on-inverse-selected" aria-hidden="true">
          {{ props.session.organization_acronym ?? initialsOf(props.session.organization_name) }}
        </span>
      </span>
      <p class="min-w-0">
        <span class="block font-condensed text-[0.95rem] font-semibold tracking-[0.05em] text-text-on-inverse-muted uppercase">
          {{ t('activity.hero.ledBy') }}
        </span>
        <b class="block text-[1.02rem] font-bold">{{ props.session.organization_name }}</b>
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
