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
import { CLE_FILE_THEMATIQUES } from '~/utils/guide-nego/thematiques'

// La barre système du téléphone ne lit pas le CSS : elle veut une valeur. Ce sont
// `--gn-charte-vert-tres-fonce` et `--gn-nuance-sombre-fond`, les seuls endroits du
// code où une couleur s'écrit — la barre n'a pas d'autre langue.
const BARRE_SYSTEME = { clair: '#233400', sombre: '#101704' }

const { affiche } = useGnTheme()

// Le manifeste et l'icône ne se posent QUE sous cette mise en page : le site n'est pas
// installable, et son onglet garde son propre titre.
useHead(() => ({
  htmlAttrs: { lang: 'fr' },
  titleTemplate: (titre) => (titre ? `${titre} — Guide Négo` : 'Guide Négo'),
  link: [
    { rel: 'manifest', href: assetUrl('/guide-nego/manifest.webmanifest') },
    { rel: 'apple-touch-icon', href: assetUrl('/guide-nego/icones/180.png') },
  ],
  meta: [
    { name: 'theme-color', content: BARRE_SYSTEME[affiche.value] },
    { name: 'apple-mobile-web-app-capable', content: 'yes' },
    { name: 'apple-mobile-web-app-title', content: 'Guide Négo' },
  ],
}))

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

// Le vocabulaire des thématiques se lit avec le drapeau et se garde comme lui : une
// lecture publique de dix termes, qui donne ses noms à la ligne du profil même sur un
// appareil qui n'a jamais ouvert l'écran des thématiques.
const { assurerLeVocabulaire } = useGnThematiques()
// Ce qui a été choisi sans réseau repart d'ici — et de nulle part ailleurs.
const { partir, avis } = useGnFile()
// Les téléchargements demandés sans réseau partagent ses déclencheurs, pas sa file.
const copies = useGnCopies()

// Un choix abandonné ou refusé se dit là où la personne se trouve quand le réseau
// revient (FR-009 bis) ; l'écran qui l'a pris le redit en place, et on ne l'y double pas.
const ECRAN_DE_LA_CLE: Record<string, string> = { [CLE_FILE_THEMATIQUES]: '/guide-nego/thematiques' }
const avisAnnonce = ref<{ texte: string; rang: number } | null>(null)
watch(avis, (suite) => {
  if (!suite?.message || route.path.replace(/\/$/, '') === ECRAN_DE_LA_CLE[suite.cle]) return
  avisAnnonce.value = { texte: suite.message, rang: (avisAnnonce.value?.rang ?? 0) + 1 }
})

// Le navigateur annonce le retour du réseau ; seule une lecture réussie le prouve, et
// c'est elle qui met à jour « Synchronisé à » — sans recharger la page.
function relire() {
  void rafraichir()
  void partir()
  void copies.partir()
}

// Trois déclencheurs, tous nécessaires : un téléphone rouvert le lendemain n'émet pas
// d'`online`, et une application restée ouverte ne se réouvre pas.
function auRetourAuPremierPlan() {
  if (document.visibilityState !== 'visible') return
  void partir()
  void copies.partir()
}

onMounted(() => {
  window.addEventListener('online', relire)
  document.addEventListener('visibilitychange', auRetourAuPremierPlan)
  enregistrerLaGarde()
  void assurerLeVocabulaire()
  void partir()
  // Le navigateur a pu vider une copie depuis la dernière ouverture : elle redevient
  // « non téléchargée » avant qu'un écran ne la montre, puis ce qui attend repart.
  void copies
    .verifier()
    .then(() => copies.partir())
    .catch(() => undefined)
})

onBeforeUnmount(() => {
  window.removeEventListener('online', relire)
  document.removeEventListener('visibilitychange', auRetourAuPremierPlan)
})

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
 * Résout dès que la garde est CONSTITUÉE, sans attendre qu'elle contrôle la page.
 * `navigator.serviceWorker.ready` attend le contrôle, qui n'arrive qu'au chargement
 * suivant : la personne qui entre puis referme n'apprenait jamais, à la seule ouverture
 * qui a pris le temps du réseau, que l'application tiendra sans réseau.
 */
function gardeConstituee(inscription: ServiceWorkerRegistration): Promise<void> {
  if (inscription.active) return Promise.resolve()
  const enCours = inscription.installing ?? inscription.waiting
  if (!enCours) return navigator.serviceWorker.ready.then(() => undefined)
  return new Promise((resoudre) => {
    enCours.addEventListener('statechange', () => {
      if (enCours.state === 'activated') resoudre()
    })
  })
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
    await gardeConstituee(inscription)
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
  <div data-app="guide-nego" :data-theme="affiche">
    <slot />
    <div id="gn-portail" />
    <GnMessageEphemere
      v-if="gardePrete"
      :texte="t('gn-connexion.prete')"
      @fini="gardePrete = false"
    />
    <GnMessageEphemere
      v-if="avisAnnonce"
      :key="avisAnnonce.rang"
      :texte="avisAnnonce.texte"
      @fini="avisAnnonce = null"
    />
  </div>
</template>
