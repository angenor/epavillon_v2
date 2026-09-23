<script setup lang="ts">
/**
 * Un écran posé sur le lecteur — le sommaire (04 · 04), la recherche (04 · 05). Le texte
 * reste dessous, à sa page : refermer y ramène sans rien recharger ni recaler.
 */
defineProps<{ titre: string; sousTitre?: string }>()

const ouvert = defineModel<boolean>({ required: true })

const surface = useTemplateRef<HTMLElement>('surface')

const fermer = () => (ouvert.value = false)
useGnPiegeFocus(ouvert, () => surface.value, { fermer })
</script>

<template>
  <Teleport to="#gn-portail">
    <div v-if="ouvert" ref="surface" class="gn-panneau-lecteur" role="dialog" aria-modal="true" :aria-label="titre">
      <div class="gn-panneau-lecteur__colonne">
        <GnEntete :titre="titre" :sous-titre="sousTitre" retour-bouton @retour="fermer" />
        <slot />
      </div>
    </div>
  </Teleport>
</template>

<style>
[data-app="guide-nego"] .gn-panneau-lecteur {
  position: fixed;
  inset: 0;
  z-index: 20;
  overflow-y: auto;
  overscroll-behavior: contain;
  background: var(--gn-fond);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-panneau-lecteur__colonne {
  max-width: var(--gn-colonne-largeur);
  margin-inline: auto;
  padding: var(--gn-espace-12) var(--gn-marge-ecran) calc(var(--gn-espace-24) + env(safe-area-inset-bottom));
}
</style>
