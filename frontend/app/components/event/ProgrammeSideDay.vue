<script setup lang="ts">
import type { PublicScheduleTrack } from '~/types/programme/track-page'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'atelier préparatoire dans `/programmations` : son étiquette sur un filet
 * bleu nuit, puis un panneau bleu nuit, sans déroulé, qui mène à sa page. Ce
 * n'est pas une journée spéciale et il ne s'appelle jamais ainsi.
 */

interface Props {
  track: PublicScheduleTrack
  to: string
  timezone: TimeZoneName
  /** Nul tant que l'horloge du navigateur n'a pas parlé. */
  state?: 'upcoming' | 'ongoing' | 'past' | null
}

const props = withDefaults(defineProps<Props>(), { state: null })

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()

const icon = computed(() => trackIcon(props.track.icon_code))
const details = computed(() => {
  const { starts_at: start, ends_at: end } = props.track
  const hours = start && end ? t('programme.tracks.side.hours', { start: time(start, props.timezone), end: time(end, props.timezone) }) : ''
  return [hours, props.track.venue_name ? tr(props.track.venue_name) : ''].filter(Boolean).join(', ')
})
</script>

<template>
  <section
    :aria-label="tr(props.track.title)"
    :class="{ 'opacity-60': props.state === 'past' }"
    :style="{ '--rule-solid': 'var(--color-surface-inverse)', '--rule-on': 'var(--color-text-on-inverse)' }"
  >
    <div class="rule-line">
      <span class="rule-label">
        <UiIcon v-if="icon" :name="icon" size="1.05em" />
        {{ t('programme.tracks.side.kicker') }}
      </span>
      <span class="pb-1.5 text-sm text-text-secondary">{{ t('programme.tracks.side.audience') }}</span>
    </div>

    <div
      class="flex flex-wrap items-center justify-between gap-x-6 gap-y-4 rounded-b-xl rounded-tr-xl bg-surface-inverse px-4 py-5 text-text-on-inverse sm:px-6"
    >
      <div class="min-w-0">
        <p class="font-condensed text-[1.45rem] leading-[1.1] font-semibold text-balance text-text-on-inverse sm:text-[1.6rem]">
          {{ tr(props.track.title) }}
        </p>
        <p v-if="details" class="mt-1.5 text-sm text-text-on-inverse-muted tabular-nums">{{ details }}</p>
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <UiStatusBadge v-if="props.state" :state="props.state" size="sm" :label="t(`programme.tracks.side.state.${props.state}`)" />
        <UiButton variant="on-inverse" :to="props.to">
          {{ t('programme.tracks.side.open') }}
        </UiButton>
      </div>
    </div>
  </section>
</template>
