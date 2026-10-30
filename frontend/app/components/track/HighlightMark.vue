<script setup lang="ts">
/** « Temps fort » au-dessus du titre d'une activité mise en avant (`session_tracks.is_highlight`). */
const { t } = useI18n()
</script>

<template>
  <span
    class="mb-2 inline-flex rounded-sm bg-(--color-day-solid) px-2 py-0.5 text-xs font-bold text-(--color-day-on-solid) uppercase"
    :style="{ letterSpacing: 'var(--tracking-caps)' }"
  >
    {{ t('track.highlight.mark') }}
  </span>
</template>
