<script setup lang="ts">
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'

interface Props {
  session: PublicScheduleRow
  edition: PublicEditionRow
  state: ProgrammeSessionState
  backTo: string
  trail: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { intlLocale } = useDateTime()
const { titleParts, showFormat } = useProgrammeSession()
const { image } = useEditionSummary(() => props.edition)

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

const pill = 'inline-flex min-h-9 items-center gap-2 rounded-full border px-3.5 text-sm font-bold'
</script>

<template>
  <header
    class="relative isolate overflow-hidden bg-surface-inverse pt-6 pb-28 text-text-on-inverse [--color-focus:var(--color-accent-on-inverse)] lg:pb-30"
  >
    <template v-if="image">
      <UiImage
        :image="image"
        ratio="auto"
        loading="eager"
        sizes="100vw"
        frame-class="size-full"
        class="absolute inset-0 -z-10"
        aria-hidden="true"
      />
      <div class="absolute inset-0 -z-10 bg-scrim/50" aria-hidden="true" />
      <div class="scrim-fade-top absolute inset-0 -z-10" aria-hidden="true" />
    </template>

    <div class="mx-auto max-w-7xl px-4 sm:px-6">
      <nav :aria-label="t('nav.breadcrumb.label')" class="flex flex-wrap items-center gap-x-5 text-sm">
        <NuxtLink
          :to="props.backTo"
          class="inline-flex min-h-11 items-center gap-2 text-[0.9375rem] font-bold text-text-on-inverse hover:text-text-on-inverse hover:underline"
        >
          <UiIcon name="arrow-left" size="1rem" :stroke-width="2" />
          {{ t('activity.back') }}
        </NuxtLink>
        <span class="min-w-0 text-text-on-inverse-muted">{{ props.trail }}</span>
      </nav>

      <div class="mt-8 flex flex-wrap gap-2 sm:mt-10">
        <span :class="[pill, props.state === 'live' ? 'border-glass-border bg-live/40' : 'border-glass-border bg-glass-raised']">
          <span class="size-2 shrink-0 rounded-full" :class="stateDot[props.state]" aria-hidden="true" />
          {{ stateLabel }}
        </span>
        <span v-if="showFormat(props.session)" :class="[pill, 'border-glass-border bg-glass-raised']">
          <UiIcon name="globe" size="0.9375rem" :stroke-width="2" />
          {{ t(`session-card.format.${props.session.format}`) }}
        </span>
        <span
          v-for="track in specialDays"
          :key="track.slug"
          :class="[pill, 'border-accent-on-inverse/60 bg-accent-on-inverse/12']"
        >
          {{ tr(track.title) }}
        </span>
      </div>

      <h1
        class="mt-5.5 max-w-280 font-sans leading-[1.1] font-light text-balance text-text-on-inverse"
        :class="[titleSize, { 'line-through decoration-2': props.state === 'cancelled' }]"
      >
        <b class="font-bold">{{ parts.lead }}</b>{{ parts.rest }}
      </h1>

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
    </div>
  </header>
</template>
