<script setup lang="ts">
/**
 * Pastille posée sur `UiMediaBanner` : état, format, journée spéciale. Verre sur
 * une photo, nuance de l'aplat sans photo. `ongoing` prend le jaune de l'attention
 * (« en cours » n'est pas une réussite), `live` le rouge du direct.
 */

interface Props {
  tone?: 'neutral' | 'accent' | 'live' | 'ongoing'
  /** Classe du point de tête ; sans elle, pas de point. */
  dot?: string
  to?: string
}

const props = withDefaults(defineProps<Props>(), { tone: 'neutral', dot: '', to: undefined })

const onMedia = inject(MEDIA_BANNER_ON_MEDIA, computed(() => false))

const toneClass = computed(() => {
  if (props.tone === 'ongoing') return 'border-warning-on-inverse bg-warning-on-inverse text-text-on-warning-inverse'
  if (props.tone === 'live') return onMedia.value ? 'border-glass-border bg-live/40' : 'border-live bg-live/40'
  if (props.tone === 'accent') return 'border-accent-on-inverse/60 bg-accent-on-inverse/12'
  return onMedia.value ? 'border-glass-border bg-glass-raised' : 'border-border-on-inverse bg-surface-inverse-raised'
})

const NuxtLink = resolveComponent('NuxtLink')
</script>

<template>
  <component
    :is="props.to ? NuxtLink : 'span'"
    :to="props.to"
    class="inline-flex items-center gap-2 rounded-full border px-3.5 text-sm font-bold"
    :class="[toneClass, props.to ? 'min-h-11 text-text-on-inverse no-underline hover:text-text-on-inverse hover:underline' : 'min-h-9']"
  >
    <span v-if="props.dot" class="size-2 shrink-0 rounded-full" :class="props.dot" aria-hidden="true" />
    <slot />
  </component>
</template>
