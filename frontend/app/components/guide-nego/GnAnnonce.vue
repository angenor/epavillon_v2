<script setup lang="ts">
/** Une ligne qui dit une chose une fois, et se ferme d'un toucher sur sa croix. */
const props = defineProps<{ texte: string }>()
const emit = defineEmits<{ fermer: [] }>()
const { t } = useI18n()
</script>

<template>
  <p class="gn-annonce" role="status">
    <GnPicto nom="info" :taille="20" class="gn-annonce__picto" />
    <span class="gn-annonce__texte">{{ props.texte }}</span>
    <button type="button" class="gn-annonce__fermer" :aria-label="t('gn-annonce.fermer')" @click="emit('fermer')">
      <GnPicto nom="close" :taille="20" />
    </button>
  </p>
</template>

<style>
[data-app="guide-nego"] .gn-annonce {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding: var(--gn-espace-8) var(--gn-espace-4) var(--gn-espace-8) var(--gn-marge-ecran);
  background: var(--gn-fond-2);
  color: var(--gn-texte);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-annonce__picto {
  flex: none;
  margin-top: var(--gn-espace-12);
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-annonce__texte {
  flex: 1;
  min-width: 0;
  padding-block: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-annonce__fermer {
  flex: none;
  display: inline-grid;
  place-items: center;
  min-width: var(--gn-cible);
  min-height: var(--gn-cible);
  border: none;
  background: none;
  color: var(--gn-texte);
  cursor: pointer;
}
</style>
