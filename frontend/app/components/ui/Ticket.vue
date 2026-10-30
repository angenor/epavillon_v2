<script setup lang="ts">
/**
 * Le billet : la carte levée qui remonte sur le bandeau d'une page de la
 * programmation, avec son filet de tête. `barClass` en change la couleur
 * (accent pour une activité, couleur de la journée pour une journée spéciale).
 */

interface Props {
  label: string
  barClass?: string
}

const props = withDefaults(defineProps<Props>(), { barClass: 'bg-accent' })
</script>

<template>
  <aside class="overflow-hidden rounded-lg bg-surface-raised text-text shadow-lg" :aria-label="props.label">
    <div class="h-1" :class="props.barClass" aria-hidden="true" />
    <div class="flex flex-col p-6">
      <slot />
    </div>
    <slot name="outside" />
  </aside>
</template>
