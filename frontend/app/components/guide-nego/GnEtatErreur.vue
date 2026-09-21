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
    <GnPicto nom="warn" :taille="20" />
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
  padding: var(--gn-espace-24) 0;
}

[data-app="guide-nego"] .gn-erreur .gn-picto {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-erreur__titre {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-erreur__texte {
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-erreur__sorties {
  width: 100%;
  margin-top: var(--gn-espace-4);
  display: flex;
  gap: var(--gn-espace-8);
}
</style>
