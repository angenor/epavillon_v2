<script setup lang="ts">
/** Nom de groupe en capitales, compteur facultatif, filet franc dessous. */
withDefaults(defineProps<{ titre: string; compteur?: number | string; picto?: string }>(), {
  compteur: undefined,
  picto: undefined,
})
</script>

<template>
  <h2 class="gn-groupe">
    <span class="gn-groupe__titre">{{ titre }}</span>
    <span v-if="compteur !== undefined" class="gn-groupe__compteur">{{ compteur }}</span>
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

[data-app="guide-nego"] .gn-groupe .gn-picto {
  color: var(--gn-picto-secondaire);
}
</style>
