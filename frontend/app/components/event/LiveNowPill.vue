<script setup lang="ts">
/** Dans la barre, une fois `EventLiveNowBanner` refermé : le direct dure encore, la pastille y mène. */

const { t } = useI18n()
const { tr } = useI18nText()
const { live, to, pillShown } = useLiveNow()
</script>

<template>
  <UiBannerPill v-if="pillShown && live" tone="live" dot="bg-text-on-inverse" :to="to" class="shrink-0 whitespace-nowrap">
    {{ t('live-now.tag') }}
    <span class="sr-only">: {{ tr(live.session.title) }}</span>
  </UiBannerPill>
</template>
