<script setup lang="ts">
/** Les points d'une ligne du déroulé (« Mot d'ouverture, OIF »), puces à la couleur de la journée. */

interface Props {
  points: string[]
}

const props = defineProps<Props>()
</script>

<template>
  <ul v-if="props.points.length" class="mt-2 grid gap-0.5 text-sm text-text-secondary">
    <li
      v-for="point in props.points"
      :key="point"
      class="relative pl-4 before:absolute before:top-[0.6em] before:left-0.5 before:size-1.5 before:rounded-full before:bg-(--color-day-line)"
    >
      {{ point }}
    </li>
  </ul>
</template>
