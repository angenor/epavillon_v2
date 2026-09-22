<script setup lang="ts">
import type { Segment } from '~/utils/guide-nego/texte-long'

/** Une suite de segments en ligne : texte, emphase, lien — jamais de HTML. */
defineProps<{ segments: Segment[] }>()
</script>

<template>
  <template v-for="(segment, index) in segments" :key="index">
    <strong v-if="segment.type === 'gras'">{{ segment.texte }}</strong>
    <em v-else-if="segment.type === 'italique'">{{ segment.texte }}</em>
    <a
      v-else-if="segment.type === 'lien'"
      :href="segment.adresse"
      target="_blank"
      rel="noopener noreferrer"
    >{{ segment.texte }}</a>
    <template v-else>{{ segment.texte }}</template>
  </template>
</template>
