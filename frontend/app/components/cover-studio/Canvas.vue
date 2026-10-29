<script setup lang="ts">
import type { CoverDesignOptions } from '~/types/programme/cover-design'
import type { CoverScene } from '~/utils/cover-studio'

/**
 * UNE IMAGE DE COUVERTURE DESSINÉE, toujours à 1600 × 900 : seule la taille
 * d'AFFICHAGE change (aperçu, vignette d'un modèle, aperçu de partage).
 * `toBlob()` rend exactement ce qui est montré.
 */

interface Props {
  scene: CoverScene | null
  template: string
  options: CoverDesignOptions
  /** Nom accessible : ce que l'image représente. */
  label: string
}

const props = defineProps<Props>()

const canvas = ref<HTMLCanvasElement | null>(null)
const width = COVER_WIDTH
const height = COVER_HEIGHT

function draw(): void {
  const element = canvas.value
  const context = element?.getContext('2d')
  if (!element || !context || !props.scene) return
  drawCover(context, props.scene, props.template, props.options)
}

watch(() => [props.scene, props.template, props.options], draw, { deep: true })
onMounted(draw)

function toBlob(): Promise<Blob | null> {
  return new Promise((resolve) => {
    if (!canvas.value) return resolve(null)
    canvas.value.toBlob((blob) => resolve(blob), 'image/png')
  })
}

defineExpose({ toBlob })
</script>

<template>
  <canvas
    ref="canvas"
    :width="width"
    :height="height"
    role="img"
    :aria-label="props.label"
    class="block aspect-video h-auto w-full bg-surface-sunken"
  />
</template>
