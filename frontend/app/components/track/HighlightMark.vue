<script setup lang="ts">
/** « Temps fort » sous une activité mise en avant (`session_tracks.is_highlight`), à la couleur de la journée. */
const { t } = useI18n()
</script>

<template>
  <p class="mt-2 inline-flex items-center gap-1.5 text-[0.82rem] font-semibold text-(--color-day)">
    <UiIcon name="star" size="0.95rem" :stroke-width="2" />
    {{ t('track.highlight.mark') }}
  </p>
</template>
