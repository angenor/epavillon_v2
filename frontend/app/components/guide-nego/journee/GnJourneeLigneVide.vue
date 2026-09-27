<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'

/** Un agenda sans rien aujourd'hui, dans « Ma journée » (07 · 08) : il garde sa ligne et son nom (ADR-008). */
defineProps<{
  picto: NomDePicto
  origine: string
  texte: string
  vers: string
}>()
</script>

<template>
  <NuxtLink :to="vers" class="gn-journee-ligne-vide">
    <span class="gn-journee-ligne-vide__picto"><GnPicto :nom="picto" :taille="24" /></span>
    <span class="gn-journee-ligne-vide__corps">
      <span class="gn-journee-ligne-vide__origine">{{ origine }}</span>
      <span class="gn-journee-ligne-vide__texte">{{ texte }}</span>
    </span>
    <GnPicto nom="chevron" :taille="24" class="gn-journee-ligne-vide__chevron" />
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-journee-ligne-vide {
  min-height: var(--gn-cible);
  padding-block: var(--gn-ligne-air);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte-2);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-journee-ligne-vide:active {
  background: var(--gn-presse);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-journee-ligne-vide:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-journee-ligne-vide__picto {
  flex: none;
  width: var(--gn-colonne-heure);
  align-self: center;
  display: flex;
  color: var(--gn-picto-secondaire);
}

[data-app="guide-nego"] .gn-journee-ligne-vide__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-journee-ligne-vide__origine {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-journee-ligne-vide__texte {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-journee-ligne-vide__chevron {
  flex: none;
  color: var(--gn-picto-secondaire);
}
</style>
