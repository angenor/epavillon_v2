<script setup lang="ts">
import { NuxtLink } from '#components'

/**
 * Bouton ou lien, selon `vers`. L'état `actif` sert aux commandes qui basculent
 * — Favori, M'inscrire, Dans mon agenda : le mot ne change pas, l'aplat le dit,
 * et `aria-pressed` le fait entendre.
 */
withDefaults(
  defineProps<{
    variante?: 'principal' | 'secondaire' | 'discret' | 'dangereux'
    largeur?: 'pleine' | 'demie'
    vers?: string
    type?: 'button' | 'submit'
    desactive?: boolean
    actif?: boolean
    picto?: string
  }>(),
  {
    variante: 'principal',
    largeur: 'pleine',
    vers: undefined,
    type: 'button',
    desactive: false,
    actif: false,
    picto: undefined,
  },
)

defineEmits<{ clic: [MouseEvent] }>()
</script>

<template>
  <component
    :is="vers && !desactive ? NuxtLink : 'button'"
    :to="vers && !desactive ? vers : undefined"
    :type="vers && !desactive ? undefined : type"
    :disabled="vers ? undefined : desactive || undefined"
    :aria-disabled="vers && desactive ? 'true' : undefined"
    :aria-pressed="actif ? 'true' : undefined"
    class="gn-bouton"
    :class="[`gn-bouton--${variante}`, `gn-bouton--${largeur}`, { 'gn-bouton--actif': actif }]"
    @click="$emit('clic', $event)"
  >
    <GnPicto v-if="picto" :nom="picto" :taille="20" />
    <slot />
  </component>
</template>

<style>
[data-app="guide-nego"] .gn-bouton {
  min-height: var(--gn-cible);
  padding-inline: var(--gn-espace-16);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  border: none;
  border-radius: var(--gn-rayon-4);
  background: none;
  font-size: var(--gn-taille-17);
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

[data-app="guide-nego"] .gn-bouton--principal {
  background: var(--gn-accent);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-bouton--secondaire {
  border: var(--gn-filet-2) solid var(--gn-filet-fort);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-bouton--discret {
  color: var(--gn-accent);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-bouton--dangereux {
  background: var(--gn-danger);
  color: var(--gn-charte-blanc);
}

[data-app="guide-nego"][data-theme="sombre"] .gn-bouton--dangereux {
  color: var(--gn-nuance-sombre-fond);
}

/* L'état actif d'une commande qui bascule : aplat plein, jamais un simple changement de teinte. */
[data-app="guide-nego"] .gn-bouton--secondaire.gn-bouton--actif {
  background: var(--gn-titre);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"][data-theme="sombre"] .gn-bouton--secondaire.gn-bouton--actif {
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-bouton:not(:disabled):active {
  filter: brightness(0.9);
}

[data-app="guide-nego"] .gn-bouton--secondaire:not(:disabled):active,
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
