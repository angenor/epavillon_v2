/**
 * ENREGISTREMENT D'APEXCHARTS — côté navigateur, et À LA DEMANDE.
 *
 * POURQUOI `.client` ET NON UN GREFFON ORDINAIRE. ApexCharts mesure le DOM pour
 * dessiner : il n'a rien à produire au rendu serveur, et l'importer là-bas
 * embarquerait une bibliothèque de graphiques dans le paquet SSR pour un résultat
 * vide.
 *
 * POURQUOI UN IMPORT DYNAMIQUE ET NON `app.use()`. Un greffon client appartient au
 * paquet d'ENTRÉE : enregistrer le composant depuis un import de tête faisait
 * télécharger les ~500 ko d'ApexCharts sur CHAQUE page, y compris la page publique
 * d'un événement, qui n'affiche aucun graphique. Mesuré dans le navigateur : la
 * requête partait sur `/evenements/cop31-belem-2027`. Déclaré en composant
 * asynchrone, le paquet ne part qu'au premier `<apexchart>` réellement rendu —
 * donc dans le back-office, et seulement sur les écrans qui en portent un.
 *
 * LES ÉCRANS L'ENVELOPPENT DANS `<ClientOnly>` (voir `UiChart`), qui réserve la
 * hauteur du tracé pendant le chargement : sans cette réserve, la page sauterait
 * au moment où le graphique arrive.
 *
 * LE COMPOSANT S'APPELLE `apexchart`, nom que la bibliothèque emploie dans sa
 * documentation. Son typage pour `vue-tsc` vit dans `types/apexcharts.d.ts`.
 */
export default defineNuxtPlugin((nuxtApp) => {
  nuxtApp.vueApp.component(
    'apexchart',
    defineAsyncComponent(() => import('vue3-apexcharts').then((module) => module.default)),
  )
})
