<script setup lang="ts">
/**
 * Dit ce qui a échoué, puis CE QUI RESTE VRAI — en salle, la phrase qui sauve n'est
 * pas l'échec mais l'heure des données encore affichées.
 *
 * Deux sorties à demi-largeur : la secondaire d'abord, l'action pleine ensuite, comme
 * dans la maquette. Chacune mène ailleurs (`…Vers`) ou émet son événement ; les
 * libellés viennent toujours de l'appelant, qui seul sait nommer l'agenda concerné.
 */
withDefaults(
  defineProps<{
    titre: string
    texte: string
    sortie?: string
    sortieVers?: string
    sortieSecondaire?: string
    sortieSecondaireVers?: string
  }>(),
  {
    sortie: undefined,
    sortieVers: undefined,
    sortieSecondaire: undefined,
    sortieSecondaireVers: undefined,
  },
)

defineEmits<{ sortie: []; sortieSecondaire: [] }>()
</script>

<template>
  <div class="gn-erreur" role="alert">
    <span class="gn-erreur__pastille" aria-hidden="true"><GnPicto nom="warn" :taille="20" /></span>
    <h2 class="gn-erreur__titre">{{ titre }}</h2>
    <p class="gn-erreur__texte">{{ texte }}</p>
    <div v-if="sortie || sortieSecondaire" class="gn-erreur__sorties">
      <GnBouton
        v-if="sortieSecondaire"
        variante="secondaire"
        largeur="demie"
        :vers="sortieSecondaireVers"
        @clic="$emit('sortieSecondaire')"
      >
        {{ sortieSecondaire }}
      </GnBouton>
      <GnBouton
        v-if="sortie"
        variante="principal"
        largeur="demie"
        :vers="sortieVers"
        @clic="$emit('sortie')"
      >
        {{ sortie }}
      </GnBouton>
    </div>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-erreur {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  padding: 18px var(--gn-espace-16);
  border-radius: var(--gn-rayon-24);
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-erreur__pastille {
  flex: none;
  width: var(--gn-pastille-icone);
  height: var(--gn-pastille-icone);
  margin-block-end: var(--gn-espace-4);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-14);
  background: var(--gn-bloc-releve);
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-erreur__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-erreur__texte {
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-erreur__sorties {
  width: 100%;
  margin-top: var(--gn-espace-8);
  display: flex;
  gap: var(--gn-espace-8);
}
</style>
