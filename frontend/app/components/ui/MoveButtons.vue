<script setup lang="ts">
/**
 * Monter ou descendre un élément d'une liste ordonnée. Le clavier et le
 * tactile y arrivent sans geste à apprendre, ce que le glisser-déposer n'offre pas.
 */

const props = defineProps<{
  /** Nom de l'élément, lu par les lecteurs d'écran : « Monter Matin ». */
  item: string
  first?: boolean
  last?: boolean
  disabled?: boolean
}>()

const emit = defineEmits<{ up: []; down: [] }>()

const { t } = useI18n()
</script>

<template>
  <div class="flex shrink-0 items-center gap-1">
    <UiButton
      variant="ghost"
      icon="chevron-up"
      icon-only
      :label="t('common.actions.moveUp', { item: props.item })"
      :disabled="props.disabled || props.first"
      @click="emit('up')"
    />
    <UiButton
      variant="ghost"
      icon="chevron-down"
      icon-only
      :label="t('common.actions.moveDown', { item: props.item })"
      :disabled="props.disabled || props.last"
      @click="emit('down')"
    />
  </div>
</template>
