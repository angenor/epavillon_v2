<script setup lang="ts">
import type { SegmentDeChoix } from '~/components/guide-nego/GnSegmente.vue'
import type { ChoixDeTheme } from '~/utils/guide-nego/theme'

/**
 * Un seul réglage à cette étape, et c'est sa place définitive : 0b y ajoutera
 * « Mon accès » et la déconnexion, 0c le reste.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { choix, choisir } = useGnTheme()

const segments = computed<SegmentDeChoix[]>(() => [
  { valeur: 'clair', libelle: t('guide-nego.reglages.theme.clair'), picto: 'sun' },
  { valeur: 'sombre', libelle: t('guide-nego.reglages.theme.sombre'), picto: 'moon' },
  { valeur: 'systeme', libelle: t('guide-nego.reglages.theme.systeme') },
])

const theme = computed({
  get: () => choix.value as string,
  set: (valeur: string) => choisir(valeur as ChoixDeTheme),
})

useHead({ title: t('guide-nego.reglages.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.reglages.titre')"
    retour="/guide-nego/ressources"
    :onglets="false"
  >
    <GnEnteteGroupe :titre="t('guide-nego.reglages.affichage')" />
    <section class="gn-reglages__bloc">
      <h3 class="gn-reglages__libelle">{{ t('guide-nego.reglages.theme.libelle') }}</h3>
      <GnSegmente
        v-model="theme"
        :segments="segments"
        :libelle="t('guide-nego.reglages.theme.libelle')"
      />
      <p class="gn-reglages__aide">{{ t('guide-nego.reglages.theme.aide') }}</p>
    </section>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-reglages__bloc {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-reglages__libelle {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-reglages__aide {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}
</style>
