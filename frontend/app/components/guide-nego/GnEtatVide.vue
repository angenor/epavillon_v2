<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
/** Dit ce qu'il n'y a pas, et quand ça viendra. Une sortie, s'il y en a une. */
withDefaults(
  defineProps<{ picto: NomDePicto; titre: string; texte: string; sortie?: string; sortieVers?: string }>(),
  { sortie: undefined, sortieVers: undefined },
)
</script>

<template>
  <div class="gn-vide">
    <span class="gn-vide__pastille" aria-hidden="true"><GnPicto :nom="picto" :taille="20" /></span>
    <h2 class="gn-vide__titre">{{ titre }}</h2>
    <p class="gn-vide__texte">{{ texte }}</p>
    <GnBouton v-if="sortie && sortieVers" variante="secondaire" :vers="sortieVers" class="gn-vide__sortie">{{ sortie }}</GnBouton>
  </div>
</template>

<style>
/* Les états vide, erreur, verrou et lecture impossible partagent ce bloc. */
[data-app="guide-nego"] .gn-vide {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  padding: 18px var(--gn-espace-16);
  border-radius: var(--gn-rayon-24);
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-vide__pastille {
  flex: none;
  width: var(--gn-pastille-icone);
  height: var(--gn-pastille-icone);
  margin-block-end: var(--gn-espace-4);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-14);
  background: var(--gn-bloc-releve);
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-vide__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-vide__texte {
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-vide__sortie {
  margin-block-start: var(--gn-espace-8);
}
</style>
