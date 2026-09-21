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
import { CLE_GARDE_ANNONCEE, lireCle, poserCle } from '~/utils/guide-nego/stockage'

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
const { ouverte, pret, rafraichir } = useGnDrapeaux()
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

const { t } = useI18n()
const gardePrete = ref(false)

// Le navigateur annonce le retour du réseau ; seule une lecture réussie le prouve, et
// c'est elle qui met à jour « Synchronisé à » — sans recharger la page.
function relire() {
  void rafraichir()
}

onMounted(() => {
  window.addEventListener('online', relire)
  enregistrerLaGarde()
})

onBeforeUnmount(() => window.removeEventListener('online', relire))

/**
 * Une version gardée lors d'une visite précédente prend la main MAINTENANT, au
 * chargement, et jamais en cours d'usage : une version en attente a, par construction,
 * son cache complet, et la page n'a encore rien à perdre. Sans cela elle attendrait que
 * l'application soit fermée pour de bon — sur un téléphone, ce jour peut ne pas venir.
 */
function prendreLaVersionEnAttente(inscription: ServiceWorkerRegistration) {
  // Sans contrôleur, c'est la toute première ouverture : il n'y a rien à remplacer.
  if (!inscription.waiting || !navigator.serviceWorker.controller) return
  let rechargee = false
  navigator.serviceWorker.addEventListener(
    'controllerchange',
    () => {
      if (rechargee) return
      rechargee = true
      location.reload()
    },
    { once: true },
  )
  inscription.waiting.postMessage('prendre-la-main')
}

/**
 * Le service worker n'existe qu'en construction et ne vit que sous `guide-nego/` : le
 * site n'en enregistre aucun, et celui-ci ne peut contrôler aucune de ses pages.
 */
async function enregistrerLaGarde() {
  if (import.meta.dev || !('serviceWorker' in navigator)) return
  try {
    const inscription = await navigator.serviceWorker.register(assetUrl('/guide-nego/sw.js'), {
      scope: assetUrl('/guide-nego/'),
    })
    prendreLaVersionEnAttente(inscription)
    // À chaque ouverture : c'est ainsi qu'une nouvelle version se garde en arrière-plan.
    void inscription.update()
    await navigator.serviceWorker.ready
    // Dit une seule fois par téléphone : la première garde a pris le temps du réseau,
    // et c'est elle qui permet la salle sans réseau (FR-012 bis).
    if (!lireCle(CLE_GARDE_ANNONCEE)) {
      gardePrete.value = true
      poserCle(CLE_GARDE_ANNONCEE)
    }
  } catch {
    /* Sans garde, l'application reste utilisable avec du réseau. */
  }
}
</script>

<template>
  <div data-app="guide-nego">
    <slot />
    <div id="gn-portail" />
    <GnMessageEphemere
      v-if="gardePrete"
      :texte="t('gn-connexion.prete')"
      @fini="gardePrete = false"
    />
  </div>
</template>
