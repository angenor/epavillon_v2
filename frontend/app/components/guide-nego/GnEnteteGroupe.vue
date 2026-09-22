<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
/**
 * Nom de groupe en capitales, filet franc dessous. À droite, au choix : un compteur,
 * une note en graisse normale — « mis à jour à 11:35 » —, ou un pictogramme.
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
[data-app="guide-nego"] .gn-groupe {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gn-espace-8);
  padding: var(--gn-espace-16) 0 6px;
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-groupe__titre {
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

[data-app="guide-nego"] .gn-groupe__note {
  font-weight: var(--gn-graisse-regulier);
}

[data-app="guide-nego"] .gn-groupe .gn-picto {
  color: var(--gn-picto-secondaire);
}
</style>
