<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
/**
 * Titre de section. À droite, au choix : un compteur, une note — « à 10:12 » —, ou un
 * pictogramme.
 */
withDefaults(
  defineProps<{ titre: string; compteur?: number | string; note?: string; picto?: NomDePicto }>(),
  { compteur: undefined, note: undefined, picto: undefined },
)
</script>

<template>
  <h2 class="gn-groupe">
    <span class="gn-groupe__titre">{{ titre }}</span>
    <span v-if="compteur !== undefined" class="gn-groupe__compteur">{{ compteur }}</span>
    <span v-else-if="note" class="gn-groupe__note">{{ note }}</span>
    <GnPicto v-else-if="picto" :nom="picto" :taille="20" />
  </h2>
</template>

<style>
/* Le titre de section de la maquette : Sora 18/700, note à droite en 13. */
[data-app="guide-nego"] .gn-groupe {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--gn-espace-8);
  padding: var(--gn-entre-blocs) 0 var(--gn-espace-8);
  color: var(--gn-titre);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-18);
  line-height: var(--gn-interligne-18);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-groupe__compteur,
[data-app="guide-nego"] .gn-groupe__note {
  flex: none;
  font-family: var(--gn-police);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-regulier);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-groupe .gn-picto {
  align-self: center;
  color: var(--gn-picto-secondaire);
}
</style>
