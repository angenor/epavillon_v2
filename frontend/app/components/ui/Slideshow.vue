<script setup lang="ts" generic="T extends UiSlide">
import type { UiSlide } from '~/types/ui'

/**
 * Des images qui se relaient en fondu, posées sur tout le cadre de l'appelant
 * (positionné, qui porte ses proportions).
 *
 * Avec `controls`, un diaporama : pastilles pour choisir, bouton lecture/pause
 * (WCAG 2.2.2), défilement suspendu au survol, au focus et onglet caché.
 * Sans, une vignette discrète, décorative, qui tourne seule plus lentement.
 * Dans les deux cas, une barre d'avancement en bas du cadre, absente si « moins d'animations ».
 *
 * « Moins d'animations » arrête la minuterie, lue en JavaScript (`utils/showcase.ts`) :
 * la feuille de style coupe les fondus, pas un `setTimeout`. Le bouton lecture
 * reste actif : un choix explicite l'emporte sur une préférence générale.
 * Rien ne tourne avant l'hydratation : le serveur rend la première image.
 */

interface Props {
  slides: T[]
  /** Durée d'une image, en millisecondes. */
  interval?: number
  controls?: boolean
  /** Nom du diaporama pour un lecteur d'écran ; requis avec `controls`. */
  label?: string
  sizes?: string
  /** La première image est au-dessus de la ligne de flottaison. */
  eager?: boolean
  imageClass?: string | Record<string, boolean> | Array<string | Record<string, boolean>>
}

const props = withDefaults(defineProps<Props>(), {
  interval: 6000,
  controls: false,
  label: '',
  sizes: undefined,
  eager: false,
  imageClass: undefined,
})

defineSlots<{
  /** Une image absente : ce qui la remplace dans le cadre. */
  fallback?: (scope: { slide: T; index: number }) => unknown
  /** Posé sur l'image, en bas à gauche, et fondu avec elle. */
  caption?: (scope: { slide: T; index: number }) => unknown
}>()

const { t } = useI18n()

const current = ref(0)
const playing = ref(false)
const interacting = ref(false)
const hidden = ref(false)
const reducedMotion = ref(false)

const total = computed(() => props.slides.length)
const running = computed(() => playing.value && !interacting.value && !hidden.value && total.value > 1)

let timer: ReturnType<typeof setTimeout> | null = null
/** Change à chaque départ de minuterie et à chaque image : la barre d'avancement repart de zéro avec elle. */
const cycle = ref(0)
const mounted = ref(false)
const showProgress = computed(() => mounted.value && !reducedMotion.value && total.value > 1)

function schedule(): void {
  if (timer !== null) clearTimeout(timer)
  timer = null
  if (!running.value) return
  timer = setTimeout(() => (current.value = showcaseIndexAfter(current.value, total.value, 1)), props.interval)
  cycle.value += 1
}

function onVisibilityChange(): void {
  hidden.value = document.visibilityState === 'hidden'
}

let stopMotionWatch: (() => void) | null = null

onMounted(() => {
  reducedMotion.value = prefersReducedMotion()
  playing.value = !reducedMotion.value
  hidden.value = document.visibilityState === 'hidden'
  document.addEventListener('visibilitychange', onVisibilityChange)
  mounted.value = true
  stopMotionWatch = onReducedMotionChange((reduced) => {
    reducedMotion.value = reduced
    if (reduced) {
      playing.value = false
      if (!props.controls) current.value = 0
    } else if (!props.controls) playing.value = true
  })
  schedule()
})

onBeforeUnmount(() => {
  if (timer !== null) clearTimeout(timer)
  document.removeEventListener('visibilitychange', onVisibilityChange)
  stopMotionWatch?.()
})

// Chaque image obtient son temps entier, y compris celle qu'on vient de choisir.
watch([current, running], schedule)
// Une image choisie en pause (survol compris) remet aussi la barre à zéro, figée.
watch(current, () => (cycle.value += 1))
watch(total, (count) => {
  current.value = clampShowcaseIndex(current.value, count)
})

const fade = computed(() => (props.controls ? 'duration-700' : 'duration-1000'))
</script>

<template>
  <div
    class="absolute inset-0 overflow-hidden"
    :role="props.controls ? 'region' : undefined"
    :aria-roledescription="props.controls ? t('slideshow.carouselRole') : undefined"
    :aria-label="props.controls ? props.label : undefined"
    :aria-hidden="props.controls ? undefined : 'true'"
    @mouseenter="interacting = props.controls"
    @mouseleave="interacting = false"
    @focusin="interacting = props.controls"
    @focusout="interacting = false"
  >
    <div
      v-for="(slide, index) in props.slides"
      :key="slide.key"
      class="absolute inset-0 transition-opacity ease-in-out"
      :class="[fade, index === current ? 'opacity-100' : 'pointer-events-none opacity-0']"
      :role="props.controls ? 'group' : undefined"
      :aria-roledescription="props.controls ? t('slideshow.slideRole') : undefined"
      :aria-label="props.controls ? t('slideshow.position', { index: index + 1, total }) : undefined"
      :aria-hidden="props.controls && index !== current ? 'true' : undefined"
      :inert="index !== current || undefined"
    >
      <UiImage
        v-if="slide.image"
        :image="slide.image"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0"
        :class="props.imageClass"
        :loading="props.eager && index === 0 ? 'eager' : 'lazy'"
        :sizes="props.sizes"
      />
      <slot v-else name="fallback" :slide="slide" :index="index" />
      <div v-if="$slots.caption" class="absolute bottom-3 left-3 max-w-[55%] sm:bottom-4 sm:left-4">
        <slot name="caption" :slide="slide" :index="index" />
      </div>
    </div>

    <template v-if="props.controls && total > 1">
      <div class="scrim-fade-bottom pointer-events-none absolute inset-x-0 bottom-0 h-2/5" aria-hidden="true" />
      <div class="absolute right-2 bottom-2 flex items-center sm:right-3 sm:bottom-3">
        <button
          v-for="(slide, index) in props.slides"
          :key="slide.key"
          type="button"
          class="group/dot flex size-(--target-min) cursor-pointer items-center justify-center"
          :aria-label="t('slideshow.goTo', { index: index + 1, total })"
          :aria-current="index === current ? 'true' : undefined"
          @click="current = index"
        >
          <span
            class="block size-2.5 rounded-full border border-glass-border-strong transition-colors"
            :class="index === current ? 'bg-text-on-inverse' : 'bg-glass group-hover/dot:bg-glass-hover'"
          />
        </button>
        <button
          type="button"
          class="ml-1 flex size-(--target-min) shrink-0 cursor-pointer items-center justify-center rounded-full border border-glass-border bg-glass text-text-on-inverse shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover"
          :aria-label="playing ? t('slideshow.pause') : t('slideshow.play')"
          @click="playing = !playing"
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path v-if="playing" d="M8 5h3v14H8zM13 5h3v14h-3z" />
            <path v-else d="M8 5.5v13l11-6.5z" />
          </svg>
        </button>
      </div>
      <!-- Seul l'état de lecture est annoncé : il ne change que sur une action de la personne. -->
      <p class="sr-only" aria-live="polite">
        {{ playing ? t('slideshow.playing') : t('slideshow.paused') }}
        <template v-if="reducedMotion && !playing">{{ t('slideshow.reducedMotion') }}</template>
      </p>
    </template>

    <!-- Elle incite à attendre l'image suivante : elle avance avec la minuterie, se fige avec elle. -->
    <div
      v-if="showProgress"
      class="pointer-events-none absolute inset-x-0 bottom-0 z-1"
      :class="props.controls ? 'h-1 bg-glass-border' : 'h-0.75'"
      aria-hidden="true"
    >
      <span
        :key="cycle"
        class="slideshow-progress block h-full"
        :class="props.controls ? 'bg-[var(--color-day-mark,var(--color-accent))]' : 'bg-[var(--color-day-solid,var(--color-accent))]'"
        :style="{ animationDuration: `${props.interval}ms`, animationPlayState: running ? 'running' : 'paused' }"
      />
    </div>
  </div>
</template>

<style scoped>
.slideshow-progress {
  transform: scaleX(0);
  transform-origin: left;
  animation-name: slideshow-progress;
  animation-timing-function: linear;
  animation-fill-mode: forwards;
}

@keyframes slideshow-progress {
  to {
    transform: scaleX(1);
  }
}
</style>
