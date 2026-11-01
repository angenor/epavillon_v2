<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'

/**
 * UNE ÉDITION DANS LA VUE « LISTE » DE L'ACCUEIL : millésime en grand,
 * vignette, nature, titre, volume, dates, état (maquette du 01/11). Mêmes
 * données que `HomeEditionCard`, par `useEditionSummary` : deux vues, une
 * seule lecture.
 */

interface Props {
  edition: PublicEditionRow
  /** Séances publiées : déjà résolu, l'absence de ligne valant zéro. */
  sessionCount: number
  /** Millésime ANNONCÉ (`groupEditionsByYear`), pas l'année de `starts_at`. */
  year: number
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { intlLocale } = useDateTime()
const { to, image, zone, place, stamp } = useEditionSummary(() => props.edition)

const isPast = computed(() => props.edition.temporal_state === 'past')

/** « 9–20 novembre » : le millésime est déjà à gauche ; il revient si l'édition chevauche deux années. */
const shortDates = computed(() => {
  const { starts_at: start, ends_at: end, timezone } = props.edition
  const spansYears =
    new Intl.DateTimeFormat('en', { year: 'numeric', timeZone: timezone }).format(new Date(start)) !==
    new Intl.DateTimeFormat('en', { year: 'numeric', timeZone: timezone }).format(new Date(end))
  return new Intl.DateTimeFormat(intlLocale.value, {
    day: 'numeric',
    month: 'long',
    ...(spansYears ? { year: 'numeric' } : {}),
    timeZone: timezone,
  }).formatRange(new Date(start), new Date(end))
})
</script>

<template>
  <article
    data-bird-perch
    class="group relative grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-x-4 gap-y-2.5 py-5 sm:grid-cols-[6rem_minmax(0,1fr)] lg:grid-cols-[7.5rem_12.5rem_minmax(0,1fr)_13.75rem_auto_2rem] lg:gap-6"
  >
    <p
      class="row-span-3 self-start font-condensed text-[2.2rem] leading-[0.9] font-bold tabular-nums lg:row-span-1 lg:self-center lg:text-5xl"
      :class="isPast ? 'text-text-muted' : 'text-heading'"
    >
      {{ props.year }}
    </p>

    <div class="relative hidden aspect-video overflow-hidden rounded-lg bg-surface-inverse lg:block">
      <UiImage
        v-if="image"
        :image="image"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0 transition duration-300 motion-safe:group-hover:scale-105"
        sizes="12.5rem"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center font-condensed text-xl font-bold text-text-on-inverse/20"
        aria-hidden="true"
      >
        {{ stamp }}
      </span>
    </div>

    <div class="min-w-0">
      <p
        v-if="props.edition.series_name"
        class="truncate font-condensed text-[0.95rem] font-semibold tracking-[0.06em] text-text-muted uppercase"
      >
        {{ tr(props.edition.series_name) }}
      </p>
      <!-- Lien couvrant : toute la ligne mène à l'édition, sans second lien à tabuler. -->
      <h3 class="mt-1 font-condensed text-[1.3rem] leading-[1.1] font-semibold sm:text-[1.55rem]">
        <NuxtLink
          :to="to"
          class="text-heading no-underline after:absolute after:inset-0 group-hover:text-accent group-hover:underline"
        >
          {{ tr(props.edition.title) }}
        </NuxtLink>
      </h3>
      <p class="mt-1.5 text-sm font-semibold text-accent">
        {{ t('home.history.sessions', { count: props.sessionCount }, props.sessionCount) }}
      </p>
    </div>

    <p class="col-start-2 lg:col-start-auto">
      <span class="block font-condensed text-[1.2rem] leading-tight font-bold tracking-[0.02em] text-heading uppercase">
        {{ shortDates }}
      </span>
      <span class="block text-[0.82rem] text-text-muted">
        <template v-if="place">{{ place }}, </template>{{ zone }}
      </span>
    </p>

    <div class="col-start-2 lg:col-start-auto">
      <UiStatusBadge
        :state="props.edition.temporal_state"
        size="sm"
        :label="t(`home.history.state.${props.edition.temporal_state}`)"
      />
    </div>

    <span class="hidden text-accent lg:block" aria-hidden="true">
      <UiIcon
        name="arrow-right"
        size="1.3rem"
        class="transition-transform duration-200 motion-safe:group-hover:translate-x-1"
      />
    </span>
  </article>
</template>
