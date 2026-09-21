<script setup lang="ts">
/** Dit ce qu'il n'y a pas, et quand ça viendra. Une sortie, s'il y en a une. */
withDefaults(
  defineProps<{ picto: string; titre: string; texte: string; sortie?: string; sortieVers?: string }>(),
  { sortie: undefined, sortieVers: undefined },
)
</script>

<template>
  <div class="gn-vide">
    <GnPicto :nom="picto" />
    <h2 class="gn-vide__titre">{{ titre }}</h2>
    <p class="gn-vide__texte">{{ texte }}</p>
    <NuxtLink v-if="sortie && sortieVers" :to="sortieVers" class="gn-vide__sortie">{{ sortie }}</NuxtLink>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-vide {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  padding: var(--gn-espace-32) 0;
}

[data-app="guide-nego"] .gn-vide .gn-picto {
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-vide__titre {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-vide__texte {
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-vide__sortie {
  min-height: var(--gn-cible);
  display: inline-flex;
  align-items: center;
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>
