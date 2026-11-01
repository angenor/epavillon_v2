<script setup lang="ts">
import type { HighlightId } from '~/types/content'
import type { ShowcaseRow } from '~/types/views'

/**
 * LA VITRINE DE L'ACCUEIL : la diapositive, la rangée de vignettes, ses
 * commandes, et le panneau « À venir » reçu par le créneau `aside`. Ce composant
 * porte ce qui demande le navigateur (minuteries, préférence de mouvement,
 * visibilité de l'onglet) ; le reste vit dans `utils/showcase.ts`.
 *
 * Elle sort du conteneur de 1280 px par `.full-bleed` (`main.css`), sans faire
 * défiler le corps ; la marge négative du haut compense le `py-8` du layout.
 *
 * L'image remplit le cadre par recadrage (`object-fit: cover`), jamais
 * déformée ; la citation se borne par `line-clamp`.
 *
 * `prefers-reduced-motion` ARRÊTE L'AUTODÉFILEMENT, lu en JavaScript : une
 * minuterie n'est ni une animation ni une transition CSS. Le bouton lecture
 * reste actif : un choix explicite l'emporte sur une préférence générale.
 * Onglet caché, pointeur ou focus dans la vitrine suspendent le défilement sans
 * toucher à l'état du bouton.
 */

interface Props {
  /** `content.v_showcase`, `placement = 'home_hero'`, déjà trié par `sort_order`. */
  slides: ShowcaseRow[]
}

const props = defineProps<Props>()

const { t } = useI18n()

const hasSlides = computed(() => props.slides.length > 0)

const current = ref(0)

/** État du BOUTON. Faux au rendu serveur : rien ne défile avant l'hydratation. */
const playing = ref(false)

/** Suspensions passagères, elles n'écrivent jamais dans `playing`. */
const interacting = ref(false)
const hidden = ref(false)

const reducedMotion = ref(false)

/**
 * Les diapositives dont la vidéo a échoué. On les retient : sans cette mémoire,
 * un bandeau de deux diapositives dont l'une porte une vidéo cassée tournerait
 * sur son erreur au lieu de se rabattre sur l'image.
 */
const failedVideos = ref<Set<HighlightId>>(new Set())

const slide = computed(() => showcaseAt(props.slides, current.value))

const skipVideo = (row: ShowcaseRow): boolean =>
  reducedMotion.value || failedVideos.value.has(row.id)

/** Le défilement tourne-t-il RÉELLEMENT à cet instant ? */
const running = computed(
  () => playing.value && !interacting.value && !hidden.value && props.slides.length > 1,
)

let timer: ReturnType<typeof setTimeout> | null = null
const cycle = ref(0)
const duration = computed(() => (slide.value ? showcaseDurationMs(slide.value, { skipVideo: skipVideo(slide.value) }) : 0))

function clearTimer(): void {
  if (timer !== null) {
    clearTimeout(timer)
    timer = null
  }
}

function schedule(): void {
  clearTimer()
  if (!running.value) return
  const row = slide.value
  if (!row) return
  timer = setTimeout(() => go(1), showcaseDurationMs(row, { skipVideo: skipVideo(row) }))
  cycle.value += 1
}

function go(step: number): void {
  current.value = showcaseIndexAfter(current.value, props.slides.length, step)
}

function select(index: number): void {
  current.value = clampShowcaseIndex(index, props.slides.length)
}

/**
 * Une vidéo qui ne charge pas fait passer à la suivante, comportement de la
 * v1, et il est juste. La diapositive fautive n'est pas perdue pour autant :
 * elle repassera avec son image.
 */
function onMediaError(): void {
  const row = slide.value
  if (!row) return
  const next = new Set(failedVideos.value)
  next.add(row.id)
  failedVideos.value = next
  if (props.slides.length > 1) go(1)
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
  stopMotionWatch = onReducedMotionChange((reduced) => {
    reducedMotion.value = reduced
    if (reduced) playing.value = false
  })
  schedule()
})

onBeforeUnmount(() => {
  clearTimer()
  document.removeEventListener('visibilitychange', onVisibilityChange)
  stopMotionWatch?.()
})

// La minuterie repart à chaque changement de diapositive et à chaque
// suspension : chaque diapositive obtient son temps entier, y compris celle
// qu'on vient d'atteindre en cliquant sur une vignette.
watch([current, running], schedule)

// Une liste qui rétrécit (rechargement, fenêtre de diffusion échue) ne laisse
// pas un index mort derrière elle.
watch(
  () => props.slides.length,
  (total) => {
    current.value = clampShowcaseIndex(current.value, total)
  },
)
</script>

<template>
  <!-- Grand écran : la diapositive et le panneau « À venir » partagent la même
       cellule de grille ; le panneau, en verre, se cale à droite du cadre de
       1280 px, en bas. Une cellule plutôt qu'une position absolue : un panneau
       plus haut que le bandeau l'agrandit au lieu d'être rogné. Téléphone : le
       panneau suit la vitrine, sur l'aplat. -->
  <section
    class="full-bleed relative -mt-8 grid overflow-hidden bg-surface-inverse sm:-mt-10"
    :aria-roledescription="t('home.showcase.carouselRole')"
    :aria-label="t('home.showcase.label')"
    @mouseenter="interacting = true"
    @mouseleave="interacting = false"
    @focusin="interacting = true"
    @focusout="interacting = false"
  >
    <!-- Hauteur de la maquette validée le 01/11 : un écran, plafonné à 720 px.
         Elle ne dépend pas du contenu ; `svh` mesure la fenêtre barres d'adresse
         visibles, la barre collée et le bandeau institutionnel sont retranchés. -->
    <div
      v-if="hasSlides"
      class="relative flex min-h-[min(720px,calc(100svh-var(--nav-height)-var(--topbar-height)))] flex-col lg:[grid-area:1/1]"
    >
      <Transition name="showcase-slide">
        <HomeShowcaseSlide
          v-if="slide"
          :key="slide.id"
          :slide="slide"
          :skip-video="skipVideo(slide)"
          :paused="!running"
          eager
          class="flex-1"
          content-class="pb-16 lg:pe-[calc(400px+3rem)] lg:pb-[6.5rem]"
          @media-error="onMediaError"
        />
      </Transition>

      <HomeShowcaseRail
        class="absolute inset-x-0 bottom-0 z-10"
        :slides="props.slides"
        :current="current"
        :playing="playing"
        :reduced-motion="reducedMotion"
        :running="running"
        :duration="duration"
        :cycle="cycle"
        @select="select"
        @previous="go(-1)"
        @next="go(1)"
        @toggle="playing = !playing"
      />
    </div>

    <!-- C'est l'information la plus utile de la page : jamais un tiroir. -->
    <div class="relative z-20" :class="hasSlides ? 'lg:pointer-events-none lg:[grid-area:1/1]' : ''">
      <div
        class="mx-auto w-full max-w-[1280px] px-4 py-6 sm:px-6"
        :class="hasSlides ? 'lg:flex lg:h-full lg:items-end lg:justify-end lg:px-8 lg:pt-14 lg:pb-8' : 'lg:py-10'"
      >
        <div :class="hasSlides ? 'lg:pointer-events-auto lg:w-[400px]' : ''">
          <slot name="aside" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.showcase-slide-enter-active,
.showcase-slide-leave-active {
  transition: opacity 700ms ease;
}
.showcase-slide-leave-active {
  position: absolute;
  inset: 0;
  z-index: 1;
}
.showcase-slide-enter-from,
.showcase-slide-leave-to {
  opacity: 0;
}

.showcase-slide-enter-active :deep(img),
.showcase-slide-enter-active :deep(video) {
  transition: transform 1400ms cubic-bezier(0.2, 0.7, 0.2, 1);
}
.showcase-slide-enter-from :deep(img),
.showcase-slide-enter-from :deep(video) {
  transform: scale(1.05);
}

.showcase-slide-enter-active :deep([data-showcase-text]) {
  transition:
    transform 700ms cubic-bezier(0.2, 0.7, 0.2, 1) 200ms,
    opacity 700ms ease 200ms;
}
.showcase-slide-enter-from :deep([data-showcase-text]) {
  transform: translateY(16px);
  opacity: 0;
}
</style>
