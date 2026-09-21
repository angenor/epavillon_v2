<script setup lang="ts">
/**
 * L'attente sous ses deux formes : l'arc quand on ignore ce qui vient, le squelette
 * quand la forme de la liste est déjà connue.
 *
 * Un arc, et non un disque qui tourne : sous « réduire les animations » tout s'immobilise,
 * et seul un quart de cercle ouvert reste reconnaissable comme un indicateur à l'arrêt.
 * Les blocs du squelette sont décoratifs — la région vivante les annonce une seule fois.
 *
 * `decoratif` retire cette région : à poser quand l'hôte annonce déjà l'attente, par
 * exemple un bouton en `aria-busy`. Deux annonces pour une seule attente en font une
 * de trop, et un `role="status"` dans un `<button>` est un nid vivant dans une commande.
 */
withDefaults(
  defineProps<{
    forme?: 'arc' | 'squelette'
    taille?: 20 | 24
    lignes?: number
    derniereLargeur?: string
    libelle?: string
    decoratif?: boolean
  }>(),
  { forme: 'arc', taille: 24, lignes: 3, derniereLargeur: '60%', libelle: undefined, decoratif: false },
)

const { t } = useI18n()
</script>

<template>
  <span
    v-if="forme === 'arc'"
    class="gn-chargement gn-chargement--arc"
    :role="decoratif ? undefined : 'status'"
    :aria-label="decoratif ? undefined : (libelle ?? t('gn-chargement.en-cours'))"
    :aria-hidden="decoratif ? 'true' : undefined"
  >
    <!-- 2,5 px : le seul trait du système, aucun jeton ne le porte. -->
    <svg
      :width="taille"
      :height="taille"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2.5"
      stroke-linecap="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M12 3a9 9 0 0 1 9 9" />
    </svg>
  </span>
  <div
    v-else
    class="gn-chargement gn-chargement--squelette"
    :role="decoratif ? undefined : 'status'"
    :aria-label="decoratif ? undefined : (libelle ?? t('gn-chargement.en-cours'))"
    :aria-hidden="decoratif ? 'true' : undefined"
  >
    <span
      v-for="ligne in lignes"
      :key="ligne"
      class="gn-chargement__bloc"
      :style="ligne === lignes ? { width: derniereLargeur } : undefined"
      aria-hidden="true"
    />
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-chargement--arc {
  flex: none;
  display: inline-flex;
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-chargement--arc svg {
  display: block;
  animation: gn-spin var(--gn-duree-arc) linear infinite;
}

[data-app="guide-nego"] .gn-chargement--squelette {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-chargement__bloc {
  width: 100%;
  height: var(--gn-squelette-hauteur);
  background: var(--gn-squelette);
  animation: gn-pulse var(--gn-duree-squelette) ease-in-out infinite;
}
</style>
