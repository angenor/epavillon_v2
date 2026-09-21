/**
 * Ce que le service worker doit garder — la partie qui se raisonne, et se teste.
 *
 * Vit à côté de `sw.modele.js` et non sous `modules/`, que Nuxt balaie pour y trouver
 * des modules. `modules/guide-nego-garde.ts` l'appelle, `tests/guide-nego/` l'éprouve.
 */

export interface EntreeDeManifeste {
  file?: string
  src?: string
  css?: string[]
  assets?: string[]
  imports?: string[]
  dynamicImports?: string[]
  isEntry?: boolean
}

export type ManifesteDeConstruction = Record<string, EntreeDeManifeste>

/** Ce dont la garde a besoin, en plus des fichiers de construction. */
export const FICHIERS_PUBLICS = [
  './',
  'manifest.webmanifest',
  'icones/192.png',
  'icones/512.png',
  'icones/192-masque.png',
  'icones/512-masque.png',
  'icones/180.png',
]

/** La mise en page, les pages, et le paquet de la locale servie. L'entrée est à part. */
export function estRacineDeGuideNego(cle: string, meta: EntreeDeManifeste): boolean {
  const source = meta.src ?? cle
  return (
    source.includes('layouts/guide-nego.vue') ||
    source.includes('pages/guide-nego/') ||
    /locales\/(\.generated\/)?fr(\.ts)?$/.test(source)
  )
}

/**
 * Les fichiers de construction de Guide Négo, et rien du site.
 *
 * Les imports dynamiques de L'ENTRÉE sont toutes les pages du site : les suivre
 * garderait le site entier. On ne les suit que depuis Guide Négo, dont les dynamiques
 * sont ses propres écrans.
 */
export function fichiersDeConstruction(manifeste: ManifesteDeConstruction): string[] {
  const vus = new Set<string>()
  const fichiers = new Set<string>()

  const suivre = (cle: string, avecDynamiques: boolean) => {
    const marque = `${cle}|${avecDynamiques}`
    if (vus.has(marque)) return
    vus.add(marque)
    const meta = manifeste[cle]
    if (!meta) return
    for (const fichier of [meta.file, ...(meta.css ?? []), ...(meta.assets ?? [])]) {
      if (fichier) fichiers.add(fichier)
    }
    for (const importe of meta.imports ?? []) suivre(importe, avecDynamiques)
    if (avecDynamiques) for (const importe of meta.dynamicImports ?? []) suivre(importe, true)
  }

  for (const [cle, meta] of Object.entries(manifeste)) {
    if (meta.isEntry) suivre(cle, false)
    else if (estRacineDeGuideNego(cle, meta)) suivre(cle, true)
  }

  return [...fichiers].sort()
}

/**
 * L'empreinte de la route qui sert les traductions.
 *
 * `@nuxtjs/i18n` v10 ne met PAS les messages dans le paquet client quand le site rend
 * côté serveur : il compte sur le rendu pour les injecter, et le client les redemande à
 * `/_i18n/<empreinte>/fr/messages.json`. Sous `ssr: false`, aucun rendu ne les a
 * injectées : sans cette adresse dans la liste, l'application s'ouvre hors connexion en
 * affichant ses clés au lieu de ses textes.
 *
 * L'empreinte suit le contenu des fichiers de traduction : on la lit dans le paquet qui
 * vient d'être écrit, plutôt que de recalculer une formule interne au module.
 */
export function empreinteDesMessages(contenuDuPaquet: string, locale = 'fr'): string | null {
  // Le paquet est minifié : les chaînes peuvent être entre guillemets ou entre accents graves.
  const motif = new RegExp(`_i18n["'\`][\\s\\S]{0,400}?["'\`]?${locale}["'\`]?\\s*:\\s*["'\`]([0-9a-f]{6,16})["'\`]`)
  return contenuDuPaquet.match(motif)?.[1] ?? null
}

/** Des adresses RELATIVES au service worker : le préfixe `/v2/` reste sans objet. */
export function listeDeGarde(
  fichiers: string[],
  dossierDeConstruction: string,
  empreinte: string,
  locale = 'fr',
): string[] {
  const construction = dossierDeConstruction.replace(/^\/+|\/+$/g, '')
  return [
    ...FICHIERS_PUBLICS,
    `../_i18n/${empreinte}/${locale}/messages.json`,
    ...fichiers.map((fichier) => `../${construction}/${fichier}`),
  ]
}
