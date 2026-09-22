<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
import { NuxtLink } from '#components'

/**
 * Une ligne de réglage : toute la ligne est la cible, jamais le seul chevron.
 * Sans `vers`, elle ne mène nulle part et porte sa commande en fin de ligne.
 */
withDefaults(
  defineProps<{
    libelle: string
    valeur?: string
    picto?: NomDePicto
    vers?: string
    /** La dernière d'une liste n'a pas de filet : le bloc s'achève de lui-même. */
    derniere?: boolean
  }>(),
  { valeur: undefined, picto: undefined, vers: undefined, derniere: false },
)
</script>

<template>
  <component
    :is="vers ? NuxtLink : 'div'"
    :to="vers"
    class="gn-reglage"
    :class="{ 'gn-reglage--derniere': derniere, 'gn-reglage--cible': vers }"
  >
    <GnPicto v-if="picto" :nom="picto" />
    <span class="gn-reglage__texte">
      <span class="gn-reglage__libelle">{{ libelle }}</span>
      <span v-if="valeur" class="gn-reglage__valeur">{{ valeur }}</span>
    </span>
    <slot />
    <GnPicto v-if="vers" nom="chevron" class="gn-reglage__chevron" />
  </component>
</template>

<style>
[data-app="guide-nego"] .gn-reglage {
  min-height: var(--gn-ligne-reglage);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-reglage--derniere {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-reglage--cible:active {
  background: var(--gn-presse);
  /* La pression déborde les marges de l'écran, comme une ligne pleine largeur. */
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-reglage > .gn-picto {
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-reglage__texte {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  /* Une adresse électronique n'a pas d'espace où revenir à la ligne. */
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-reglage__libelle {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-reglage__valeur {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-reglage__chevron {
  color: var(--gn-picto-secondaire);
}
</style>
