<script setup lang="ts">
/**
 * La planche de design, rendue par les vrais jetons et les vrais composants.
 *
 * Aucun lien de l'application n'y mène et elle est en `noindex` : c'est un outil de
 * travail, pas un écran. On y vient par son adresse. Un seul thème, « Nuit » (ADR-023).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

/** Les sept ancres, dans l'ordre de la maquette. Le numéro sert de clé et d'ancre. */
const sections = ['1', '2', '3', '4', '5', '6', '7'] as const

useHead({
  title: t('guide-nego.composants.titre'),
  meta: [{ name: 'robots', content: 'noindex' }],
})
</script>

<template>
  <div class="gn-planche">
    <header class="gn-planche__entete">
      <h1 class="gn-planche__titre">{{ t('guide-nego.composants.titre') }}</h1>
      <p class="gn-planche__propos">{{ t('guide-nego.composants.propos') }}</p>
      <nav class="gn-planche__sommaire" :aria-label="t('guide-nego.composants.sommaire')">
        <a v-for="numero in sections" :key="numero" :href="`#gn-planche-${numero}`">
          {{ numero }} · {{ t(`guide-nego.composants.section.${numero}`) }}
        </a>
      </nav>
    </header>

    <div class="gn-planche__volet">
      <div id="gn-planche-1"><GnPlancheCouleurs /></div>
      <div id="gn-planche-2"><GnPlancheTypographie /></div>
      <div id="gn-planche-3"><GnPlancheMesures /></div>
      <div id="gn-planche-4"><GnPlanchePictogrammes /></div>
      <div id="gn-planche-5"><GnPlancheComposants /></div>
      <div id="gn-planche-6"><GnPlancheEtats /></div>
      <div id="gn-planche-7"><GnPlancheMouvement /></div>
    </div>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-planche {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
  padding: var(--gn-espace-24) var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-planche__entete {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-planche__titre {
  color: var(--gn-titre);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-28);
  line-height: var(--gn-interligne-28);
  letter-spacing: var(--gn-approche-28);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-planche__propos {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-planche__sommaire {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-planche__sommaire a {
  min-height: var(--gn-pilule-jour);
  padding-inline: var(--gn-espace-16);
  display: inline-flex;
  align-items: center;
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-pilule);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-planche__volet {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-32);
}
</style>
