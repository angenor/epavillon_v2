<script setup lang="ts">
import type { ModeDeLecture } from '~/utils/guide-nego/appareil-lecture'

/**
 * Les réglages de lecture — maquette 04 · 07, sans le thème : l'application n'en a qu'un
 * (ADR-023). La taille ne vaut que pour « Texte agrandi » : une page du PDF garde la sienne.
 */
const props = withDefaults(defineProps<{ texteOffert?: boolean }>(), { texteOffert: false })
const ouverte = defineModel<boolean>({ required: true })
const mode = defineModel<ModeDeLecture>('mode', { default: 'pages' })

const { t } = useI18n()
</script>

<template>
  <GnFeuilleBasse v-model="ouverte" :titre="t('gn-reglages-lecture.titre')" :fermeture="t('gn-reglages-lecture.fermer')">
    <div class="gn-reglages-lecture">
      <div v-if="props.texteOffert" class="gn-reglages-lecture__reglage">
        <h3 class="gn-reglages-lecture__libelle">{{ t('gn-reglages-lecture.mode.libelle') }}</h3>
        <GnChoixMode v-model="mode" />
      </div>
      <div v-if="mode === 'texte' && props.texteOffert" class="gn-reglages-lecture__reglage">
        <h3 class="gn-reglages-lecture__libelle">{{ t('gn-reglages-lecture.taille.libelle') }}</h3>
        <GnChoixTailleLecture :libelle="t('gn-reglages-lecture.taille.libelle')" />
      </div>
      <p class="gn-reglages-lecture__aide">{{ t('gn-reglages-lecture.aide') }}</p>
    </div>
  </GnFeuilleBasse>
</template>

<style>
[data-app="guide-nego"] .gn-reglages-lecture {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-20);
}

[data-app="guide-nego"] .gn-reglages-lecture__reglage {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-reglages-lecture__libelle {
  color: var(--gn-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-reglages-lecture__aide {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}
</style>
