<script setup lang="ts">
import { compteurDeCloche } from '~/utils/guide-nego/signalements'

/** La cloche de l'en-tête : le compteur jaune dit les non lues, et rien d'autre (05-design, l. 102). */
const props = withDefaults(defineProps<{ nonLues: number; vers?: string }>(), { vers: '/guide-nego/notifications' })

const { t } = useI18n()
const libelle = computed(() =>
  props.nonLues > 0 ? t('gn-cloche.libelle-non-lues', { count: props.nonLues }, props.nonLues) : t('gn-cloche.libelle'),
)
</script>

<template>
  <NuxtLink :to="vers" class="gn-cloche" :aria-label="libelle">
    <GnPicto nom="bell" :taille="24" />
    <span v-if="nonLues > 0" class="gn-cloche__compteur" aria-hidden="true">{{ compteurDeCloche(nonLues) }}</span>
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-cloche {
  position: relative;
  flex: none;
  width: var(--gn-cible);
  height: var(--gn-cible);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--gn-titre);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-cloche:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-cloche__compteur {
  position: absolute;
  top: 4px;
  right: 2px;
  min-width: var(--gn-compteur);
  height: var(--gn-compteur);
  padding-inline: 5px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
  font-size: var(--gn-taille-13);
  line-height: 1;
  font-weight: var(--gn-graisse-gras);
}
</style>
