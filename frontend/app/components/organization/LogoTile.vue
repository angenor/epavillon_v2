<script setup lang="ts">
/**
 * Le logo d'une organisation sur une tuile blanche, dans les deux thèmes : un
 * logo est dessiné pour un fond clair. Sans logo, le sigle, à défaut le nom.
 */

interface Props {
  name: string
  acronym?: string | null
  /** Adresse de l'image, vignette de préférence. */
  logo?: string | null
  /** « Soutien financier », sous le nom quand il n'y a pas de logo. */
  role?: string
  compact?: boolean
}

const props = withDefaults(defineProps<Props>(), { acronym: null, logo: null, role: '', compact: false })
</script>

<template>
  <div
    class="flex h-full flex-col items-center justify-center rounded-lg border border-border-subtle bg-surface-inverse-selected p-3 text-center leading-snug text-text-on-inverse-selected"
    :class="props.compact ? 'min-h-14' : 'min-h-18'"
    :title="props.name"
  >
    <img v-if="props.logo" :src="props.logo" :alt="props.name" class="max-h-10 max-w-full object-contain" loading="lazy">
    <template v-else>
      <b class="text-sm font-bold">{{ props.acronym ?? props.name }}</b>
      <span v-if="props.acronym && !props.compact" class="mt-0.5 line-clamp-2 text-xs">{{ props.name }}</span>
      <span v-else-if="props.role && props.compact" class="sr-only">{{ props.role }}</span>
    </template>
  </div>
</template>
