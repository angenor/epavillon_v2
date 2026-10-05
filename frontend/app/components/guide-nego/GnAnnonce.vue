<script setup lang="ts">
/** Une ligne qui dit une chose une fois, et se ferme d'un toucher sur sa croix. */
const props = defineProps<{ texte: string }>()
const emit = defineEmits<{ fermer: [] }>()
const { t } = useI18n()
</script>

<template>
  <p class="gn-annonce" role="status">
    <span class="gn-annonce__picto" aria-hidden="true"><GnPicto nom="info" :taille="18" /></span>
    <span class="gn-annonce__texte">{{ props.texte }}</span>
    <GnBoutonRond class="gn-annonce__fermer" picto="close" :libelle="t('gn-annonce.fermer')" @clic="emit('fermer')" />
  </p>
</template>

<style>
/* La carte bordée d'invitation (maquette 04), ramenée à une ligne. */
[data-app="guide-nego"] .gn-annonce {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  padding: var(--gn-espace-8) var(--gn-espace-4) var(--gn-espace-8) var(--gn-espace-16);
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-24);
  background: var(--gn-fond-2);
  color: var(--gn-texte);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-annonce__picto {
  flex: none;
  width: var(--gn-pastille-icone-petite);
  height: var(--gn-pastille-icone-petite);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-12);
  background: var(--gn-bloc-releve);
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-annonce__texte {
  flex: 1;
  min-width: 0;
  padding-block: var(--gn-espace-8);
}
</style>
