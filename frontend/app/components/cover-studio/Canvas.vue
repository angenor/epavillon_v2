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

function encode(quality: number): Promise<Blob | null> {
  return new Promise((resolve) => {
    if (!canvas.value) return resolve(null)
    canvas.value.toBlob((blob) => resolve(blob), 'image/jpeg', quality / 100)
  })
}

/**
 * L'image en JPEG, à la plus haute qualité qui tient sous `maxBytes` (300 Ko
 * demandés par le commanditaire pour la version haute qualité). Un PNG de
 * photo pèserait plusieurs mégaoctets.
 */
async function toJpeg(maxBytes: number): Promise<Blob | null> {
  const best = await encode(90)
  if (!best || best.size <= maxBytes) return best
  let low = 40
  let high = 89
  let kept: Blob | null = null
  while (low <= high) {
    const middle = Math.floor((low + high) / 2)
    const attempt = await encode(middle)
    if (!attempt) return null
    if (attempt.size <= maxBytes) {
      kept = attempt
      low = middle + 1
    } else {
      high = middle - 1
    }
  }
  return kept ?? encode(40)
}

defineExpose({ toJpeg })
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
