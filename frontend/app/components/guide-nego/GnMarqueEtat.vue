<script setup lang="ts">
import { couleurDEtat, dessinDEtat, type NomDEtat } from '~/utils/guide-nego/etats'

/**
 * Un état = un pictogramme + un mot + une couleur. Jamais la couleur seule, jamais
 * un fond coloré derrière le mot.
 *
 * Le mot vient du nom de l'état, pas de l'appelant : c'est ce qui empêche d'écrire
 * « Annulée » d'une couleur qui dit autre chose. `libelle` n'existe que pour les
 * états qui portent une donnée — « Remplacé par… », « Synchronisé à 11:35 ».
 */
const props = defineProps<{ etat: NomDEtat; libelle?: string }>()

const { t } = useI18n()

const dessin = computed(() => dessinDEtat(props.etat))
const taille = computed(() => dessin.value.taille ?? 20)
const mot = computed(() => props.libelle ?? t(`gn-marque-etat.${props.etat}`))
</script>

<template>
  <span
    class="gn-marque-etat"
    :class="`gn-marque-etat--picto-${taille}`"
    :style="{ color: couleurDEtat(etat) }"
  >
    <GnPicto v-if="dessin.picto" :nom="dessin.picto" :taille="taille" class="gn-marque-etat__picto" />
    <span v-else class="gn-marque-etat__carre" aria-hidden="true" />
    {{ mot }}
  </span>
</template>

<style>
[data-app="guide-nego"] .gn-marque-etat {
  display: inline-flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

/* Un état qui porte une donnée passe à la ligne ; le signe reste centré sur la première. */
[data-app="guide-nego"] .gn-marque-etat--picto-20 .gn-marque-etat__picto {
  margin-block-start: calc((1em * var(--gn-interligne-15) - var(--gn-picto-marque)) / 2);
}

[data-app="guide-nego"] .gn-marque-etat--picto-18 .gn-marque-etat__picto {
  margin-block-start: calc((1em * var(--gn-interligne-15) - var(--gn-picto-18)) / 2);
}

[data-app="guide-nego"] .gn-marque-etat--picto-16 .gn-marque-etat__picto {
  margin-block-start: calc((1em * var(--gn-interligne-15) - var(--gn-picto-16)) / 2);
}

/* « Nouveau » : le carré jaune du non-lu, jamais un pictogramme. */
[data-app="guide-nego"] .gn-marque-etat__carre {
  flex: none;
  width: var(--gn-carre-non-lu);
  height: var(--gn-carre-non-lu);
  margin-block-start: calc((1em * var(--gn-interligne-15) - var(--gn-carre-non-lu)) / 2);
  background: var(--gn-attention-aplat);
}
</style>
