<script setup lang="ts">
/**
 * État de chargement — PREMIER des quatre états d'écran, et le seul qui manquait
 * comme composant de plein droit.
 *
 * Il existait des squelettes (`UiSkeletonLoader`) mais pas d'ÉTAT : chaque écran
 * réinventait sa disposition d'attente, son `aria-busy` et — surtout — son
 * absence de garde-fou. D'où ce composant, symétrique de `UiEmptyState`,
 * `UiErrorState` et `UiForbiddenState`.
 *
 * DEUX RÈGLES DU GUIDE, ET ELLES SONT TOUTES DEUX IMPLÉMENTÉES ICI :
 *
 * 1. « SQUELETTES CALQUÉS SUR LA FORME DU CONTENU ATTENDU — JAMAIS UN ROND QUI
 *    TOURNE AU CENTRE D'UNE PAGE VIDE. » Un tourniquet ne dit rien de ce qui
 *    arrive, et la page se recompose d'un coup quand les données tombent. Les
 *    dispositions de `variant` couvrent les formes courantes ; dès qu'un écran a
 *    une silhouette qui lui est propre, il compose la sienne dans le créneau par
 *    défaut plutôt que de tordre une disposition générique. C'est le cas normal,
 *    pas l'exception.
 *
 * 2. « AU-DELÀ DE DIX SECONDES, LE SQUELETTE CÈDE LA PLACE À L'ÉTAT D'ERREUR. »
 *    Un squelette qui balaie indéfiniment ment : il promet un contenu qui
 *    n'arrivera pas, et laisse l'utilisateur attendre au lieu de réessayer ou
 *    d'écrire au support. Passé `timeoutMs`, l'événement `timeout` est émis —
 *    à l'écran d'afficher alors `UiErrorState`. Le composant ne se substitue PAS
 *    lui-même : seul l'appelant sait s'il faut abandonner la requête, la
 *    relancer, ou quel identifiant de requête montrer.
 *
 * ACCESSIBILITÉ : `aria-busy="true"` sur le conteneur et un libellé lu une seule
 * fois. Les squelettes eux-mêmes sont `aria-hidden` — annoncer douze blocs
 * reviendrait à lire douze fois « chargement ».
 */

/** Silhouettes courantes. Au-delà, on compose dans le créneau par défaut. */
type LoadingVariant = 'text' | 'card' | 'list' | 'table' | 'form'

interface Props {
  /** Silhouette attendue. Ignorée si le créneau par défaut est fourni. */
  variant?: LoadingVariant
  /** Nombre de lignes, de cartes ou de rangées, selon la silhouette. */
  lines?: number
  /**
   * Libellé annoncé aux lecteurs d'écran. À défaut, le libellé générique — un
   * écran qui sait ce qu'il charge gagne à le dire (« chargement du programme »).
   */
  label?: string
  /**
   * Délai au terme duquel `timeout` est émis. `0` désactive la garde — à ne
   * faire que pour une attente réellement sans fin (un direct qui va démarrer),
   * jamais pour un appel d'API.
   */
  timeoutMs?: number
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'text',
  lines: 3,
  timeoutMs: 10_000,
})

const emit = defineEmits<{
  /** Dix secondes écoulées : à l'appelant de basculer sur `UiErrorState`. */
  timeout: []
}>()

const { t } = useI18n()

/**
 * `setTimeout` posé au montage seulement : côté serveur, il n'y a pas d'attente
 * à mesurer, et un minuteur non nettoyé y fuirait à chaque rendu.
 */
let timer: ReturnType<typeof setTimeout> | null = null

onMounted(() => {
  if (props.timeoutMs <= 0) return
  timer = setTimeout(() => emit('timeout'), props.timeoutMs)
})

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})
</script>

<template>
  <div :aria-busy="true" :aria-label="props.label ?? t('common.states.loading.label')" role="status">
    <!-- Le créneau prime : une silhouette sur mesure vaut toujours mieux qu'une
         disposition générique approchante. -->
    <slot>
      <!-- Texte — titre, paragraphe, jetons, bloc. La forme d'une fiche lue. -->
      <div v-if="props.variant === 'text'" class="flex flex-col gap-3">
        <UiSkeletonLoader width="52%" height="1.25rem" />
        <UiSkeletonLoader variant="text" :lines="props.lines" />
      </div>

      <!-- Carte — grille de vignettes de session, de la hauteur d'une vraie. -->
      <div v-else-if="props.variant === 'card'" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <UiSkeletonLoader v-for="index in props.lines" :key="index" height="12rem" />
      </div>

      <!-- Liste — une pastille et deux lignes par entrée. -->
      <ul v-else-if="props.variant === 'list'" class="flex flex-col">
        <li
          v-for="index in props.lines"
          :key="index"
          class="flex items-start gap-3 border-b border-border-subtle py-3 last:border-0"
        >
          <UiSkeletonLoader variant="circle" width="2rem" height="2rem" />
          <div class="min-w-0 flex-1 space-y-2">
            <UiSkeletonLoader width="60%" height="0.9rem" />
            <UiSkeletonLoader width="35%" height="0.75rem" />
          </div>
        </li>
      </ul>

      <!-- Tableau — largeurs volontairement inégales : des barres parfaitement
           alignées ne ressemblent à aucun tableau réel. -->
      <div v-else-if="props.variant === 'table'" class="rounded-lg border border-border bg-surface-raised">
        <div
          v-for="index in props.lines"
          :key="index"
          class="flex items-center gap-4 border-b border-border-subtle px-3 py-3 last:border-0"
        >
          <UiSkeletonLoader :width="index % 2 === 0 ? '38%' : '46%'" height="0.9rem" />
          <UiSkeletonLoader :width="index % 3 === 0 ? '22%' : '28%'" height="0.9rem" />
          <UiSkeletonLoader width="3rem" height="0.9rem" />
        </div>
      </div>

      <!-- Formulaire — libellé court, champ pleine largeur, et un bouton en pied. -->
      <div v-else class="flex flex-col gap-5">
        <div v-for="index in props.lines" :key="index" class="space-y-2">
          <UiSkeletonLoader width="30%" height="0.85rem" />
          <UiSkeletonLoader width="100%" height="2.75rem" />
        </div>
        <UiSkeletonLoader width="9rem" height="2.75rem" />
      </div>
    </slot>
  </div>
</template>
