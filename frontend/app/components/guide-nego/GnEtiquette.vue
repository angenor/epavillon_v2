<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
/**
 * Une étiquette bordée : d'où vient une donnée, ce qui l'a produite — « Source
 * officielle — lu à 11:35 », « Traduction automatique », « Suggéré par l'IA ».
 *
 * Elle ne dit pas un état : pas de couleur de rôle, pas d'aplat. C'est une mention
 * discrète, et son texte vient de l'écran qui la pose.
 */
defineProps<{ texte: string; picto?: NomDePicto }>()
</script>

<template>
  <span class="gn-etiquette">
    <GnPicto v-if="picto" :nom="picto" :taille="16" />
    {{ texte }}
  </span>
</template>

<style>
[data-app="guide-nego"] .gn-etiquette {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 9px;
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-pilule);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte-2);
}
</style>
