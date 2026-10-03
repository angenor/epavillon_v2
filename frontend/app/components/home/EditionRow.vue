<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'

/**
 * UNE ÉDITION DANS LA VUE « LISTE » DE L'ACCUEIL — le sommaire d'une revue,
 * arbitré le 03/10 comme alternative au rail d'affiches. Mêmes données que
 * `HomeEditionCard`, par `useEditionSummary` : deux vues, une seule lecture.
 */

interface Props {
  edition: PublicEditionRow
  /** Séances publiées — déjà résolu, l'absence de ligne valant zéro. */
  sessionCount: number
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { to, image, dates, zone, place, stamp } = useEditionSummary(() => props.edition)

const isPast = computed(() => props.edition.temporal_state === 'past')
</script>

<template>
  <article
    data-bird-perch
    class="group relative grid grid-cols-[7rem_minmax(0,1fr)] items-start gap-x-4 gap-y-1.5 py-4 sm:grid-cols-[12rem_minmax(0,1fr)] sm:gap-x-6 lg:grid-cols-[12rem_minmax(0,1fr)_14rem_8rem_1.5rem] lg:items-center lg:gap-y-0"
  >
    <div
      class="relative row-span-3 aspect-video overflow-hidden rounded-md bg-surface-inverse lg:row-span-1"
    >
      <UiImage
        v-if="image"
        :image="image"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0 transition duration-200 motion-safe:group-hover:scale-[1.03]"
        :class="{ grayscale: isPast }"
        sizes="(min-width: 640px) 12rem, 7rem"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center font-display text-sm font-bold text-text-on-inverse/20 sm:text-xl"
        aria-hidden="true"
      >
        {{ stamp }}
      </span>
    </div>

    <div class="min-w-0">
      <p
        v-if="props.edition.series_name"
        class="truncate text-xs uppercase text-text-muted"
        :style="{ letterSpacing: 'var(--tracking-caps)' }"
      >
        {{ tr(props.edition.series_name) }}
      </p>
      <!-- Lien couvrant : toute la ligne mène à l'édition, sans second lien à tabuler. -->
      <h3 class="mt-0.5 font-display text-base leading-snug sm:text-xl">
        <NuxtLink :to="to" class="text-text no-underline after:absolute after:inset-0 hover:underline">
          {{ tr(props.edition.title) }}
        </NuxtLink>
      </h3>
      <p class="mt-1 text-sm text-accent">
        {{ t('home.history.sessions', { count: props.sessionCount }, props.sessionCount) }}
      </p>
    </div>

    <p class="col-start-2 text-sm text-text lg:col-start-auto">
      {{ dates }}
      <span class="block text-xs text-text-muted">
        {{ zone }}<template v-if="place"> · {{ place }}</template>
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
        size="1.25rem"
        class="transition-transform duration-200 motion-safe:group-hover:translate-x-1"
      />
    </span>
  </article>
</template>
