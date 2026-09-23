<script setup lang="ts">
import type { Span } from '~/types/negotiation-documents'

defineProps<{ spans: Span[] }>()

const { t } = useI18n()
</script>

<template>
  <template v-for="(span, index) in spans" :key="index">
    <em
      v-if="span.term"
      class="rounded-sm bg-accent-surface px-0.5 underline decoration-accent decoration-dotted underline-offset-4"
      :class="span.bold && 'font-semibold'"
      :title="t('admin.negociations.documents.preview.reading.term')"
    >{{ span.text }}</em>
    <em v-else-if="span.italic" :class="span.bold && 'font-semibold'">{{ span.text }}</em>
    <strong v-else-if="span.bold" class="font-semibold">{{ span.text }}</strong>
    <template v-else>{{ span.text }}</template>
  </template>
</template>
