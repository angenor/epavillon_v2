<script setup lang="ts">
/** La barre « MAINTENANT · 10:48 » de la frise (Nuit 02), posée entre deux lignes. */
defineProps<{ heure: string }>()

const { t } = useI18n()
</script>

<template>
  <p class="gn-frise-maintenant">
    <span class="gn-frise-maintenant__trait" aria-hidden="true" />
    {{ t('gn-frise-maintenant.libelle', { heure }) }}
    <span class="gn-frise-maintenant__fil" aria-hidden="true" />
  </p>
</template>

<style>
[data-app="guide-nego"] .gn-frise-maintenant {
  margin: 2px 0 2px 46px;
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--gn-accent);
  font-size: var(--gn-taille-12);
  line-height: var(--gn-interligne-12);
  font-weight: var(--gn-graisse-extra-gras);
  letter-spacing: var(--gn-approche-12);
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-frise-maintenant__trait {
  flex: none;
  width: 12px;
  height: 2px;
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-frise-maintenant__fil {
  flex: 1;
  height: 2px;
  background: var(--gn-accent);
}
</style>
