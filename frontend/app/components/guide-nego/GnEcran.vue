<script setup lang="ts">
/**
 * Le cadre commun : en-tête, contenu, et la barre d'onglets quand l'écran en porte une.
 * Un écran secondaire — lexique, réglages — donne un `retour` et se passe d'onglets.
 *
 * Les zones sûres et les marges viennent de `.gn-ecran` (mesures.css) ; rien ici ne
 * les redit.
 */
withDefaults(
  defineProps<{
    titre: string
    sousTitre?: string
    retour?: string
    onglets?: boolean
    lexiqueOuvert?: boolean
  }>(),
  { sousTitre: undefined, retour: undefined, onglets: true, lexiqueOuvert: false },
)

const { echangesOuverts } = useGnDrapeaux()
</script>

<template>
  <div class="gn-ecran" :class="{ 'gn-ecran--sans-onglets': !onglets }">
    <GnEntete :titre="titre" :sous-titre="sousTitre" :retour="retour" :lexique-ouvert="lexiqueOuvert">
      <template #connexion><slot name="connexion" /></template>
    </GnEntete>
    <main class="gn-ecran__contenu">
      <slot />
    </main>
    <GnBarreOnglets v-if="onglets" :echanges-ouverts="echangesOuverts" />
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-ecran {
  flex: 1;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-ecran__contenu {
  flex: 1;
}
</style>
