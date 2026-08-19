<script setup lang="ts">
import type { HighlightId } from '~/types/content'
import type { ShowcaseRow } from '~/types/views'

/**
 * LE BANDEAU D'OUVERTURE DE L'ACCUEIL.
 *
 * Il porte la diapositive, le rail de vignettes, ses commandes, et il accueille
 * dans son créneau `aside` la colonne « À venir ». Trois responsabilités qui
 * demandent le navigateur — les minuteries, la préférence de mouvement, la
 * visibilité de l'onglet ; tout le reste est dans `utils/showcase.ts` ou dans
 * les composants qu'il assemble.
 *
 * ── IL SORT DU CONTENEUR DE 1280 px, ET LE CORPS NE DÉFILE PAS POUR AUTANT ──
 *
 * `layouts/public.vue` enferme chaque page dans un conteneur centré. La classe
 * `.full-bleed` de `main.css` l'en sort par des marges négatives calculées, sans
 * `position: absolute` ni largeur devinée ; l'en-tête de cette classe explique
 * pourquoi le débordement de la largeur de barre de défilement est exactement
 * ce que `html { overflow-x: hidden }` absorbe. La marge négative du haut
 * compense le `py-8` du layout, pour que le bandeau touche la barre de
 * navigation.
 *
 * ── LE BANDEAU FAIT UN ÉCRAN, TOUJOURS ──────────────────────────────────────
 *
 * `calc(100svh - var(--nav-height))`. Arbitrage du commanditaire, 19/08 : « la
 * hauteur du hero ne doit pas varier selon la taille de l'image, il doit
 * toujours rester plein écran ».
 *
 * La version précédente valait `min(84vh, 720px)` à partir de `lg` et `34rem`
 * en dessous — un PLANCHER, pensé pour laisser respirer une longue citation.
 * Elle avait deux défauts que seul l'écran révèle : le bandeau changeait de
 * hauteur d'une diapositive à l'autre, et il se plafonnait à 720 px sur un
 * écran de 1 400 px, laissant une bande vide sous lui.
 *
 * `svh` et non `vh` : sur mobile, `vh` mesure la fenêtre barres d'adresse
 * MASQUÉES ; le bandeau dépassait donc toujours d'une centaine de pixels.
 * `--nav-height` est retranchée parce que la barre est `sticky`.
 *
 * CE QUE CELA IMPOSE À LA CITATION : elle ne peut plus s'étirer, donc le
 * `line-clamp` cesse d'être un garde-fou pour devenir la règle. C'est le bon
 * sens de lecture — l'éditeur choisit son extrait, la mise en page ne décide
 * pas à sa place.
 *
 * L'IMAGE REMPLIT CE CADRE PAR RECADRAGE. `UiImage` pose `object-fit: cover` :
 * une photographie plus étroite ou plus haute que le bandeau est rognée, jamais
 * déformée. Une partie coupée est sans conséquence ; un visage étiré, non.
 *
 * ── `prefers-reduced-motion` ARRÊTE L'AUTODÉFILEMENT ────────────────────────
 *
 * Et il faut le lire EN JAVASCRIPT. `main.css` neutralise les animations et les
 * transitions CSS ; une minuterie n'en est ni l'une ni l'autre. Sans cette
 * lecture, la page continuerait de changer toutes les sept secondes sous les
 * yeux d'une personne qui a demandé le contraire.
 *
 * ELLE ARRÊTE, ELLE N'ACCÉLÈRE PAS, et elle ne condamne pas le bouton : une
 * personne qui appuie sur « lecture » a formulé un choix, qui l'emporte sur une
 * préférence générale.
 *
 * ── TROIS AUTRES RAISONS DE NE PAS DÉFILER ──────────────────────────────────
 *
 * L'onglet est en arrière-plan (rien à voir, autant ne pas consommer), le
 * pointeur est sur le bandeau (on lit), le focus est dedans (on navigue au
 * clavier dans le rail — voir ses diapositives changer sous les doigts serait
 * insupportable). Aucune ne modifie l'état du bouton : elles suspendent, elles
 * ne décident pas.
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

/** Suspensions passagères — elles n'écrivent jamais dans `playing`. */
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
}

function go(step: number): void {
  current.value = showcaseIndexAfter(current.value, props.slides.length, step)
}

function select(index: number): void {
  current.value = clampShowcaseIndex(index, props.slides.length)
}

/**
 * Une vidéo qui ne charge pas fait passer à la suivante — comportement de la
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
  <section
    class="full-bleed relative -mt-8 overflow-hidden bg-surface-inverse sm:-mt-10"
    :aria-roledescription="t('home.showcase.carouselRole')"
    :aria-label="t('home.showcase.label')"
    @mouseenter="interacting = true"
    @mouseleave="interacting = false"
    @focusin="interacting = true"
    @focusout="interacting = false"
  >
    <!-- LA ZONE DE DIAPOSITIVE. Sur grand écran, elle laisse à sa gauche les
         340 px du panneau « À venir » ; en dessous de `lg`, elle occupe toute la
         largeur et le panneau passe dessous. Le décalage est posé ICI, sur un
         élément qui ne porte aucune autre marge, et non mêlé aux classes du
         composant de diapositive. -->
    <!-- LA HAUTEUR DU BANDEAU NE DÉPEND PAS DE SON CONTENU.

         Elle valait `min(84vh, 720px)` sur grand écran et `34rem` en dessous :
         le bandeau changeait donc de taille d'une diapositive à l'autre, selon
         la longueur de la citation, et se plafonnait à 720 px sur un écran de
         1 400 px de haut. Le commanditaire a tranché le 19/08 : **un écran,
         toujours**.

         `100svh` et non `100vh` : sur mobile, `vh` mesure la fenêtre BARRES
         D'ADRESSE MASQUÉES, si bien que le bandeau dépassait toujours d'une
         centaine de pixels et qu'aucune diapositive ne tenait vraiment en un
         écran. `svh` mesure la fenêtre la plus petite — celle qu'on voit à
         l'arrivée sur la page.

         `--nav-height` est retranchée parce que la barre est `sticky` : sans
         elle, le bandeau ferait un écran PLUS la barre, et le bas serait
         toujours coupé. La barre se cale sur ce jeton, elle ne le subit pas.

         L'image, elle, remplit ce cadre par `object-fit: cover` (`UiImage`) :
         elle est recadrée, jamais déformée. -->
    <div
      v-if="hasSlides"
      class="relative flex min-h-[calc(100svh-var(--nav-height))] flex-col lg:ps-[340px]"
    >
      <!-- `flex-1` PLUTÔT QUE `absolute inset-0` : la diapositive porte déjà
           `position: relative` (ses couches de fond en dépendent), et deux
           classes de position sur un même élément se départagent par l'ordre de
           génération de Tailwind, pas par l'ordre d'écriture. On lui donne donc
           de la place, on ne lui impose pas de position. -->
      <div class="relative flex min-h-0 flex-1 flex-col">
        <HomeShowcaseSlide
          v-if="slide"
          :key="slide.id"
          :slide="slide"
          :skip-video="skipVideo(slide)"
          :paused="!running"
          eager
          class="flex-1"
          content-class="pb-28 sm:pb-32"
          @media-error="onMediaError"
        />

        <!-- L'INVITATION À DÉROULER. Reprise de la plateforme de référence : sur
             un bandeau qui occupe presque toute la hauteur de l'écran, rien ne
             dit qu'il y a une suite. C'est un repère, pas une commande — d'où
             `aria-hidden` : le contenu qu'il annonce est atteignable par la
             tabulation comme par le défilement, et l'annoncer une seconde fois
             à la voix n'apprendrait rien.

             L'animation s'arrête d'elle-même sous `prefers-reduced-motion` :
             `main.css` neutralise toutes les animations, celle-ci comprise. -->
        <div
          class="pointer-events-none absolute inset-x-0 bottom-24 z-10 hidden justify-center sm:bottom-28 lg:flex"
          aria-hidden="true"
        >
          <span
            class="flex animate-bounce items-center justify-center rounded-full border border-glass-border bg-glass text-text-on-inverse backdrop-blur-glass"
            :style="{ width: 'var(--target-compact)', height: 'var(--target-compact)' }"
          >
            <UiIcon name="chevron-down" size="1.1rem" />
          </span>
        </div>

        <!-- LE RAIL FLOTTE SUR LE MÉDIA, il ne se pose pas dessous.

             C'est la disposition de la plateforme de référence, arbitrée le
             19/08 : empilé, le rail rognait la photographie d'une bande de
             cent pixels et le bandeau perdait sa pleine hauteur. Superposé, il
             fait partie de l'image — son fondu (`.scrim-fade-bottom`) assure la
             jonction, et le contenu de la diapositive lui laisse la place par
             `content-class`, jamais en devinant sa hauteur. -->
        <HomeShowcaseRail
          class="absolute inset-x-0 bottom-0 z-10"
          :slides="props.slides"
          :current="current"
          :playing="playing"
          :reduced-motion="reducedMotion"
          @select="select"
          @previous="go(-1)"
          @next="go(1)"
          @toggle="playing = !playing"
        />
      </div>
    </div>

    <!-- LE PANNEAU « À VENIR ». Posé DANS le bandeau sur grand écran, il en
         sort en dessous de `lg` et devient une section à part entière — jamais
         un tiroir : c'est l'information la plus utile de la page, la cacher
         derrière un bouton serait un contresens. -->
    <div :class="hasSlides ? 'lg:absolute lg:inset-y-0 lg:start-0 lg:z-20 lg:w-[340px]' : ''">
      <slot name="aside" />
    </div>
  </section>
</template>
