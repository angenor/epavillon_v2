<script setup lang="ts">
/**
 * État vide — DEUXIÈME des quatre états d'écran.
 *
 * UN ÉTAT VIDE N'EST PAS UNE ERREUR, et deux situations très différentes se
 * cachent derrière le même écran blanc :
 *
 * · RIEN N'EXISTE ENCORE — première visite, aucun dossier déposé. L'écran doit
 *   alors expliquer quoi faire et proposer l'action : c'est le seul endroit de
 *   l'interface où un bouton principal est mérité.
 * · UN FILTRE NE RAMÈNE RIEN — quarante dossiers existent, aucun ne correspond.
 *   L'écran doit alors proposer d'élargir la recherche, JAMAIS de créer quelque
 *   chose. C'est `filtered`.
 *
 * Les confondre produit le classique « Aucun résultat — Créer une organisation »
 * affiché à quelqu'un qui a simplement mal orthographié un sigle. C'est ainsi
 * qu'on fabrique des doublons — le défaut n° 1 de la v1.
 *
 * Pas d'illustration : la direction artistique écarte les blobs et les formes
 * flottantes. Une icône en trait, un titre, une phrase, une action.
 */

interface Props {
  title?: string
  description?: string
  /** Icône de tête (nom de `UiIcon`). */
  icon?: string
  /**
   * L'écran est vide PARCE QU'UN FILTRE est actif. L'action proposée devient
   * « réinitialiser », jamais « créer ».
   */
  filtered?: boolean
  /** Libellé de l'action principale. Absent, aucun bouton n'est rendu. */
  actionLabel?: string
  /** Destination de l'action ; sans elle, l'action émet `action`. */
  actionTo?: string
  /** Version compacte, dans une carte ou une cellule de tableau. */
  compact?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ action: [] }>()

const { t } = useI18n()

const icon = computed(() => props.icon ?? (props.filtered ? 'filter' : 'document'))
</script>

<template>
  <div
    class="flex flex-col items-center rounded-lg border border-dashed border-border text-center"
    :class="props.compact ? 'px-4 py-6' : 'px-6 py-12'"
  >
    <UiIcon :name="icon" :size="props.compact ? '1.5rem' : '2rem'" class="text-text-subtle" />

    <p class="mt-3 font-display" :class="props.compact ? 'text-base' : 'text-xl'">
      {{ props.title ?? t('common.states.empty.title') }}
    </p>

    <p class="mt-1.5 max-w-prose text-sm text-text-muted">
      {{ props.description ?? t('common.states.empty.description') }}
    </p>

    <div v-if="props.actionLabel || $slots.actions" class="mt-5 flex flex-wrap justify-center gap-2">
      <slot name="actions">
        <UiButton
          v-if="props.actionLabel"
          :variant="props.filtered ? 'secondary' : 'primary'"
          :to="props.actionTo"
          :icon="props.filtered ? 'refresh' : 'plus'"
          @click="emit('action')"
        >
          {{ props.actionLabel }}
        </UiButton>
      </slot>
    </div>
  </div>
</template>
