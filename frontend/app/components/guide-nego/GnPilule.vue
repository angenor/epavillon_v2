<script setup lang="ts">
/**
 * La pilule de filtre, dessinée comme la pilule de jour de la maquette (44 px).
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
    /** Une pilule qui mène ailleurs — un terme lié — et ne bascule rien. */
    vers?: string
  }>(),
  { variante: 'filtre', desactive: false, ouverte: false, vers: undefined },
)

const emit = defineEmits<{ clic: [MouseEvent] }>()
const choisie = defineModel<boolean>('choisie', { default: false })

function basculer(evenement: MouseEvent) {
  if (props.variante !== 'chevron') choisie.value = !choisie.value
  emit('clic', evenement)
}
</script>

<template>
  <NuxtLink v-if="vers" :to="vers" class="gn-pilule gn-pilule--lien">
    <slot />
  </NuxtLink>
  <button
    v-else
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
/* La pilule de jour de la maquette : 44 de haut, bordée au repos, à l'accent une fois choisie. */
[data-app="guide-nego"] .gn-pilule {
  position: relative;
  flex: none;
  height: var(--gn-pilule-jour);
  padding-inline: var(--gn-espace-16);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-pilule);
  background: transparent;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: 1;
  font-weight: var(--gn-graisse-demi-gras);
  white-space: nowrap;
}

[data-app="guide-nego"] .gn-pilule--lien {
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-pilule--choisie {
  background: var(--gn-accent);
  border-color: var(--gn-accent);
  color: var(--gn-accent-inv);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-pilule:not(.gn-pilule--choisie):not(:disabled):active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-pilule--choisie:not(:disabled):active {
  filter: brightness(0.9);
}

[data-app="guide-nego"] .gn-pilule:disabled {
  background: transparent;
  border-color: var(--gn-filet-doux);
  color: var(--gn-desactive-texte);
  cursor: not-allowed;
}
</style>
