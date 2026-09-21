<script setup lang="ts">
/**
 * Le bandeau d'un épisode hors connexion : une fois, puis l'en-tête suffit à le
 * rappeler. Il dit ce qui reste lisible — l'important n'est pas la panne, c'est que
 * tout ce qui est là se lit quand même.
 */
const props = defineProps<{ luA: string | null }>()

const { t } = useI18n()
const { momentLisible } = useGnMomentLecture()

const texte = computed(() => {
  const moment = momentLisible(props.luA)
  return moment ? t('gn-connexion.bandeau', { moment }) : t('gn-connexion.bandeau-sans-heure')
})
</script>

<template>
  <p class="gn-bandeau" role="status">
    <GnPicto nom="wifi-off" :taille="18" />
    {{ texte }}
  </p>
</template>

<style>
[data-app="guide-nego"] .gn-bandeau {
  min-height: var(--gn-cible);
  padding: var(--gn-espace-12) var(--gn-marge-ecran);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}
</style>
