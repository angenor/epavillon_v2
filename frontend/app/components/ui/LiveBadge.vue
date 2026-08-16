<script setup lang="ts">
import type { SessionId } from '~/types/shared'

/**
 * PASTILLE « EN DIRECT » — et sa règle d'usage.
 *
 * RÈGLE MÉTIER N° 4 : UN SEUL DIRECT À LA FOIS, tous événements confondus. Une
 * seule équipe technique, un seul flux. Cette pastille ne peut donc apparaître
 * que sur UNE carte de toute la plateforme, à un instant donné.
 *
 * LA RÈGLE EST TENUE PAR CONSTRUCTION, pas par discipline : le composant
 * interroge `useLiveSession()` et ne rend RIEN si la séance qu'on lui donne
 * n'est pas celle qui est déclarée en direct. Une carte ne décide jamais seule
 * d'afficher son repère — c'est ce qui évite le cas banal de deux séances
 * restées marquées « live » en base après leur heure de fin.
 *
 * `force` court-circuite le registre : réservé au guide de style et aux
 * démonstrations, jamais à un écran réel. C'est écrit ici pour que la
 * dérogation reste visible.
 *
 * TROIS SIGNAUX, comme pour toute information portée par la couleur : un point
 * rouge qui pulse, le mot « En direct », et une phrase pour les lecteurs
 * d'écran. Le point seul serait invisible en niveaux de gris, et le rouge seul
 * indistinguable d'une annulation.
 *
 * `aria-live="polite"` : le passage en direct est annoncé sans interrompre la
 * lecture en cours.
 */

interface Props {
  /** Séance concernée. Le repère n'apparaît que si c'est ELLE qui est en direct. */
  sessionId?: SessionId | null
  /** Ignore le registre — démonstration seulement. */
  force?: boolean
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })

const { t } = useI18n()
const { isLive } = useLiveSession()

const visible = computed(() => props.force || isLive(props.sessionId))
</script>

<template>
  <span
    v-if="visible"
    class="inline-flex items-center gap-1.5 rounded-full border border-danger-border bg-danger-surface font-semibold tracking-wide text-danger uppercase"
    :class="props.size === 'sm' ? 'px-2 py-0.5 text-[0.6875rem]' : 'px-2.5 py-1 text-xs'"
    aria-live="polite"
  >
    <span class="ui-live-dot size-2 shrink-0 rounded-full bg-danger-solid" aria-hidden="true" />
    {{ t('live-badge.label') }}
    <span class="sr-only">— {{ t('live-badge.description') }}</span>
  </span>
</template>

<style scoped>
/* Une pulsation lente, pas un clignotement : au-delà de trois éclats par
   seconde, une animation devient un risque pour les personnes photosensibles
   (WCAG 2.3.1). `prefers-reduced-motion` la neutralise entièrement — le point
   reste alors visible, fixe. */
.ui-live-dot {
  animation: ui-live-pulse 2s ease-in-out infinite;
}

@keyframes ui-live-pulse {
  0%,
  100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.45;
    transform: scale(0.82);
  }
}
</style>
