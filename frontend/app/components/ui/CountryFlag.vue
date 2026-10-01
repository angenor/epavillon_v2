<script setup lang="ts">
/** Drapeau d'un pays, par son code ISO 3166-1 alpha-2. Des SVG plutôt que des emoji : Windows n'affiche pas ces derniers. */

interface Props {
  code: string
  /** Nom du pays, lu par les lecteurs d'écran. */
  label: string
}

const props = defineProps<Props>()

const FLAGS = import.meta.glob<string>('../../../node_modules/flag-icons/flags/4x3/*.svg', {
  eager: true,
  query: '?url',
  import: 'default',
})

const src = computed(() => FLAGS[`../../../node_modules/flag-icons/flags/4x3/${props.code.toLowerCase()}.svg`] ?? null)
</script>

<template>
  <img
    v-if="src"
    :src="src"
    :alt="props.label"
    class="inline-block h-4.5 w-6 shrink-0 rounded-[2px] border border-border object-cover"
    loading="lazy"
  >
</template>
