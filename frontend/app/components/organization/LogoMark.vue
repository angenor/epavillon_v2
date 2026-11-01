<script setup lang="ts">
import type { AttachedImage } from '~/types/media'

/**
 * Le logo d'une organisation devant son nom, dans une ligne de programme : une
 * petite tuile blanche (un logo est dessiné pour un fond clair), le sigle à
 * défaut de logo. Décoratif : le nom est écrit à côté.
 */

interface Props {
  name: string
  acronym?: string | null
  logo?: AttachedImage | null
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { acronym: null, logo: null, size: 'md' })

const src = computed(() => (props.logo ? (props.logo.sources.thumb?.url ?? props.logo.url) : null))
</script>

<template>
  <span
    class="inline-flex shrink-0 items-center justify-center overflow-hidden rounded-md border border-border-subtle bg-surface-inverse-selected font-condensed font-bold tracking-[0.02em] text-text-on-inverse-selected"
    :class="[
      props.size === 'sm' ? 'h-8 text-[0.65rem]' : 'h-10 text-xs',
      src ? (props.size === 'sm' ? 'max-w-16 min-w-8 px-1' : 'max-w-20 min-w-10 px-1.5') : props.size === 'sm' ? 'w-8' : 'w-10',
    ]"
    aria-hidden="true"
  >
    <img v-if="src" :src="src" alt="" class="max-h-[80%] max-w-full object-contain" loading="lazy">
    <template v-else>{{ (props.acronym ?? initialsOf(props.name)).slice(0, 6) }}</template>
  </span>
</template>
