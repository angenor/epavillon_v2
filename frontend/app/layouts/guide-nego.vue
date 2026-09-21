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

useHead({ htmlAttrs: { lang: 'fr' } })

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
