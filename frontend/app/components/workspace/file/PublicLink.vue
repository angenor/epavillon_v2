<script setup lang="ts">
import { encode } from 'uqr'

/**
 * LE QR CODE DE LA PAGE PUBLIQUE, dans le coin du titre (29/10).
 *
 * Dessiné ici en SVG à partir de la matrice d'`uqr`, et non injecté : aucun
 * `v-html`. Tuile claire dans les deux thèmes, comme un logo : un QR code inversé
 * ne se lit plus. Le téléchargement rend la même matrice en PNG sur fond blanc,
 * à imprimer sur un support ou à afficher à l'entrée de la salle.
 */

interface Props {
  /** Adresse absolue de la page publique de l'activité. */
  url: string
  /** Nom du fichier téléchargé, sans extension. */
  fileName: string
}

const props = defineProps<Props>()

const { t } = useI18n()

const matrix = computed(() => encode(props.url, { ecc: 'M', border: 1 }).data)

const cells = computed(() => {
  const rects: { x: number; y: number }[] = []
  matrix.value.forEach((row, y) => row.forEach((dark, x) => dark && rects.push({ x, y })))
  return rects
})

const size = computed(() => matrix.value.length)

const copied = ref(false)

async function copy(): Promise<void> {
  await navigator.clipboard.writeText(props.url)
  copied.value = true
  setTimeout(() => (copied.value = false), 2500)
}

/** En PNG (30/10) : le SVG ne s'ouvrait pas dans les outils de mise en page courants. */
function download(): void {
  const n = size.value
  // Un nombre entier de pixels par module : un QR code lissé se lit mal.
  const scale = Math.ceil(1024 / n)
  const canvas = document.createElement('canvas')
  canvas.width = canvas.height = n * scale
  const context = canvas.getContext('2d')
  if (!context) return
  context.fillStyle = '#fff'
  context.fillRect(0, 0, canvas.width, canvas.height)
  context.fillStyle = '#000'
  for (const { x, y } of cells.value) context.fillRect(x * scale, y * scale, scale, scale)
  canvas.toBlob((blob) => {
    if (!blob) return
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `${props.fileName}.png`
    link.click()
    URL.revokeObjectURL(link.href)
  }, 'image/png')
}
</script>

<template>
  <div class="flex shrink-0 items-center gap-3">
    <div class="flex flex-col items-end">
      <UiButton variant="link" :icon="copied ? 'check' : 'copy'" @click="copy">
        {{ t(copied ? 'organization.workspace.proposal.publicLink.copied' : 'organization.workspace.proposal.publicLink.copy') }}
      </UiButton>
      <UiButton variant="link" icon="download" @click="download">
        {{ t('organization.workspace.proposal.publicLink.download') }}
      </UiButton>
    </div>
    <a
      :href="props.url"
      target="_blank"
      rel="noopener"
      class="block size-24 rounded-md border border-border bg-surface-inverse-selected p-1.5 text-text-on-inverse-selected"
      :aria-label="t('organization.workspace.proposal.publicLink.open')"
    >
      <svg :viewBox="`0 0 ${size} ${size}`" class="size-full" shape-rendering="crispEdges" aria-hidden="true">
        <rect v-for="cell in cells" :key="`${cell.x}-${cell.y}`" :x="cell.x" :y="cell.y" width="1" height="1" fill="currentColor" />
      </svg>
    </a>
  </div>
</template>
