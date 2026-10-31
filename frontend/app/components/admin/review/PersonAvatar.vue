<script setup lang="ts">
import type { AttachedImage } from '~/types/media'

/** La photo d'une personne, à défaut ses initiales. */

const props = withDefaults(defineProps<{ name: string; photo: AttachedImage | null; size?: string }>(), {
  size: '2.75rem',
})
</script>

<template>
  <span class="inline-flex shrink-0 overflow-hidden rounded-full" :style="{ width: props.size, height: props.size }">
    <UiImage v-if="props.photo" :image="props.photo" ratio="1 / 1" sizes="48px" rounded="rounded-full" class="size-full" />
    <span
      v-else
      class="inline-flex size-full items-center justify-center bg-surface-sunken text-sm font-semibold text-text-secondary"
      aria-hidden="true"
    >
      {{ initialsOf(props.name) }}
    </span>
  </span>
</template>
