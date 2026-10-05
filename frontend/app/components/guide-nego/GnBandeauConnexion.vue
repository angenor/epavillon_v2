<script setup lang="ts">
/**
 * Le bandeau d'un épisode hors connexion : une fois, puis l'en-tête suffit à le
 * rappeler. Il dit ce qui reste lisible — l'important n'est pas la panne, c'est que
 * tout ce qui est là se lit quand même.
 */
const props = defineProps<{
  luA: string | null
  /** Ce qui reste lisible, quand l'écran n'est pas tout entier lisible sans réseau. */
  ceQuiSeLit?: string
}>()

const { t } = useI18n()
const { momentLisible } = useGnMomentLecture()

const texte = computed(() => {
  const moment = momentLisible(props.luA)
  const suite = props.ceQuiSeLit ?? t('gn-connexion.tout-se-lit')
  return moment ? t('gn-connexion.bandeau', { moment, suite }) : t('gn-connexion.bandeau-sans-heure', { suite })
})
</script>

<template>
  <p class="gn-bandeau" role="status">
    <GnPicto nom="wifi-off" :taille="18" />
    {{ texte }}
  </p>
</template>

<style>
/* L'aplat d'attention, seul bandeau plein : GnEcran le pose pleine largeur, sous l'en-tête. */
[data-app="guide-nego"] .gn-bandeau {
  min-height: var(--gn-bouton-rond);
  padding: var(--gn-espace-12) var(--gn-marge-ecran);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-bandeau .gn-picto {
  flex: none;
}
</style>
