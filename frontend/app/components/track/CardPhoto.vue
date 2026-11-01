<script setup lang="ts">
import type { AttachedImage } from '~/types/media'

/**
 * La photo voilée d'une carte de fil (accueil, page d'un événement, autres
 * temps forts d'une journée), posée sous le texte blanc. Le voile vient de
 * `--color-scrim` : une photo claire rendrait le texte illisible sans lui. La
 * carte porte `relative isolate group`.
 */

interface Props {
  image: AttachedImage
  sizes: string
}

const props = defineProps<Props>()
</script>

<template>
  <UiImage
    :image="props.image"
    ratio="auto"
    frame-class="size-full"
    class="absolute inset-0 -z-20 transition-transform duration-500 motion-safe:group-hover:scale-[1.06]"
    :sizes="props.sizes"
  />
  <div class="absolute inset-0 -z-10 bg-scrim/20" aria-hidden="true" />
  <div class="scrim-fade-bottom absolute inset-0 -z-10" aria-hidden="true" />
</template>
