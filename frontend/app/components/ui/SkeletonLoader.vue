<script setup lang="ts">
/**
 * Squelette de chargement — PREMIER des quatre états d'écran.
 *
 * POURQUOI UN SQUELETTE PLUTÔT QU'UN TOURNIQUET. Un squelette annonce la FORME
 * de ce qui arrive : trois lignes de titre, une grille de six cartes, douze
 * lignes de tableau. La page ne saute pas quand les données arrivent, et
 * l'attente paraît plus courte parce qu'elle est renseignée. Un tourniquet
 * centré ne dit rien et laisse la mise en page se recomposer d'un coup.
 *
 * DIMENSIONS DONNÉES PAR L'APPELANT. Un squelette qui ne ressemble pas à ce
 * qu'il remplace est pire que rien : il déplace le contenu au moment du
 * remplacement, exactement ce qu'il devait éviter.
 *
 * ACCESSIBILITÉ : le squelette est décoratif (`aria-hidden`), et c'est le
 * conteneur qui porte `aria-busy="true"` — `UiLoadingState` s'en charge.
 * Annoncer douze squelettes reviendrait à lire douze fois « chargement ».
 *
 * UN BALAYAGE, PAS UNE PULSATION. Le bloc ne clignote pas : une bande claire le
 * traverse. La différence n'est pas cosmétique — une pulsation d'opacité fait
 * respirer TOUTE la page au même rythme, ce qui attire l'œil au lieu de le
 * laisser lire, et rend illisible ce qui est déjà arrivé. Le balayage reste
 * local au bloc et suggère un mouvement de remplissage.
 *
 * PAS DE BORDURE. Un squelette n'est pas un cadre vide en attente : c'est la
 * silhouette du contenu. Une bordure lui donnerait un contour que le texte
 * remplaçant n'aura pas, et la substitution se verrait.
 *
 * `prefers-reduced-motion` : plus d'animation du tout, le voile reste posé à
 * 40 % sans transformation. Un balayage ralenti reste un mouvement.
 */

interface Props {
  /** Largeur CSS. Un pourcentage donne l'aspect irrégulier d'un vrai texte. */
  width?: string
  height?: string
  /** Nombre de lignes ; la dernière est raccourcie, comme un paragraphe réel. */
  lines?: number
  /** Forme : bloc rectangulaire, ligne de texte, pastille ronde. */
  variant?: 'block' | 'text' | 'circle'
  rounded?: string
}

const props = withDefaults(defineProps<Props>(), {
  width: '100%',
  lines: 1,
  variant: 'block',
})

const defaultHeight = computed(() => (props.variant === 'text' ? '0.85rem' : '1.25rem'))

const radius = computed(() => {
  if (props.rounded) return props.rounded
  if (props.variant === 'circle') return 'var(--radius-full)'
  if (props.variant === 'text') return 'var(--radius-sm)'
  return 'var(--radius-md)'
})
</script>

<template>
  <div v-if="props.lines > 1" class="space-y-2" aria-hidden="true">
    <span
      v-for="line in props.lines"
      :key="line"
      class="ui-skeleton block"
      :style="{
        // La dernière ligne est plus courte : un paragraphe ne finit jamais au
        // ras de la marge.
        width: line === props.lines ? '65%' : props.width,
        height: props.height ?? defaultHeight,
        borderRadius: radius,
      }"
    />
  </div>

  <span
    v-else
    class="ui-skeleton block"
    aria-hidden="true"
    :style="{
      width: props.width,
      height: props.height ?? defaultHeight,
      borderRadius: radius,
    }"
  />
</template>

<style scoped>
.ui-skeleton {
  background-color: var(--color-neutral-surface);
  position: relative;
  /* Le voile est translaté au-delà des deux bords : sans découpe, il déborderait
     du bloc et balaierait ce qui l'entoure. */
  overflow: hidden;
}

.ui-skeleton::after {
  content: "";
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--color-surface) 55%, transparent);
  transform: translateX(-100%);
  animation: ui-skeleton-sweep 1.4s ease-in-out infinite;
}

@keyframes ui-skeleton-sweep {
  to {
    transform: translateX(100%);
  }
}

@media (prefers-reduced-motion: reduce) {
  .ui-skeleton::after {
    animation: none;
    opacity: 0.4;
    transform: none;
  }
}
</style>
