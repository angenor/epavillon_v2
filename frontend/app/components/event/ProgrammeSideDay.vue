<script setup lang="ts">
import type { PublicScheduleTrack } from '~/types/programme/track-page'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'atelier préparatoire dans `/programmations` : un encadré bleu nuit, sans
 * déroulé, qui mène à sa page. Ce n'est pas une journée spéciale et il ne
 * s'appelle jamais ainsi.
 */

interface Props {
  track: PublicScheduleTrack
  to: string
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()

const kicker = computed(() =>
  props.track.venue_name
    ? t('programme.tracks.side.kickerVenue', { venue: tr(props.track.venue_name) })
    : t('programme.tracks.side.kicker'),
)
const details = computed(() => {
  const { starts_at: start, ends_at: end } = props.track
  const hours = start && end ? t('programme.tracks.side.hours', { start: time(start, props.timezone), end: time(end, props.timezone) }) : ''
  return [hours, t('programme.tracks.side.audience')].filter(Boolean).join(' · ')
})
</script>

<template>
  <section
    class="flex flex-col gap-4 rounded-[10px] lg:-ml-3 bg-surface-inverse px-5 py-5 text-text-on-inverse sm:flex-row sm:items-center sm:justify-between sm:px-6"
    :aria-label="tr(props.track.title)"
  >
    <div class="min-w-0">
      <p class="text-xs font-bold text-text-on-inverse/75 uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">{{ kicker }}</p>
      <p class="mt-1.5 text-lg leading-snug font-bold text-balance text-text-on-inverse sm:text-xl">{{ tr(props.track.title) }}</p>
      <p class="mt-1.5 text-sm text-text-on-inverse/80 tabular-nums">{{ details }}</p>
    </div>
    <UiButton variant="on-inverse" :to="props.to" icon-trailing="arrow-right" class="shrink-0 self-start sm:self-center">
      {{ t('programme.tracks.side.open') }}
    </UiButton>
  </section>
</template>
