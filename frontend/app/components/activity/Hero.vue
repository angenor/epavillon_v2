<script setup lang="ts">
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'

/** Le haut de la fiche, en affiche : repères, titre, porteur, bandeau horaire. */

interface Props {
  session: PublicScheduleRow
  edition: PublicEditionRow
  state: ProgrammeSessionState
  /** Heure de la personne qui consulte, quand elle diffère de celle de l'édition. */
  homeTime: { range: string; zone: string } | null
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time, zoneLabel, zoneOffsetShort } = useDateTime()
const { fill, showFormat } = useProgrammeSession()

const timezone = computed(() => props.session.timezone)
const day = computed(() => dayKeyInZone(props.session.starts_at, timezone.value))
const at = computed(() => new Date(`${day.value}T12:00:00Z`))
const weekday = computed(() => new Intl.DateTimeFormat(locale.value, { weekday: 'short', timeZone: 'UTC' }).format(at.value))
const month = computed(() => new Intl.DateTimeFormat(locale.value, { month: 'short', year: 'numeric', timeZone: 'UTC' }).format(at.value))

const minutes = computed(() => Math.round((Date.parse(props.session.ends_at) - Date.parse(props.session.starts_at)) / 60_000))
const duration = computed(() => {
  const hours = Math.floor(minutes.value / 60)
  const rest = minutes.value % 60
  if (!hours) return t('programme.list.minutes', { minutes: minutes.value })
  return rest ? t('programme.list.hoursMinutes', { hours, minutes: String(rest).padStart(2, '0') }) : t('programme.list.hours', { hours })
})

/** Le pays ne dit quelque chose que d'une institution publique nationale : il la désigne. */
const nationalCountry = computed(() =>
  props.session.organization_type_code === 'public_national_institution' && props.session.organization_country
    ? tr(props.session.organization_country)
    : '',
)

const specialDays = computed(() => props.session.tracks.filter((track) => track.kind === 'special_day'))
const titleSize = computed(() => (tr(props.session.title).length > 60 ? 'text-[clamp(2.5rem,5.5vw,4rem)]' : 'text-[clamp(2.75rem,6.5vw,4.75rem)]'))
</script>

<template>
  <section class="min-w-0">
    <div class="flex flex-wrap items-center gap-2">
      <span
        v-for="theme in props.session.themes.slice(0, 3)"
        :key="theme.code"
        class="inline-flex h-7.5 items-center rounded-full border-2 border-poster-ink px-3 text-[0.8125rem] font-bold"
        :style="{ background: fill(theme.color) }"
      >
        {{ tr(theme.label) }}
      </span>
      <span v-if="props.session.themes.length > 3" class="font-poster-mono text-xs">+{{ props.session.themes.length - 3 }}</span>
      <span
        v-for="track in specialDays"
        :key="track.slug"
        class="inline-flex h-7.5 items-center gap-1.5 rounded border-2 border-poster-ink bg-poster-paper-raised px-3 text-[0.8125rem] font-bold"
      >
        <UiIcon name="star" size="0.875rem" :style="track.color ? { color: track.color } : undefined" />
        {{ tr(track.title) }}
      </span>
      <span
        v-if="props.state === 'live'"
        class="inline-flex h-7.5 items-center rounded bg-live px-3 text-[0.8125rem] font-bold tracking-[0.06em] text-live-contrast uppercase"
      >
        ● {{ t('activity.state.live') }}
      </span>
      <span
        v-else-if="props.state !== 'upcoming'"
        class="inline-flex h-7.5 items-center rounded px-3 text-[0.8125rem] font-bold"
        :class="{
          'bg-poster-today-strong text-poster-on-today': props.state === 'ongoing',
          'bg-postponed-surface text-postponed border-2 border-postponed-border': props.state === 'postponed',
          'bg-poster-past text-poster-ink-muted': props.state === 'past' || props.state === 'cancelled',
        }"
      >
        {{ t(`session-card.state.${props.state}`) }}
      </span>
      <span v-if="showFormat(props.session)" class="ml-1 font-poster-mono first:ml-0 text-xs font-semibold tracking-[0.08em] text-poster-ink-muted uppercase">
        {{ t(`session-card.format.${props.session.format}`) }}
      </span>
    </div>

    <h1
      class="mt-5 font-poster leading-[0.95] font-black tracking-[-0.015em] text-balance font-stretch-[72%]"
      :class="[titleSize, { 'line-through decoration-4': props.state === 'cancelled' }]"
    >
      {{ tr(props.session.title) }}
    </h1>

    <div v-if="props.session.organization_name" class="mt-6 flex items-center gap-3.5">
      <span
        class="inline-flex h-13 min-w-13 items-center justify-center rounded-md bg-poster-ink px-2 font-poster text-[0.9375rem] font-black text-poster-on-ink-accent font-stretch-[75%]"
        aria-hidden="true"
      >
        {{ props.session.organization_acronym ?? initialsOf(props.session.organization_name) }}
      </span>
      <p class="min-w-0">
        <span class="block font-poster-mono text-[0.6875rem] font-semibold tracking-[0.08em] text-poster-ink-muted uppercase">
          {{ t('activity.hero.ledBy') }}
        </span>
        <span class="text-[1.0625rem] font-semibold">{{ props.session.organization_name }}</span>
        <span v-if="nationalCountry" class="text-poster-ink-muted">
          ·
          <span class="inline-flex items-center gap-1.5 align-[-0.125em]">
            <span v-if="props.session.organization_country_code" class="inline-flex" aria-hidden="true">
              <UiCountryFlag :code="props.session.organization_country_code" :label="nationalCountry" />
            </span>
            {{ nationalCountry }}
          </span>
        </span>
      </p>
    </div>

    <div class="mt-6 flex flex-wrap items-stretch overflow-hidden rounded-md border-2 border-poster-ink bg-poster-paper-raised shadow-poster-sm sm:w-fit">
      <div class="flex shrink-0 items-baseline gap-2 bg-poster-ink px-3 py-2.5 text-poster-on-ink sm:px-4">
        <span class="font-poster-mono text-xs font-semibold tracking-[0.08em] uppercase">{{ weekday }}</span>
        <span class="font-poster text-[2.25rem] leading-[0.85] font-black text-poster-on-ink-accent font-stretch-[62%]">{{ day.slice(8) }}</span>
        <span class="font-poster-mono text-xs">{{ month }}</span>
      </div>
      <div class="flex min-w-0 flex-1 flex-col justify-center px-3 py-2 sm:px-4">
        <p class="font-poster-mono text-lg leading-none sm:text-[1.375rem] font-semibold tracking-[-0.02em] whitespace-nowrap">
          {{ time(props.session.starts_at, timezone) }} — {{ time(props.session.ends_at, timezone) }}
        </p>
        <p class="mt-1 text-xs text-poster-ink-muted">
          {{ zoneLabel(timezone, props.edition.city ?? undefined) }} ({{ zoneOffsetShort(timezone, props.session.starts_at) }}) · {{ duration }}
        </p>
      </div>
      <div
        v-if="props.homeTime"
        class="flex w-full flex-col justify-center border-t-2 border-poster-ink bg-poster-paper px-4 py-2 sm:w-auto sm:border-t-0 sm:border-l-2"
      >
        <p class="font-poster-mono text-[0.625rem] font-semibold tracking-[0.08em] text-poster-ink-muted uppercase">
          {{ t('activity.hero.yourTime') }}
        </p>
        <p class="mt-1 flex flex-wrap items-baseline gap-x-2">
          <span class="font-poster-mono text-[0.9375rem] font-semibold whitespace-nowrap">{{ props.homeTime.range }}</span>
          <span class="text-xs text-poster-ink-muted">{{ props.homeTime.zone }}</span>
        </p>
      </div>
    </div>
  </section>
</template>
