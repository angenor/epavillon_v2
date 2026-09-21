<script setup lang="ts">
/**
 * La pilule de filtre : 40 px de dessin dans une cible de 48, gagnée par un
 * pseudo-élément — grossir la pilule changerait le dessin, pas la portée du doigt.
 *
 * Trois variantes : `filtre` bascule ; `decochable` bascule en montrant une coche
 * (les suggestions de l'IA) ; `chevron` ouvre une feuille de choix et ne bascule
 * rien — l'appelant y répond, et lui seul compose « Type : Bulletin » ou « Type : 2 ».
 */
const props = withDefaults(
  defineProps<{
    variante?: 'filtre' | 'decochable' | 'chevron'
    desactive?: boolean
    /** Variante `chevron` seulement : la feuille de choix est ouverte. */
    ouverte?: boolean
  }>(),
  { variante: 'filtre', desactive: false, ouverte: false },
)

const emit = defineEmits<{ clic: [MouseEvent] }>()
const choisie = defineModel<boolean>('choisie', { default: false })

function basculer(evenement: MouseEvent) {
  if (props.variante !== 'chevron') choisie.value = !choisie.value
  emit('clic', evenement)
}
</script>

<template>
  <button
    type="button"
    class="gn-pilule"
    :class="[`gn-pilule--${variante}`, { 'gn-pilule--choisie': choisie }]"
    :disabled="desactive || undefined"
    :aria-pressed="variante === 'chevron' ? undefined : choisie"
    :aria-haspopup="variante === 'chevron' ? 'dialog' : undefined"
    :aria-expanded="variante === 'chevron' ? ouverte : undefined"
    @click="basculer"
  >
    <GnPicto v-if="variante === 'decochable' && choisie" nom="check" :taille="16" />
    <slot />
    <GnPicto v-if="variante === 'chevron'" nom="chev-down" :taille="20" />
  </button>
</template>

<style>
[data-app="guide-nego"] .gn-pilule {
  position: relative;
  height: var(--gn-filtre-hauteur);
  padding-inline: var(--gn-espace-12);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  /* Le bord reste posé sous l'aplat : sans lui, être choisie décalerait le libellé de 2 px. */
  border: var(--gn-filet-2) solid var(--gn-filet);
  border-radius: var(--gn-rayon-24);
  background: none;
  color: var(--gn-texte);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  white-space: nowrap;
}

/* Le dessin fait 40, le doigt vise 48 : la cible déborde, la pilule ne grossit pas. */
[data-app="guide-nego"] .gn-pilule::after {
  content: '';
  position: absolute;
  inset-inline: 0;
  inset-block: calc((var(--gn-filtre-hauteur) - var(--gn-cible)) / 2);
}

[data-app="guide-nego"] .gn-pilule--choisie {
  background: var(--gn-titre);
  border-color: var(--gn-titre);
  color: var(--gn-sur-titre);
  font-weight: var(--gn-graisse-gras);
}

/* En sombre, l'aplat `titre` vire au presque-blanc : trop fort pour une rangée de filtres. */
[data-app="guide-nego"][data-theme="sombre"] .gn-pilule--choisie {
  background: var(--gn-accent);
  border-color: var(--gn-accent);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-pilule:not(.gn-pilule--choisie):not(:disabled):active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-pilule--choisie:not(:disabled):active {
  filter: brightness(0.9);
}

[data-app="guide-nego"] .gn-pilule:disabled {
  background: none;
  border-color: var(--gn-desactive-fond);
  color: var(--gn-desactive-texte);
  cursor: not-allowed;
}
</style>
