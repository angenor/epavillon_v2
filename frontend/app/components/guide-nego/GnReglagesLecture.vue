<script setup lang="ts">
import type { SegmentDeChoix } from '~/components/guide-nego/GnSegmente.vue'
import type { ChoixDeTheme } from '~/utils/guide-nego/theme'

/**
 * Les réglages de lecture — maquette 04 · 07. Le thème est celui de 0a, pas un second
 * réglage : le changer ici change l'application entière.
 */
const ouverte = defineModel<boolean>({ required: true })

const { t } = useI18n()
const { choix, choisir: choisirLeTheme } = useGnTheme()

const segmentsDeTheme = computed<SegmentDeChoix[]>(() => [
  { valeur: 'clair', libelle: t('gn-reglages-lecture.theme.clair'), picto: 'sun' },
  { valeur: 'sombre', libelle: t('gn-reglages-lecture.theme.sombre'), picto: 'moon' },
  { valeur: 'systeme', libelle: t('gn-reglages-lecture.theme.systeme') },
])

const themeChoisi = computed({
  get: () => choix.value,
  set: (valeur: string) => choisirLeTheme(valeur as ChoixDeTheme),
})
</script>

<template>
  <GnFeuilleBasse v-model="ouverte" :titre="t('gn-reglages-lecture.titre')" :fermeture="t('gn-reglages-lecture.fermer')">
    <div class="gn-reglages-lecture">
      <div class="gn-reglages-lecture__reglage">
        <h3 class="gn-reglages-lecture__libelle">{{ t('gn-reglages-lecture.taille.libelle') }}</h3>
        <GnChoixTailleLecture :libelle="t('gn-reglages-lecture.taille.libelle')" />
      </div>
      <div class="gn-reglages-lecture__reglage">
        <h3 class="gn-reglages-lecture__libelle">{{ t('gn-reglages-lecture.theme.libelle') }}</h3>
        <GnSegmente v-model="themeChoisi" :segments="segmentsDeTheme" :libelle="t('gn-reglages-lecture.theme.libelle')" />
      </div>
      <p class="gn-reglages-lecture__aide">{{ t('gn-reglages-lecture.aide') }}</p>
    </div>
  </GnFeuilleBasse>
</template>

<style>
[data-app="guide-nego"] .gn-reglages-lecture {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-reglages-lecture__reglage {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-reglages-lecture__libelle {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-reglages-lecture__aide {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}
</style>
