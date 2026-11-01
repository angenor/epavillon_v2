<script setup lang="ts">
/**
 * Le billet : la carte levée qui remonte sur le bandeau d'une page de la
 * programmation. Sa tête est un filet (`barClass` en change la couleur) ou,
 * par le créneau `band`, une bande pleine (la date en aplat, direction
 * condensée). Le corps peut rétrécir : une page qui colle le billet borne sa
 * hauteur et laisse défiler ce qui dépasse.
 */

interface Props {
  label: string
  barClass?: string
}

const props = withDefaults(defineProps<Props>(), { barClass: 'bg-accent' })
</script>

<template>
  <aside class="flex flex-col overflow-hidden rounded-lg bg-surface-raised text-text shadow-lg" :aria-label="props.label">
    <slot name="band">
      <div class="h-1 shrink-0" :class="props.barClass" aria-hidden="true" />
    </slot>
    <div class="flex min-h-0 flex-col p-6">
      <slot />
    </div>
    <slot name="outside" />
  </aside>
</template>
