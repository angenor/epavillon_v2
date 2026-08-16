<script setup lang="ts">
import type { TemporalState } from '~/types/views'

/**
 * PASTILLE D'ÉTAT TEMPOREL — l'état d'une activité dans le temps.
 *
 * À NE PAS CONFONDRE AVEC `UiBadge`, qui est une pilule d'information générique.
 * Celle-ci est un repère d'ÉTAT : forme carrée, capitales, point de couleur en
 * tête. Ce dessin distinct est délibéré — dans une programmation dense, l'œil
 * doit séparer d'un coup ce qui décrit l'activité (thématique, format) de ce qui
 * dit où elle en est.
 *
 * SIX ÉTATS, CINQ VENUS DE LA VUE. `v_public_schedule.temporal_state` en calcule
 * cinq : `upcoming`, `ongoing`, `past`, `postponed`, `cancelled`. Le sixième —
 * EN DIRECT — n'en fait pas partie et ne peut pas en faire partie : il ne dépend
 * pas du temps mais de la diffusion. Il est ici pour que les six rendus vivent
 * au même endroit, mais son affichage réel passe par `UiLiveBadge`, qui vérifie
 * la règle métier n° 4 — un seul direct à la fois.
 *
 * LES COULEURS SUIVENT LA RÈGLE D'USAGE, et deux d'entre elles ne vont pas de soi :
 *
 * · « EN COURS » EST JAUNE, pas vert. Une activité en cours n'est pas une
 *   réussite : c'est une position dans le temps, qui appelle l'attention de
 *   l'équipe — la salle est occupée, quelqu'un parle. Le vert dirait « c'est
 *   bon, il n'y a rien à faire », ce qui est le contraire.
 *
 * · « REPORTÉE » EST VIOLETTE, pas jaune. Un report a DÉJÀ été arbitré : il ne
 *   demande plus rien à personne. Le confondre avec l'avertissement, c'est
 *   noyer dans le même signal ce qui est réglé et ce qui reste à traiter — et
 *   une alerte qui se déclenche pour du réglé finit par ne plus être lue.
 *
 * Le point de couleur double la teinte par une FORME : l'état reste lisible en
 * niveaux de gris et à l'impression.
 */

/** Les cinq états de la vue, plus le direct. */
export type SessionDisplayState = TemporalState | 'live'

interface Props {
  state: SessionDisplayState
  /** Libellé déjà traduit. */
  label: string
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })

const TONES: Record<SessionDisplayState, string> = {
  upcoming: 'text-info bg-info-surface',
  ongoing: 'text-warning bg-warning-surface',
  past: 'text-neutral bg-neutral-surface',
  postponed: 'text-postponed bg-postponed-surface',
  cancelled: 'text-danger bg-danger-surface',
  // Aplat plein : le direct doit se voir depuis l'autre bout de la page.
  live: 'text-live-contrast bg-live',
}
</script>

<template>
  <span
    class="inline-flex items-center gap-2 rounded-sm font-bold uppercase"
    :class="[TONES[props.state], props.size === 'sm' ? 'px-2 py-0.5 text-[0.6875rem]' : 'px-3 py-1 text-xs']"
    :style="{ letterSpacing: 'var(--tracking-caps)' }"
  >
    <span
      class="size-[7px] shrink-0 rounded-full bg-current"
      :class="props.state === 'live' ? 'ui-status-pulse' : ''"
      aria-hidden="true"
    />
    {{ props.label }}
  </span>
</template>

<style scoped>
/* Une pulsation lente, jamais un clignotement : au-delà de trois éclats par
   seconde, une animation devient un risque pour les personnes photosensibles
   (WCAG 2.3.1). Neutralisée par `prefers-reduced-motion` dans `main.css` — le
   point reste alors visible, fixe. */
.ui-status-pulse {
  animation: ui-status-pulse 1.6s ease-in-out infinite;
}

@keyframes ui-status-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.25;
  }
}
</style>
