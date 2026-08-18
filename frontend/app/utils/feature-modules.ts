/**
 * LES MODULES FERMÉS DU JALON, ET CE QUI LES FERME.
 *
 * Six espaces existent dans le modèle de données et n'ont pas d'interface dans
 * ce jalon : Publications, Négociations, Formations, Outils, Messagerie et
 * l'Annuaire — celui-ci porté par l'espace Communauté. Chacun est commandé par
 * un drapeau `<module>.enabled` de `platform.feature_flags`, semé à `false` par
 * `900_seed.sql` § 2.
 *
 * POURQUOI UN REGISTRE PLUTÔT QU'UN TEST DANS CHAQUE PAGE. Le prompt le demande
 * explicitement : le routage sert la page de maintenance « sans toucher aux
 * pages elles-mêmes ». Une page qui teste son propre drapeau, c'est six tests à
 * maintenir, six occasions d'en oublier un, et un module qu'on croit fermé alors
 * qu'il s'affiche. Ici, la seule chose à faire pour fermer un espace est de
 * l'inscrire dans cette liste ; l'ouvrir se fait en base, sans redéploiement.
 *
 * ON APPARIE PAR NOM DE ROUTE, PAS PAR CHEMIN. Une page déclare ses adresses par
 * `defineI18nRoute` — `/communaute` en français, `/en/community` en anglais — et
 * comparer des chemins obligerait à connaître les deux, dans les deux langues,
 * plus le préfixe de langue. Le nom de route, lui, est dérivé du fichier et ne
 * change pas d'une locale à l'autre : `@nuxtjs/i18n` lui ajoute seulement un
 * suffixe `___fr`. C'est le seul repère stable.
 *
 * DES ROUTES QUI N'EXISTENT PAS ENCORE SONT DÉJÀ DÉCLARÉES. Quatre des six
 * modules n'ont aucune page : les nommer ici ne coûte rien et évite qu'on crée
 * un jour `pages/formations.vue` en oubliant de la fermer. Un nom sans route est
 * inerte.
 */

import type { FeatureFlagKey } from '~/types/shared'

export interface ClosedModule {
  /**
   * Clé technique du module. Elle sert deux fois : segment d'adresse de la page
   * de maintenance (`/maintenance/directory`) et branche de traduction
   * (`maintenance.modules.directory`).
   *
   * VOLONTAIREMENT NON TRADUITE, contrairement au reste des adresses du site.
   * Cette page n'est pas une destination qu'on partage ou qu'on met en favori :
   * elle constate un état transitoire. Lui donner deux adresses par langue
   * ajouterait une table de correspondance à maintenir pour rien, et le jour où
   * le module ouvre, l'adresse disparaît.
   */
  key: string
  /** Le drapeau qui commande l'espace ENTIER — jamais un drapeau de détail. */
  flag: FeatureFlagKey
  /**
   * Noms de route couverts, sans le suffixe de langue. Un nom couvre aussi ses
   * descendants : `negotiations` ferme `negotiations-id` sans qu'on l'écrive.
   */
  routeNames: string[]
  /**
   * Chemin d'entrée de l'espace, sans préfixe de langue. Il sert au retour :
   * quelqu'un qui arrive sur la page de maintenance d'un module ENTRE-TEMPS
   * OUVERT est renvoyé là plutôt que de lire une annonce périmée.
   */
  entryPath: string | null
}

export const CLOSED_MODULES: ClosedModule[] = [
  {
    key: 'publications',
    flag: 'publications.enabled',
    routeNames: ['publications'],
    entryPath: null,
  },
  {
    key: 'negotiation',
    flag: 'negotiation.enabled',
    routeNames: ['negotiations'],
    entryPath: '/negociations',
  },
  {
    key: 'training',
    flag: 'training.enabled',
    routeNames: ['formations', 'training'],
    entryPath: null,
  },
  {
    key: 'tools',
    flag: 'tools.enabled',
    routeNames: ['outils', 'tools'],
    entryPath: null,
  },
  {
    key: 'messaging',
    flag: 'messaging.enabled',
    routeNames: ['messagerie', 'messaging'],
    entryPath: null,
  },
  {
    // L'ANNUAIRE ET LA COMMUNAUTÉ SONT LE MÊME ESPACE, sous deux noms. Le modèle
    // parle d'annuaire — `identity.people.is_directory_visible`, les fiches
    // d'organisation — et la barre de navigation parle de Communauté, parce que
    // c'est ce qu'un visiteur y cherche : des organisations, des personnes, et
    // ce qu'elles se disent. Un seul drapeau, donc, et un seul module.
    key: 'directory',
    flag: 'directory.enabled',
    routeNames: ['community'],
    entryPath: '/communaute',
  },
]

/** Le nom de route dépouillé de son suffixe de langue : `community___fr` → `community`. */
export function baseRouteName(name: unknown): string | null {
  if (typeof name !== 'string') return null
  const separator = name.indexOf('___')
  return separator === -1 ? name : name.slice(0, separator)
}

/**
 * Le module fermé auquel appartient une route, s'il y en a un.
 *
 * Le tiret du test descendant n'est pas décoratif : sans lui, `communityXyz`
 * serait apparié à `community`. Nuxt sépare les segments par un tiret, et c'est
 * la seule frontière fiable dans un nom de route.
 */
export function moduleOfRoute(routeName: unknown): ClosedModule | null {
  const base = baseRouteName(routeName)
  if (base === null) return null
  return (
    CLOSED_MODULES.find((module) =>
      module.routeNames.some((prefix) => base === prefix || base.startsWith(`${prefix}-`)),
    ) ?? null
  )
}

/** Le module portant cette clé — `null` si l'adresse a été forgée à la main. */
export function closedModuleByKey(key: unknown): ClosedModule | null {
  if (typeof key !== 'string') return null
  return CLOSED_MODULES.find((module) => module.key === key) ?? null
}
