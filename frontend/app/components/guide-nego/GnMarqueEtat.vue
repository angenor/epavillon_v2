<script setup lang="ts">
import { couleurDEtat, dessinDEtat, type NomDEtat } from '~/utils/guide-nego/etats'

/**
 * Un état = un pictogramme + un mot + une couleur. Jamais la couleur seule.
 *
 * Le mot vient du nom de l'état, pas de l'appelant : c'est ce qui empêche d'écrire
 * « Annulée » d'une couleur qui dit autre chose. `libelle` n'existe que pour les
 * états qui portent une donnée — « Remplacé par… », « Synchronisé à 11:35 ».
 *
 * `aplat` : la pilule pleine de la maquette (« En cours »), pour l'état qui compte à
 * l'instant ; le mot y suffit, il porte le texte foncé en 800.
 */
const props = withDefaults(defineProps<{ etat: NomDEtat; libelle?: string; aplat?: boolean }>(), {
  libelle: undefined,
  aplat: false,
})

const { t } = useI18n()
const dessin = computed(() => dessinDEtat(props.etat))
const taille = computed(() => dessin.value.taille ?? 16)
const mot = computed(() => props.libelle ?? t(`gn-marque-etat.${props.etat}`))
</script>

<template>
  <span v-if="aplat" class="gn-marque-etat gn-marque-etat--aplat" :style="{ background: couleurDEtat(etat) }">
    {{ mot }}
  </span>
  <span v-else class="gn-marque-etat" :style="{ color: couleurDEtat(etat) }">
    <GnPicto v-if="dessin.picto" :nom="dessin.picto" :taille="taille" class="gn-marque-etat__picto" />
    <span v-else class="gn-marque-etat__point" aria-hidden="true" />
    {{ mot }}
  </span>
</template>

<style>
[data-app="guide-nego"] .gn-marque-etat {
  display: inline-flex;
  align-items: flex-start;
  gap: 6px;
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-gras);
}

/* Un état qui porte une donnée passe à la ligne ; le signe reste centré sur la première. */
[data-app="guide-nego"] .gn-marque-etat__picto {
  margin-block-start: calc((1em * var(--gn-interligne-13) - var(--gn-picto-16)) / 2);
}

/* « Nouveau » : le point du non-lu, jamais un pictogramme. */
[data-app="guide-nego"] .gn-marque-etat__point {
  flex: none;
  width: var(--gn-point-etat);
  height: var(--gn-point-etat);
  margin-block-start: calc((1em * var(--gn-interligne-13) - var(--gn-point-etat)) / 2);
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-attention-aplat);
}

[data-app="guide-nego"] .gn-marque-etat--aplat {
  align-items: center;
  padding: 3px 9px;
  border-radius: var(--gn-rayon-pilule);
  color: var(--gn-accent-inv);
  font-weight: var(--gn-graisse-extra-gras);
}
</style>
