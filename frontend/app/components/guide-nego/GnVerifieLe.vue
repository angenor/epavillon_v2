<script setup lang="ts">
import { jourDeVerification } from '~/utils/guide-nego/faq'

/**
 * « Vérifié le … » : le bouclier coché dit « validé par un expert » (écart 19). Une
 * entrée à revoir garde sa date, mais plus la coche : la validation est en question.
 */
const props = withDefaults(defineProps<{ jour: string; aRevoir?: boolean }>(), { aRevoir: false })

const { t, locale } = useI18n()
const date = computed(() => jourDeVerification(props.jour, String(locale.value)))
</script>

<template>
  <span class="gn-verifie" :class="{ 'gn-verifie--a-revoir': aRevoir }">
    <GnPicto :nom="aRevoir ? 'shield' : 'shield-check'" :taille="16" />
    {{ t('gn-verifie-le.verifie', { date }) }}
  </span>
</template>

<style>
[data-app="guide-nego"] .gn-verifie {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-succes);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-verifie--a-revoir {
  color: var(--gn-texte-2);
}
</style>
