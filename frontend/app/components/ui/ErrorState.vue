<script setup lang="ts">
/**
 * État d'erreur — TROISIÈME des quatre états d'écran.
 *
 * TROIS CHOSES, DANS CET ORDRE : ce qui s'est passé, ce que la personne peut
 * faire, et de quoi parler au support. La troisième est celle qu'on oublie : un
 * identifiant de requête (`app.request_id`, posé par l'API en tête de chaque
 * transaction) transforme un « ça ne marche pas » en un incident retrouvable
 * dans les journaux. Il est affiché en petit, sélectionnable, jamais masqué.
 *
 * LE MESSAGE TECHNIQUE N'EST PAS LE MESSAGE AFFICHÉ. Une erreur de base de
 * données n'a rien à faire sous les yeux d'une organisation : `detail` est
 * réservé au back-office et aux personnes qui peuvent en faire quelque chose.
 *
 * RÉESSAYER EST L'ACTION PAR DÉFAUT — la plupart de ces erreurs sont passagères.
 * Le bouton n'apparaît que si un écouteur `retry` est fourni : proposer de
 * réessayer une action qui échouera toujours est pire que de ne rien proposer.
 *
 * `role="alert"` : l'erreur remplace un contenu attendu, elle doit être annoncée.
 *
 * PAS DE FOND ROUGE. C'est le TRAIT qui porte l'alerte — bordure pleine en
 * `--color-danger-border` sur fond de surface ordinaire. Un aplat rouge pâle
 * derrière quatre lignes de texte, un bouton et une référence d'incident écrase
 * ce qu'on demande justement de lire, et interdit d'y poser un second bloc (le
 * détail technique) sans empiler deux teintes. Le rouge reste concentré sur le
 * glyphe et le filet, là où il distingue sans gêner.
 */

interface Props {
  title?: string
  description?: string
  /** Détail technique — back-office seulement. */
  detail?: string
  /** Identifiant de requête à communiquer au support (`app.request_id`). */
  requestId?: string
  /** Libellé du bouton de reprise. */
  retryLabel?: string
  /** Une reprise est en cours. */
  retrying?: boolean
  compact?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ retry: [] }>()

const { t } = useI18n()
const attrs = useAttrs()
/** Le bouton n'existe que si quelqu'un écoute la reprise. */
const canRetry = computed(() => Boolean(attrs.onRetry))
</script>

<template>
  <div
    role="alert"
    class="flex flex-col items-center rounded-lg border border-danger-border bg-surface text-center"
    :class="props.compact ? 'px-4 py-6' : 'px-6 py-12'"
  >
    <UiIcon name="error" :size="props.compact ? '1.5rem' : '2.75rem'" class="text-danger" />

    <p class="mt-3 font-display" :class="props.compact ? 'text-base' : 'text-xl'">
      {{ props.title ?? t('common.states.error.title') }}
    </p>

    <p class="mt-1.5 max-w-(--measure) text-sm text-text-secondary">
      {{ props.description ?? t('common.states.error.description') }}
    </p>

    <p
      v-if="props.detail"
      class="mt-3 max-w-(--measure) rounded-md border border-border bg-surface-raised px-3 py-2 text-left font-mono text-xs break-words text-text-muted"
    >
      {{ props.detail }}
    </p>

    <div v-if="canRetry || $slots.actions" class="mt-5 flex flex-wrap justify-center gap-2">
      <slot name="actions">
        <UiButton
          v-if="canRetry"
          variant="secondary"
          icon="refresh"
          :loading="props.retrying"
          @click="emit('retry')"
        >
          {{ props.retryLabel ?? t('common.states.error.action') }}
        </UiButton>
      </slot>
    </div>

    <!-- Sélectionnable : c'est fait pour être copié dans un courriel au support. -->
    <p v-if="props.requestId" class="mt-4 font-mono text-xs text-text-subtle select-all">
      {{ t('common.states.error.requestId', { id: props.requestId }) }}
    </p>
  </div>
</template>
