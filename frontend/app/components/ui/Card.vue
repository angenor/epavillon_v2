<script setup lang="ts">
/**
 * Carte — bloc de contenu délimité.
 *
 * DIRECTION ARTISTIQUE : la hiérarchie passe par le TRAIT et le BLANC, pas par
 * l'ombre. Une carte est donc bordée et posée sur la surface haute, avec une
 * ombre à peine perceptible. « Beaucoup de blanc entre les blocs, peu à
 * l'intérieur des blocs » : l'espacement intérieur est serré (16 px), l'écart
 * entre deux cartes reste large, et c'est à la grille qui les contient de le
 * donner.
 *
 * CARTE CLIQUABLE — `to`. Le lien enveloppe alors le TITRE, pas la carte
 * entière : une carte-lien intégrale rend le texte impossible à sélectionner,
 * les liens intérieurs inatteignables, et produit un nom accessible démesuré. Le
 * pseudo-élément d'extension (`ui-card-link`) élargit la zone de clic à toute la
 * carte tout en gardant un seul lien focalisable — c'est le compromis que
 * retient `UiSessionCard`.
 */

interface Props {
  /** Titre du bloc. Il porte le lien quand `to` est fourni. */
  title?: string
  /** Surtitre : catégorie, date, numéro de dossier. */
  eyebrow?: string
  /** Destination ; rend le titre cliquable et étend la zone de clic à la carte. */
  to?: string
  /** Niveau de titre HTML — à accorder à la hiérarchie de la page. */
  as?: 'h2' | 'h3' | 'h4'
  /** Carte sans marges intérieures : tableaux, listes bord à bord. */
  flush?: boolean
  /** Carte en retrait plutôt qu'en relief — encarts secondaires. */
  sunken?: boolean
  /** État sélectionné dans une liste à choix (planificateur, comparaison). */
  selected?: boolean
}

const props = withDefaults(defineProps<Props>(), { as: 'h3' })
</script>

<template>
  <article
    class="relative rounded-lg border transition-colors"
    :class="[
      props.sunken ? 'bg-surface-sunken' : 'bg-surface-raised shadow-xs',
      props.selected ? 'border-accent ring-1 ring-accent' : 'border-border',
      props.to ? 'hover:border-border-strong focus-within:border-accent' : '',
    ]"
  >
    <header
      v-if="props.title || props.eyebrow || $slots.header"
      :class="props.flush ? 'border-b border-border-subtle px-4 py-3' : 'px-4 pt-4'"
    >
      <slot name="header">
        <p v-if="props.eyebrow" class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
          {{ props.eyebrow }}
        </p>
        <component :is="props.as" v-if="props.title" class="mt-1 text-lg">
          <!-- Zone de clic étendue à toute la carte, un seul lien focalisable. -->
          <NuxtLink v-if="props.to" :to="props.to" class="ui-card-link no-underline text-text hover:text-accent">
            {{ props.title }}
          </NuxtLink>
          <template v-else>{{ props.title }}</template>
        </component>
      </slot>
    </header>

    <div :class="props.flush ? '' : 'px-4 py-4'">
      <slot />
    </div>

    <footer
      v-if="$slots.footer"
      class="border-t border-border-subtle px-4 py-3"
    >
      <slot name="footer" />
    </footer>
  </article>
</template>

<style scoped>
/* Le lien du titre couvre la carte. Les éléments interactifs placés dans le
   contenu remontent au-dessus par `position: relative` (voir `UiSessionCard`). */
.ui-card-link::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
}
</style>
