<script setup lang="ts">
import type { PublicTrackPage } from '~/types/programme/track-page'
import type { PublicScheduleRow } from '~/types/views'

/**
 * La carte levée en tête de la colonne : le direct le jour J (la tête de liste
 * de `/programmations`, en petit), puis la journée en quelques temps, chacun
 * menant à sa partie du déroulé. Sans parties, la présentation prend la place.
 */

interface Props {
  page: PublicTrackPage
  live: PublicScheduleRow | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()

const timezone = computed(() => props.page.edition.timezone)
const subject = computed(() => (props.page.track.kind === 'special_day' ? 'day' : 'other'))

const tiles = computed(() =>
  props.page.parts
    .filter((part) => part.starts_at)
    .map((part) => ({
      id: part.id,
      anchor: `#partie-${part.number}`,
      label: part.label ? tr(part.label) : t('track.part.default', { number: part.number }),
      hours:
        part.starts_at && part.ends_at
          ? t('track.part.hours', { start: time(part.starts_at, timezone.value), end: time(part.ends_at, timezone.value) })
          : '',
      text: part.summary ? tr(part.summary) : part.title ? tr(part.title) : '',
    })),
)

const paragraphs = computed(() =>
  (props.page.track.description ? tr(props.page.track.description) : '')
    .split(/\n{2,}/)
    .map((text) => text.trim())
    .filter(Boolean),
)
</script>

<template>
  <section class="rounded-lg bg-surface-raised p-6 shadow-lg" aria-labelledby="journee-apercu-titre">
    <div v-if="props.live" id="direct" class="mb-6 border-b border-separator pb-6">
      <EventProgrammeLiveLead :session="props.live" :edition-slug="props.page.edition.slug" :timezone="timezone" compact />
    </div>

    <template v-if="tiles.length > 1">
      <h2 id="journee-apercu-titre" class="font-sans text-xl font-bold text-heading">
        {{ t(`track.overview.${subject}`, { count: tiles.length }, tiles.length) }}
      </h2>
      <ol class="mt-5 grid gap-3 md:grid-cols-3">
        <li v-for="tile in tiles" :key="tile.id">
          <a
            :href="tile.anchor"
            class="block h-full min-h-11 rounded-b-md border-t-3 border-(--color-day-line) bg-(--color-day-surface) p-4 text-text no-underline transition-colors hover:bg-surface-hover"
          >
            <span class="block text-xs font-bold text-(--color-day) uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
              {{ tile.label }}
            </span>
            <span v-if="tile.hours" class="mt-1 block text-sm text-text-muted tabular-nums">{{ tile.hours }}</span>
            <b v-if="tile.text" class="mt-2 block leading-snug">{{ tile.text }}</b>
          </a>
        </li>
      </ol>
    </template>

    <template v-else>
      <h2 id="journee-apercu-titre" class="font-sans text-xl font-bold text-heading">{{ t(`track.about.${subject}`) }}</h2>
      <p v-for="(paragraph, index) in paragraphs" :key="index" class="mt-3 max-w-(--measure)" :class="index ? 'text-text-secondary' : 'text-lg'">
        {{ paragraph }}
      </p>
    </template>
  </section>
</template>
