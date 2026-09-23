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
    </p>
    <button
      v-if="!seule"
      type="button"
      class="gn-occurrence__bouton"
      :aria-label="t('gn-occurrence.precedente')"
      @click="emit('precedente')"
    >
      <GnPicto nom="chev-up" />
    </button>
    <button
      v-if="!seule"
      type="button"
      class="gn-occurrence__bouton"
      :aria-label="t('gn-occurrence.suivante')"
      @click="emit('suivante')"
    >
      <GnPicto nom="chev-down" />
    </button>
    <button
      type="button"
      class="gn-occurrence__bouton gn-occurrence__bouton--fermer"
      :aria-label="t('gn-occurrence.fermer')"
      @click="emit('fermer')"
    >
      <GnPicto nom="close" />
    </button>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-occurrence {
  min-height: var(--gn-ligne-reglage);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  padding-inline: var(--gn-marge-ecran) var(--gn-espace-4);
  border-top: var(--gn-filet-2) solid var(--gn-filet-fort);
  background: var(--gn-fond);
}

[data-app="guide-nego"] .gn-occurrence__texte {
  flex: 1;
  min-width: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  font-size: var(--gn-taille-15);
  line-height: 1.3;
}

[data-app="guide-nego"] .gn-occurrence__rang {
  color: var(--gn-titre);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-occurrence__expression {
  overflow-wrap: anywhere;
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-occurrence__bouton {
  flex: none;
  width: var(--gn-cible);
  height: var(--gn-cible);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  background: none;
  color: var(--gn-picto-secondaire);
  cursor: pointer;
}

[data-app="guide-nego"] .gn-occurrence__bouton--fermer {
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-occurrence__bouton:active {
  background: var(--gn-presse);
}
</style>
