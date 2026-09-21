<script setup lang="ts">
/**
 * Le rappel discret, au-dessus du titre. « Synchronisé à » dit l'heure de la dernière
 * lecture réussie, jamais l'instant présent (écart 25) : c'est ce qui permet de juger
 * si la salle affichée date de cinq minutes ou d'hier soir.
 */
const props = defineProps<{ enLigne: boolean; luA: string | null }>()

const { t } = useI18n()
const { momentLisible } = useGnMomentLecture()

const libelle = computed(() => {
  const moment = momentLisible(props.luA)
  if (!moment) return props.enLigne ? null : t('gn-connexion.hors-connexion')
  return props.enLigne
    ? t('gn-connexion.synchronise', { moment })
    : t('gn-connexion.hors-connexion-lu', { moment })
})
</script>

<template>
  <span v-if="libelle" class="gn-connexion" :class="{ 'gn-connexion--hors': !enLigne }">
    <GnPicto :nom="enLigne ? 'sync' : 'wifi-off'" :taille="16" />
    {{ libelle }}
  </span>
</template>

<style>
[data-app="guide-nego"] .gn-connexion {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-succes);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-connexion--hors {
  color: var(--gn-texte-2);
}
</style>
