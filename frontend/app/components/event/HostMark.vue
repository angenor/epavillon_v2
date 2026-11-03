<script setup lang="ts">
/**
 * Le cercle de l'OIF sur une activité de l'organisation hôte du pavillon (l'IFDD,
 * maquette `programmation-ifdd`, 03/11). Une copie s'agrandit et s'efface derrière
 * lui, comme le point du direct ; « moins d'animations » l'arrête, le cercle reste.
 */

interface Props {
  /** Diamètre : 28 px dans la carte du calendrier, 26 px sous l'heure de la liste. */
  size?: string
}

const props = withDefaults(defineProps<Props>(), { size: '1.75rem' })

const { t } = useI18n()
const src = assetUrl('/logos/svg/oif-cercle.svg')
</script>

<template>
  <span class="relative inline-flex shrink-0" :style="{ width: props.size, height: props.size }">
    <img :src="src" alt="" class="absolute inset-0 size-full animate-ping opacity-75 motion-reduce:animate-none" aria-hidden="true">
    <img :src="src" :alt="t('host-mark.label')" :title="t('host-mark.label')" class="relative size-full">
  </span>
</template>
