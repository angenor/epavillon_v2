<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
import { NuxtLink } from '#components'

/**
 * Bouton ou lien, selon `vers`. L'état `actif` sert aux commandes qui basculent
 * — Favori, M'inscrire, Dans mon agenda : le mot ne change pas, l'aplat le dit,
 * et `aria-pressed` le fait entendre.
 *
 * `chargement` désactive le bouton et met l'arc à la place du pictogramme. L'attente est
 * annoncée par `aria-busy` sur le bouton lui-même : l'arc reste muet, sinon la commande
 * porterait une région vivante en son sein.
 */
const props = withDefaults(
  defineProps<{
    variante?: 'principal' | 'secondaire' | 'discret' | 'dangereux'
    largeur?: 'pleine' | 'demie'
    vers?: string
    type?: 'button' | 'submit'
    desactive?: boolean
    actif?: boolean
    chargement?: boolean
    picto?: NomDePicto
  }>(),
  {
    variante: 'principal',
    largeur: 'pleine',
    vers: undefined,
    type: 'button',
    desactive: false,
    actif: false,
    chargement: false,
    picto: undefined,
  },
)

defineEmits<{ clic: [MouseEvent] }>()

/** Un bouton qui attend ne se presse plus : le chargement vaut désactivation. */
const inerte = computed(() => props.desactive || props.chargement)
</script>

<template>
  <component
    :is="vers && !inerte ? NuxtLink : 'button'"
    :to="vers && !inerte ? vers : undefined"
    :type="vers && !inerte ? undefined : type"
    :disabled="vers ? undefined : inerte || undefined"
    :aria-disabled="vers && inerte ? 'true' : undefined"
    :aria-pressed="actif ? 'true' : undefined"
    :aria-busy="chargement ? 'true' : undefined"
    class="gn-bouton"
    :class="[`gn-bouton--${variante}`, `gn-bouton--${largeur}`, { 'gn-bouton--actif': actif }]"
    @click="$emit('clic', $event)"
  >
    <GnChargement v-if="chargement" :taille="20" decoratif />
    <GnPicto v-else-if="picto" :nom="picto" :taille="20" />
    <slot />
  </component>
</template>

<style>
/* Le bouton de la maquette : 52 de haut, rayon 16, texte 15. */
[data-app="guide-nego"] .gn-bouton {
  min-height: var(--gn-bouton-principal);
  padding-inline: var(--gn-espace-20);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  border: none;
  border-radius: var(--gn-rayon-16);
  background: none;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-align: center;
  text-decoration: none;
}

[data-app="guide-nego"] .gn-bouton--pleine {
  width: 100%;
}

[data-app="guide-nego"] .gn-bouton--demie {
  flex: 1;
}

/* Un aplat d'accent, d'attention ou d'alerte porte toujours son texte en 800. */
[data-app="guide-nego"] .gn-bouton--principal {
  background: var(--gn-accent);
  color: var(--gn-accent-inv);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-bouton--secondaire {
  border: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-bouton--discret {
  color: var(--gn-accent);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-bouton--dangereux {
  background: var(--gn-danger-aplat);
  color: var(--gn-danger-texte);
  font-weight: var(--gn-graisse-extra-gras);
}

/* L'état actif d'une commande qui bascule : l'aplat d'accent, comme « Dans mes favoris ». */
[data-app="guide-nego"] .gn-bouton--secondaire.gn-bouton--actif {
  background: var(--gn-accent);
  border-color: var(--gn-accent);
  color: var(--gn-accent-inv);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-bouton:not(:disabled):active {
  filter: brightness(0.9);
}

[data-app="guide-nego"] .gn-bouton--secondaire:not(.gn-bouton--actif):not(:disabled):active,
[data-app="guide-nego"] .gn-bouton--discret:not(:disabled):active {
  background: var(--gn-presse);
  filter: none;
}

[data-app="guide-nego"] .gn-bouton:disabled,
[data-app="guide-nego"] .gn-bouton[aria-disabled="true"] {
  background: var(--gn-desactive-fond);
  border-color: var(--gn-desactive-fond);
  color: var(--gn-desactive-texte);
  cursor: not-allowed;
  text-decoration: none;
}
</style>
