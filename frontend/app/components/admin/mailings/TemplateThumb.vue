<script setup lang="ts">
import type { MailingTemplate } from '~/types/admin-mailings'

/**
 * La silhouette d'une mise en page de courriel, de la taille d'une icône :
 * bandeau, image, logo, bouton d'action. Décor seul, le nom l'accompagne.
 */
const props = defineProps<{ layout: MailingTemplate }>()

const BANDS: MailingTemplate[] = ['institutionnel', 'annonce', 'institutionnel_logo']
const IMAGES: MailingTemplate[] = ['banniere', 'texte_sur_image']
const LOGOS: MailingTemplate[] = ['banniere', 'edition']
</script>

<template>
  <span class="flex h-8 w-11 shrink-0 flex-col overflow-hidden rounded-sm border border-border bg-surface" aria-hidden="true">
    <span v-if="BANDS.includes(props.layout)" class="flex h-1.5 shrink-0 items-center justify-end bg-surface-inverse px-0.5">
      <span v-if="props.layout === 'institutionnel_logo'" class="size-1 bg-surface" />
    </span>
    <span
      v-if="IMAGES.includes(props.layout)"
      class="flex shrink-0 items-end bg-border-strong p-0.5"
      :class="props.layout === 'texte_sur_image' ? 'h-3.5' : 'h-2.5'"
    >
      <span v-if="props.layout === 'texte_sur_image'" class="h-0.5 w-1/2 bg-surface" />
    </span>
    <span class="flex min-h-0 flex-1 gap-0.5 p-0.5">
      <span v-if="LOGOS.includes(props.layout)" class="size-1.5 shrink-0 bg-accent-solid" />
      <span class="flex flex-1 flex-col gap-0.5">
        <span class="w-full" :class="props.layout === 'annonce' ? 'h-1 bg-text-subtle' : 'h-0.5 bg-border-strong'" />
        <span class="h-0.5 w-3/4 bg-border" />
        <span v-if="props.layout === 'annonce'" class="mt-auto h-1 w-3 rounded-xs bg-accent-solid" />
      </span>
    </span>
  </span>
</template>
