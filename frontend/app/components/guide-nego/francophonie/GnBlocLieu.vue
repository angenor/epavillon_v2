<script setup lang="ts">
/** Le bloc de lieu (01 · 4 decies) : l'épingle, le nom, l'adresse, et le plan s'il est connu. */
withDefaults(defineProps<{ nom: string; adresse?: string | null; plan?: string | null }>(), {
  adresse: null,
  plan: null,
})

const { t } = useI18n()
</script>

<template>
  <div class="gn-bloc-lieu">
    <span class="gn-bloc-lieu__epingle"><GnPicto nom="pin" :taille="20" /></span>
    <div class="gn-bloc-lieu__corps">
      <p class="gn-bloc-lieu__nom">{{ nom }}</p>
      <p v-if="adresse" class="gn-bloc-lieu__adresse">{{ adresse }}</p>
      <a v-if="plan" :href="plan" class="gn-bloc-lieu__plan" target="_blank" rel="noopener">
        <GnPicto nom="external" :taille="20" />
        {{ t('gn-bloc-lieu.plan') }}
      </a>
    </div>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-bloc-lieu {
  margin-top: var(--gn-espace-16);
  padding: var(--gn-espace-16);
  display: flex;
  align-items: flex-start;
  gap: 14px;
  background: var(--gn-fond-2);
  border-radius: var(--gn-rayon-20);
}

[data-app="guide-nego"] .gn-bloc-lieu__epingle {
  flex: none;
  width: var(--gn-pastille-icone);
  height: var(--gn-pastille-icone);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-14);
  background: var(--gn-bloc-releve);
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-bloc-lieu__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-bloc-lieu__nom {
  font-size: var(--gn-taille-16);
  line-height: 1.3;
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-bloc-lieu__adresse {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-pavillon > .gn-bloc-lieu {
  margin-top: 18px;
}

[data-app="guide-nego"] .gn-bloc-lieu__plan {
  align-self: flex-start;
  min-height: var(--gn-cible);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-bloc-lieu__plan .gn-picto {
  flex: none;
}
</style>
