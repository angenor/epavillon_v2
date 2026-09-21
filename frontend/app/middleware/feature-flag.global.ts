/**
 * Middleware GLOBAL `feature-flag` — il sert la page « En cours de maintenance »
 * à la place d'un module éteint.
 *
 * LE PROMPT A14 EN DONNE LA RÈGLE : « le routage doit la servir automatiquement
 * quand le drapeau de fonctionnalité du module est désactivé, SANS TOUCHER AUX
 * PAGES ELLES-MÊMES ». D'où un middleware global et un registre
 * (`utils/feature-modules.ts`), et non un test recopié dans six pages — six
 * tests, c'est six occasions d'en oublier un, et un module qu'on croit fermé
 * alors qu'il s'affiche.
 *
 * POURQUOI UNE REDIRECTION ET PAS UN RENDU SUBSTITUÉ. Un middleware de
 * navigation peut détourner, pas réécrire : Vue Router n'offre aucun moyen
 * propre de garder l'adresse et de rendre un autre composant, et les montages
 * qui s'en approchent (manipuler `to.matched`, forcer un layout) cassent au
 * premier changement de version. La redirection a d'ailleurs une qualité que la
 * substitution n'a pas : l'adresse dit la vérité. Quelqu'un qui met la page en
 * favori ou qui l'envoie à un collègue partage une adresse de maintenance, pas
 * une adresse d'espace ouvert qui n'affichera jamais l'espace.
 *
 * DANS LES DEUX SENS. Le drapeau éteint envoie vers la page de maintenance ; le
 * drapeau allumé RENVOIE DEPUIS elle vers l'espace. Sans ce second sens, une
 * adresse de maintenance partagée trois mois plus tôt continuerait d'annoncer
 * comme fermé un module ouvert entre-temps — le contraire de ce que le prompt
 * demande, qui est d'être honnête sur l'état.
 *
 * UN MODULE PEUT AVOIR SA PROPRE PAGE FERMÉE (`closedPath`) — Guide Négo a son
 * design, et la page de maintenance est à celui du site. La règle reste ici : aucune
 * page ne teste son propre drapeau.
 *
 * CE N'EST PAS UN CONTRÔLE DE SÉCURITÉ : un middleware de navigation s'exécute
 * dans le navigateur. L'API ne servira pas davantage un module éteint.
 */
import type { ClosedModule } from '~/utils/feature-modules'

/**
 * Un module `lastKnown` — Guide Négo — se résout par son dernier état lu, gardé sur
 * l'appareil : une panne d'API ne le ferme pas. Rendu côté navigateur seulement.
 */
async function isOpen(module: ClosedModule): Promise<boolean> {
  if (module.lastKnown) {
    if (import.meta.server) return true
    const drapeaux = useGnDrapeaux()
    await drapeaux.assurer()
    return drapeaux.ouverte.value
  }
  const features = useFeatureStore()
  await features.ensureLoaded()
  return features.isEnabled(module.flag)
}

export default defineNuxtRouteMiddleware(async (to) => {
  const localePath = useLocalePath()

  // La page de maintenance elle-même : si le module a ouvert, on n'annonce pas
  // une fermeture qui n'a plus cours.
  if (baseRouteName(to.name) === 'maintenance-module') {
    const module = closedModuleByKey(to.params.module)
    if (module === null) return
    if (module.closedPath) return navigateTo(module.closedPath)
    if (module.entryPath === null) return

    if (!(await isOpen(module))) return
    return navigateTo(localePath(module.entryPath))
  }

  const module = moduleOfRoute(to.name)
  if (module === null) return

  const open = await isOpen(module)

  if (module.closedPath) {
    const onClosedPage = to.path.replace(/\/$/, '') === module.closedPath
    if (open && onClosedPage && module.entryPath) return navigateTo(module.entryPath)
    if (!open && !onClosedPage) return navigateTo(module.closedPath)
    return
  }

  if (open) return
  return navigateTo(localePath(`/maintenance/${module.key}`))
})
