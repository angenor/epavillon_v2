<script setup lang="ts">
/**
 * Indicateur d'activité — le chargement d'un contrôle, pas celui d'un écran.
 *
 * Pour un écran entier, c'est `UiSkeletonLoader` qu'il faut : un squelette
 * annonce la FORME de ce qui arrive, là où un tourniquet ne dit rien. Celui-ci
 * sert dans un bouton qui travaille, un champ qui interroge le référentiel, une
 * ligne de tableau qui se rafraîchit.
 *
 * ACCESSIBILITÉ : le composant est décoratif par défaut (`aria-hidden`) — c'est
 * son contexte qui annonce l'attente, par `aria-busy` sur le bouton ou par un
 * texte visible. Avec `label`, il devient un `status` annoncé par les lecteurs
 * d'écran, à n'utiliser que s'il est seul à porter l'information.
 *
 * `prefers-reduced-motion` neutralise l'animation dans `main.css` ; le cercle
 * reste alors visible, sans tourner.
 */

interface Props {
  /** Taille CSS ; par défaut la taille de police du contexte. */
  size?: string
  /** Nom accessible. Absent, le composant est purement décoratif. */
  label?: string
}

const props = withDefaults(defineProps<Props>(), { size: '1em' })
</script>

<template>
  <span
    class="inline-flex items-center gap-2"
    :role="props.label ? 'status' : undefined"
    :aria-hidden="props.label ? undefined : true"
  >
    <svg
      :width="props.size"
      :height="props.size"
      viewBox="0 0 24 24"
      fill="none"
      class="ui-spinner shrink-0"
      aria-hidden="true"
      focusable="false"
    >
      <!-- Anneau de fond : sans lui, l'arc semble sauter d'un tour à l'autre. -->
      <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="2.4" opacity="0.22" />
      <path
        d="M21 12a9 9 0 0 0-9-9"
        stroke="currentColor"
        stroke-width="2.4"
        stroke-linecap="round"
      />
    </svg>
    <span v-if="props.label" class="sr-only">{{ props.label }}</span>
  </span>
</template>

<style scoped>
.ui-spinner {
  animation: ui-spin 750ms linear infinite;
}

@keyframes ui-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
