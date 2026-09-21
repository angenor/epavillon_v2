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
    <GnPicto v-if="picto" :nom="picto" :taille="18" />
    {{ texte }}
  </span>
</template>

<style>
[data-app="guide-nego"] .gn-etiquette {
  display: inline-flex;
  align-items: center;
  gap: var(--gn-espace-8);
  padding: var(--gn-etiquette-air);
  border: var(--gn-filet-1) solid var(--gn-filet);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}
</style>
