<script setup lang="ts">
import type { ShowcaseRow } from '~/types/views'

/**
 * LE RAIL DE VIGNETTES ET SES COMMANDES — la barre basse du bandeau.
 *
 * ── LA COULEUR NE PORTE JAMAIS SEULE UNE INFORMATION (règle d'usage n° 3) ────
 *
 * La diapositive courante est signalée TROIS FOIS, et chacune s'adresse à
 * quelqu'un de différent : un cerne pour l'œil, une coche pour qui ne distingue
 * pas les teintes, `aria-current` et un mot pour qui n'a que la voix de son
 * lecteur d'écran. Le compteur « 3 / 7 » ferme la boucle, visible de tous.
 *
 * ── LE BOUTON LECTURE / PAUSE EST OBLIGATOIRE, ET IL EST VISIBLE ────────────
 *
 * Un contenu qui bouge plus de cinq secondes doit pouvoir être arrêté
 * (WCAG 2.2.2). Il ne se cache donc ni au survol, ni derrière un menu.
 *
 * Son icône est dessinée ici, en SVG, et non tirée de `UiIcon` : ce jeu-là ne
 * porte ni triangle de lecture ni barres de pause, et l'enrichir depuis cet
 * écran reviendrait à modifier un composant partagé pour un besoin local. Le
 * jour où une seconde surface en aura besoin, la paire montera dans `UiIcon`.
 *
 * ── LA VIGNETTE A SES PROPRES REPLIS ────────────────────────────────────────
 *
 * `thumbnail`, à défaut `background_image`, à défaut l'aplat
 * `background_color_hex`. Une diapositive « chiffre clé » n'a souvent aucun
 * média : son carré de couleur reste un repère utilisable, là où une image de
 * remplacement inventée n'en serait pas un.
 */

interface Props {
  slides: ShowcaseRow[]
  /** Index de la diapositive à l'écran. */
  current: number
  /** L'autodéfilement tourne-t-il ? */
  playing: boolean
  /**
   * La personne a demandé moins d'animations. Le rail le DIT plutôt que de
   * laisser croire à une panne : le bouton lecture reste actif, un choix
   * explicite l'emportant sur une préférence générale.
   */
  reducedMotion?: boolean
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
</script>

<template>
  <!-- LA BARRE SE POSE SUR LE MÉDIA, elle ne le coupe pas.

       C'est le rendu de la plateforme de référence, arbitré le 19/08 : un aplat
       institutionnel tranchait la photographie en deux, alors qu'un fondu la
       prolonge. `.scrim-fade-bottom` porte ce dégradé — le SEUL de la charte, et
       il ne décore rien : il rend lisible un texte blanc sur une image dont on
       ignore la luminosité. Le flou du verre agit par-dessus, sur les commandes.
       -->
  <div
    v-if="total > 1"
    class="scrim-fade-bottom flex items-center gap-2 px-2 pt-10 pb-3 text-text-on-inverse sm:gap-3 sm:px-4"
  >
    <button
      type="button"
      class="flex shrink-0 cursor-pointer items-center justify-center rounded-full border border-glass-border bg-glass text-text-on-inverse shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover"
      :style="{ width: 'var(--target-min)', height: 'var(--target-min)' }"
      :aria-label="t('home.showcase.previous')"
      @click="emit('previous')"
    >
      <UiIcon name="chevron-left" size="1.25rem" />
    </button>

    <!-- SEUL LE RAIL DÉFILE HORIZONTALEMENT. Le corps de page, jamais. -->
    <ul
      class="flex min-w-0 flex-1 items-center gap-2 overflow-x-auto py-1"
      :aria-label="t('home.showcase.railLabel')"
    >
      <li v-for="(slide, index) in props.slides" :key="slide.id" class="shrink-0">
        <button
          type="button"
          class="relative block h-11 w-[4.5rem] cursor-pointer overflow-hidden rounded-md border-(length:--border-medium) shadow-glass transition-all sm:h-14 sm:w-24"
          :class="
            index === props.current
              ? 'border-glass-border-strong ring-2 ring-glass-border-strong'
              : 'border-glass-border opacity-65 hover:opacity-100'
          "
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
            sizes="96px"
          />

          <!-- La coche : le second signal, celui qui ne dépend pas de la teinte
               du cerne. -->
          <span
            v-if="index === props.current"
            class="absolute inset-x-0 bottom-0 flex items-center justify-center bg-accent-solid py-0.5 text-accent-contrast"
            aria-hidden="true"
          >
            <UiIcon name="check" size="0.8rem" />
          </span>

          <span class="sr-only">
            {{ t('home.showcase.goTo', { index: index + 1, title: tr(slide.title) }) }}
            <template v-if="index === props.current"> — {{ t('home.showcase.current') }}</template>
          </span>
        </button>
      </li>
    </ul>

    <button
      type="button"
      class="flex shrink-0 cursor-pointer items-center justify-center rounded-full border border-glass-border bg-glass text-text-on-inverse shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover"
      :style="{ width: 'var(--target-min)', height: 'var(--target-min)' }"
      :aria-label="t('home.showcase.next')"
      @click="emit('next')"
    >
      <UiIcon name="chevron-right" size="1.25rem" />
    </button>

    <button
      type="button"
      class="flex shrink-0 cursor-pointer items-center justify-center rounded-full border border-glass-border bg-glass text-text-on-inverse shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover"
      :style="{ width: 'var(--target-min)', height: 'var(--target-min)' }"
      :aria-label="props.playing ? t('home.showcase.pause') : t('home.showcase.play')"
      @click="emit('toggle')"
    >
      <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
        <path v-if="props.playing" d="M8 5h3v14H8zM13 5h3v14h-3z" />
        <path v-else d="M8 5.5v13l11-6.5z" />
      </svg>
    </button>

    <!-- LE COMPTEUR N'EST PAS UNE RÉGION VIVANTE, et c'est délibéré : il change à
         chaque bascule automatique, soit toutes les sept secondes. Déclaré
         `aria-live`, il faisait annoncer « 2 sur 7 », « 3 sur 7 »… par-dessus la
         lecture en cours, indéfiniment. La position se lit à la demande, sur la
         vignette courante qui porte `aria-current`. -->
    <p class="ml-1 hidden shrink-0 rounded-full border border-glass-border bg-glass px-3 py-1 text-sm tabular-nums text-text-on-inverse backdrop-blur-glass sm:block">
      {{ t('home.showcase.position', { index: props.current + 1, total }) }}
    </p>

    <!-- CE QUI EST VIVANT, c'est l'état de lecture, et lui seul : il ne change
         que sur une action de la personne, et l'annoncer est le seul moyen de
         savoir que la pause a pris pour qui ne voit pas l'icône. -->
    <p class="sr-only" aria-live="polite">
      {{ props.playing ? t('home.showcase.playing') : t('home.showcase.paused') }}
      <template v-if="props.reducedMotion && !props.playing">
        {{ t('home.showcase.reducedMotion') }}
      </template>
    </p>
  </div>
</template>
