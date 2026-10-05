<script setup lang="ts">
/**
 * « Cette réponse vous a-t-elle aidée ? » — maquette 01, § 4 quinquies. Une fois la
 * voix donnée, la ligne devient « Merci. » avec la coche verte.
 */
withDefaults(defineProps<{ merci?: boolean }>(), { merci: false })
defineEmits<{ oui: []; non: [] }>()

const { t } = useI18n()
</script>

<template>
  <div class="gn-retour-utile" aria-live="polite">
    <p class="gn-retour-utile__question">{{ t('gn-retour-utile.question') }}</p>
    <p v-if="merci" class="gn-retour-utile__merci">
      <GnPicto nom="check" :taille="20" />{{ t('gn-retour-utile.merci') }}
    </p>
    <div v-else class="gn-retour-utile__boutons">
      <GnBouton variante="secondaire" largeur="demie" @clic="$emit('oui')">{{ t('gn-retour-utile.oui') }}</GnBouton>
      <GnBouton variante="secondaire" largeur="demie" @clic="$emit('non')">{{ t('gn-retour-utile.non') }}</GnBouton>
    </div>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-retour-utile {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-retour-utile__question {
  color: var(--gn-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-retour-utile__boutons {
  display: flex;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-retour-utile__merci {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  min-height: var(--gn-bouton-principal);
  color: var(--gn-succes);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-retour-utile__merci .gn-picto {
  flex: none;
}
</style>
