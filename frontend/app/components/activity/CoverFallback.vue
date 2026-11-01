<script setup lang="ts">
import type { AttachedImage } from '~/types/media'

/**
 * La couverture d'une activité qui n'en a pas (maquette `couverture-par-defaut`,
 * 01/11) : la photo de son édition sous un voile bleu nuit, le sigle de
 * l'organisation en blanc par-dessus. Posée sur tout le cadre de l'appelant,
 * positionné, qui porte ses proportions. Sans photo d'édition, le slot : la
 * tuile que l'appelant montrait jusque-là.
 *
 * Le voile mêle `--color-scrim` (le contraste, comme sur toute photo) et
 * l'aplat institutionnel (la teinte) : aucun des deux ne s'inverse en thème sombre.
 */

interface Props {
  /** La photo de l'édition (`editionPhoto()`) ; nulle : le slot. */
  image: AttachedImage | null | undefined
  /** Le sigle, à défaut ce que l'appelant met dans sa tuile. */
  label?: string | null
  /** La largeur rendue : une liste prend une petite déclinaison, la page d'une activité la grande. */
  sizes?: string
  /** Taille du sigle, à la mesure du cadre. */
  textClass?: string
  /** Terminée ou annulée : la photo passe en gris, comme une couverture. */
  faded?: boolean
  loading?: 'lazy' | 'eager'
  /** Classes de l'image (survol), comme celles d'une couverture. */
  imageClass?: string | Array<string | Record<string, boolean>>
}

const props = withDefaults(defineProps<Props>(), {
  label: null,
  sizes: undefined,
  textClass: 'text-[1.5rem]',
  faded: false,
  loading: 'lazy',
  imageClass: undefined,
})
</script>

<template>
  <div v-if="props.image" class="absolute inset-0 flex items-center justify-center overflow-hidden bg-surface-inverse" aria-hidden="true">
    <UiImage
      :image="props.image"
      ratio="auto"
      frame-class="size-full"
      class="absolute inset-0"
      :class="[props.imageClass, { grayscale: props.faded }]"
      :sizes="props.sizes"
      :loading="props.loading"
    />
    <span class="absolute inset-0 bg-scrim/25" />
    <span class="absolute inset-0 bg-surface-inverse/50" />
    <span
      v-if="props.label"
      class="relative max-w-full min-w-0 truncate px-2.5 font-condensed leading-none font-bold tracking-[0.03em] whitespace-nowrap text-text-on-inverse text-shadow-md"
      :class="props.textClass"
    >
      {{ props.label }}
    </span>
  </div>
  <slot v-else />
</template>
