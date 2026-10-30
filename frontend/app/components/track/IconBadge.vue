<script setup lang="ts">
/**
 * Le pictogramme d'un fil (`icon_code`) dans un rond blanc, trait à la couleur
 * du fil. Rien sans pictogramme, ou pour un code que le site ne connaît plus.
 * Décoratif : le nom du fil est toujours écrit à côté.
 */

interface Props {
  code: string | null
  /** Couleur du fil ; nulle, le bleu nuit. */
  color: string | null
  /** `sm` : en-tête de colonne ; `md` : bandeau d'un cadre (44 px) ; `lg` : page du fil. */
  size?: 'sm' | 'md' | 'lg'
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })

const icon = computed(() => trackIcon(props.code))

const SIZES = {
  sm: { box: 'size-5', glyph: '14px' },
  md: { box: 'size-11', glyph: '24px' },
  lg: { box: 'size-11 sm:size-14', glyph: '1.75rem' },
} as const
</script>

<template>
  <span
    v-if="icon"
    class="inline-flex shrink-0 items-center justify-center rounded-full bg-surface-inverse-selected"
    :class="SIZES[props.size].box"
    :style="{ color: trackGlyphColor(props.color) }"
    aria-hidden="true"
  >
    <UiIcon :name="icon" :size="SIZES[props.size].glyph" :stroke-width="props.size === 'sm' ? 2.1 : 1.9" />
  </span>
</template>
