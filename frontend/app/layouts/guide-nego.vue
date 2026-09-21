<script setup lang="ts">
/**
 * Mise en page de Guide Négo — la borne de son système de design.
 *
 * Tout ce que l'application dessine vit sous l'élément qui porte `data-app` :
 * jetons, thème, styles, et `#gn-portail`, seule cible de téléportation.
 */
import '~/assets/guide-nego/police.css'
import '~/assets/guide-nego/theme.css'
import '~/assets/guide-nego/mesures.css'
import '~/assets/guide-nego/base.css'

// La barre système du téléphone ne lit pas le CSS : elle veut une valeur. C'est
// `--gn-charte-vert-tres-fonce` ; le thème sombre la remplacera (0a, US4).
const COULEUR_BARRE_SYSTEME = '#233400'

// Le manifeste et l'icône ne se posent QUE sous cette mise en page : le site n'est pas
// installable, et son onglet garde son propre titre.
useHead({
  htmlAttrs: { lang: 'fr' },
  titleTemplate: (titre) => (titre ? `${titre} — Guide Négo` : 'Guide Négo'),
  link: [
    { rel: 'manifest', href: assetUrl('/guide-nego/manifest.webmanifest') },
    { rel: 'apple-touch-icon', href: assetUrl('/guide-nego/icones/180.png') },
  ],
  meta: [
    { name: 'theme-color', content: COULEUR_BARRE_SYSTEME },
    { name: 'apple-mobile-web-app-capable', content: 'yes' },
    { name: 'apple-mobile-web-app-title', content: 'Guide Négo' },
  ],
})

// Le verdict du drapeau peut arriver après l'ouverture : il s'applique alors dans les
// deux sens, sans attendre une navigation.
const { ouverte, pret } = useGnDrapeaux()
const route = useRoute()
// Le routeur se prend ici : `navigateTo` n'a plus son contexte dans un observateur.
const router = useRouter()
const FERMEE = '/guide-nego/fermee'

// `immediate` : la réponse a pu arriver avant que cette mise en page ne soit montée.
watch(
  [ouverte, pret, () => route.path],
  ([estOuverte, estPret]) => {
    if (!estPret) return
    const surFermee = route.path.replace(/\/$/, '') === FERMEE
    if (!estOuverte && !surFermee) void router.replace(FERMEE)
    if (estOuverte && surFermee) void router.replace('/guide-nego')
  },
  { immediate: true },
)
</script>

<template>
  <div data-app="guide-nego">
    <slot />
    <div id="gn-portail" />
  </div>
</template>
