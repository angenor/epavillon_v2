<script setup lang="ts">
import type { ShowcaseRow } from '~/types/views'

/**
 * LA RANGÉE DE VIGNETTES DE LA VITRINE, sous la citation.
 *
 * La diapositive courante se signale trois fois, la couleur ne portant jamais
 * seule l'information (règle d'usage n° 3) : un cadre blanc, une barre cyan qui
 * se remplit au rythme du défilement, `aria-current` et un mot pour le lecteur
 * d'écran.
 *
 * LE BOUTON LECTURE / PAUSE RESTE VISIBLE PARTOUT (WCAG 2.2.2) : la maquette ne
 * le dessine pas, mais un contenu qui bouge plus de cinq secondes doit pouvoir
 * s'arrêter. Sur téléphone, la rangée de vignettes disparaît (maquette) et deux
 * flèches la remplacent.
 *
 * Vignette : `thumbnail`, à défaut `background_image`, à défaut l'aplat de la
 * diapositive. Aucune image inventée.
 */

interface Props {
  slides: ShowcaseRow[]
  current: number
  playing: boolean
  /** La personne a demandé moins d'animations : le rail le dit, le bouton lecture reste actif. */
  reducedMotion?: boolean
  /** Le défilement tourne à cet instant (ni pause, ni survol, ni onglet caché). */
  running: boolean
  /** Durée de la diapositive courante, en millisecondes. */
  duration: number
  /** Change à chaque départ de la minuterie : la barre repart de zéro avec elle. */
  cycle: number
}

const props = defineProps<Props>()

const emit = defineEmits<{
  select: [index: number]
  previous: []
  next: []
  toggle: []
}>()

const { t } = useI18n()
const { tr } = useI18nText()

const total = computed(() => props.slides.length)

const roundButton =
  'flex size-(--target-min) shrink-0 cursor-pointer items-center justify-center rounded-full border border-glass-border bg-glass text-text-on-inverse shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover'
</script>

<template>
  <div v-if="total > 1" class="pointer-events-none pb-8 text-text-on-inverse">
    <div class="mx-auto flex w-full max-w-[1280px] items-center gap-2.5 px-4 sm:px-6 lg:px-8">
      <div class="pointer-events-auto flex min-w-0 items-center gap-2.5 lg:max-w-[calc(100%-400px-3rem)]">
        <button type="button" :class="roundButton" class="lg:hidden" :aria-label="t('home.showcase.previous')" @click="emit('previous')">
          <UiIcon name="chevron-left" size="1.25rem" />
        </button>

        <!-- Seule la rangée défile horizontalement ; le corps de page, jamais. -->
        <ul class="hidden min-w-0 items-center gap-2.5 overflow-x-auto py-1 lg:flex" :aria-label="t('home.showcase.railLabel')">
          <li v-for="(slide, index) in props.slides" :key="slide.id" class="shrink-0">
            <button
              type="button"
              class="relative block h-16 w-28 cursor-pointer overflow-hidden rounded-md border-2 transition-opacity"
              :class="index === props.current ? 'border-text-on-inverse' : 'border-transparent opacity-65 hover:opacity-100'"
              :style="
                showcaseThumbnail(slide)
                  ? undefined
                  : { backgroundColor: slide.background_color_hex ?? 'var(--color-surface-inverse)' }
              "
              :aria-current="index === props.current ? 'true' : undefined"
              @click="emit('select', index)"
            >
              <UiImage
                :image="showcaseThumbnail(slide)"
                ratio="auto"
                frame-class="size-full"
                class="absolute inset-0"
                sizes="112px"
              />

              <!-- Elle se remplit pendant la diapositive, s'arrête avec la lecture, reste pleine en pause. -->
              <span v-if="index === props.current" class="absolute inset-x-0 bottom-0 h-[3px]" aria-hidden="true">
                <span
                  :key="props.cycle"
                  class="block h-full bg-accent-on-inverse"
                  :class="props.playing ? 'showcase-progress' : 'w-full'"
                  :style="
                    props.playing
                      ? { animationDuration: `${props.duration}ms`, animationPlayState: props.running ? 'running' : 'paused' }
                      : undefined
                  "
                />
              </span>

              <span class="sr-only">
                {{ t('home.showcase.goTo', { index: index + 1, title: tr(slide.title) }) }}
                <template v-if="index === props.current">, {{ t('home.showcase.current') }}</template>
              </span>
            </button>
          </li>
        </ul>

        <button type="button" :class="roundButton" class="lg:hidden" :aria-label="t('home.showcase.next')" @click="emit('next')">
          <UiIcon name="chevron-right" size="1.25rem" />
        </button>

        <button
          type="button"
          :class="roundButton"
          :aria-label="props.playing ? t('home.showcase.pause') : t('home.showcase.play')"
          @click="emit('toggle')"
        >
          <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
            <path v-if="props.playing" d="M8 5h3v14H8zM13 5h3v14h-3z" />
            <path v-else d="M8 5.5v13l11-6.5z" />
          </svg>
        </button>
      </div>

      <!-- Seul l'état de lecture est annoncé : il ne change que sur une action de la personne.
           La position se lit à la demande, sur la vignette courante. -->
      <p class="sr-only" aria-live="polite">
        {{ props.playing ? t('home.showcase.playing') : t('home.showcase.paused') }}
        <template v-if="props.reducedMotion && !props.playing">
          {{ t('home.showcase.reducedMotion') }}
        </template>
      </p>
    </div>
  </div>
</template>

<style scoped>
.showcase-progress {
  width: 0;
  animation-name: showcase-progress;
  animation-timing-function: linear;
  animation-fill-mode: forwards;
}

@keyframes showcase-progress {
  to {
    width: 100%;
  }
}
</style>
