<script setup lang="ts">
/**
 * « Occurrence 3 sur 5 », au-dessus de la barre de lecture repliée ; l'écran la place.
 * Précédente et suivante bouclent : l'écran passe de la dernière à la première, comme
 * la recherche d'un navigateur. Seule, une occurrence n'offre ni l'une ni l'autre : un
 * bouton sans effet n'apparaît pas.
 */
const props = defineProps<{
  rang: number
  total: number
  expression: string
  /** Ce que le lecteur n'a pas pu faire du passage : le marquer, ou le distinguer des autres (FR-015). */
  remarque?: string
}>()

const emit = defineEmits<{ precedente: []; suivante: []; fermer: [] }>()

const { t } = useI18n()

const seule = computed(() => props.total <= 1)
</script>

<template>
  <div class="gn-occurrence" role="group" :aria-label="t('gn-occurrence.groupe')">
    <p class="gn-occurrence__texte" aria-live="polite">
      <span class="gn-occurrence__rang">{{ t('gn-occurrence.rang', { rang, total }) }}</span>
      <span class="gn-occurrence__expression">{{ t('gn-occurrence.expression', { expression }) }}</span>
      <span v-if="remarque" class="gn-occurrence__remarque">{{ remarque }}</span>
    </p>
    <GnBoutonRond v-if="!seule" borde picto="chev-up" :libelle="t('gn-occurrence.precedente')" @clic="emit('precedente')" />
    <GnBoutonRond v-if="!seule" borde picto="chev-down" :libelle="t('gn-occurrence.suivante')" @clic="emit('suivante')" />
    <GnBoutonRond picto="close" :libelle="t('gn-occurrence.fermer')" @clic="emit('fermer')" />
  </div>
</template>

<style>
/* Posée sur la barre de lecture, comme une feuille basse : fond de bloc, rayon 24 en haut. */
[data-app="guide-nego"] .gn-occurrence {
  min-height: var(--gn-ligne-reglage);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  padding: var(--gn-espace-8) var(--gn-espace-12) var(--gn-espace-8) var(--gn-marge-ecran);
  border-radius: var(--gn-rayon-24) var(--gn-rayon-24) 0 0;
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-occurrence__texte {
  flex: 1;
  min-width: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

[data-app="guide-nego"] .gn-occurrence__rang {
  color: var(--gn-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-occurrence__expression {
  overflow-wrap: anywhere;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-occurrence__remarque {
  padding-block-end: var(--gn-espace-4);
  color: var(--gn-texte);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}
</style>
