<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
import { NuxtLink } from '#components'

/**
 * Une ligne de réglage : toute la ligne est la cible, jamais le seul chevron.
 * Sans `vers`, elle ne mène nulle part et porte sa commande en fin de ligne.
 * `#sous` ajoute sous la valeur ce qu'elle ne sait pas écrire — une barre d'avancée.
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
    :class="{ 'gn-reglage--derniere': derniere, 'gn-reglage--cible': vers, 'gn-reglage--picto': picto }"
  >
    <span v-if="picto" class="gn-reglage__pastille"><GnPicto :nom="picto" :taille="20" /></span>
    <span class="gn-reglage__texte">
      <span class="gn-reglage__libelle">{{ libelle }}</span>
      <span v-if="valeur" class="gn-reglage__valeur">{{ valeur }}</span>
      <slot name="sous" />
    </span>
    <slot />
    <GnPicto v-if="vers" nom="chevron" :taille="20" class="gn-reglage__chevron" />
  </component>
</template>

<style>
/* La ligne de « Se préparer » (maquette 05) : pastille d'icône de 44, titre 16/700, détail 14. */
[data-app="guide-nego"] .gn-reglage {
  min-height: var(--gn-ligne-reglage);
  padding-block: var(--gn-espace-8);
  display: flex;
  align-items: center;
  gap: 14px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet-doux);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-reglage--picto {
  min-height: 68px;
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

[data-app="guide-nego"] .gn-reglage__pastille {
  flex: none;
  width: var(--gn-pastille-icone);
  height: var(--gn-pastille-icone);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-14);
  background: var(--gn-fond-2);
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-reglage__texte {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  /* Une adresse électronique n'a pas d'espace où revenir à la ligne. */
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-reglage__libelle {
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-reglage__valeur {
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-reglage__chevron {
  color: var(--gn-picto-secondaire);
}
</style>
