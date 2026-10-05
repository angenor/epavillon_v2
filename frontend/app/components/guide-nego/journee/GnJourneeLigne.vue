<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'

/**
 * La ligne de l'accueil (Nuit 01 et 05) : un point d'état (changement du jour) ou une
 * pastille d'icône (se préparer, documents, lexique), un titre, une précision.
 * L'emplacement par défaut remplace la précision : l'avancement de « Ma première COP ».
 */
withDefaults(
  defineProps<{
    vers: string
    titre: string
    precision?: string
    point?: 'attention' | 'danger'
    picto?: NomDePicto
  }>(),
  { precision: undefined, point: undefined, picto: undefined },
)
</script>

<template>
  <NuxtLink :to="vers" class="gn-journee-ligne" :class="{ 'gn-journee-ligne--icone': picto }">
    <span v-if="point" class="gn-journee-ligne__point" :class="`gn-journee-ligne__point--${point}`" />
    <span v-else-if="picto" class="gn-journee-ligne__pastille"><GnPicto :nom="picto" :taille="20" /></span>
    <span class="gn-journee-ligne__corps">
      <span class="gn-journee-ligne__titre">{{ titre }}</span>
      <slot>
        <span v-if="precision" class="gn-journee-ligne__precision">{{ precision }}</span>
      </slot>
    </span>
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-journee-ligne {
  min-height: 60px;
  display: flex;
  align-items: center;
  gap: 14px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet-doux);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-journee-ligne--icone {
  min-height: 68px;
}

[data-app="guide-nego"] li:last-child > .gn-journee-ligne {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-journee-ligne:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-journee-ligne__point {
  flex: none;
  width: var(--gn-point-etat);
  height: var(--gn-point-etat);
  border-radius: var(--gn-rayon-pilule);
}

[data-app="guide-nego"] .gn-journee-ligne__point--attention {
  background: var(--gn-attention);
}

[data-app="guide-nego"] .gn-journee-ligne__point--danger {
  background: var(--gn-danger);
}

[data-app="guide-nego"] .gn-journee-ligne__pastille {
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

[data-app="guide-nego"] .gn-journee-ligne__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-journee-ligne__titre {
  font-size: var(--gn-taille-16);
  line-height: 1.3;
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-journee-ligne__precision {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}
</style>
