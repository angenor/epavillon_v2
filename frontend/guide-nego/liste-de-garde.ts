/**
 * Ce que le service worker doit garder — la partie qui se raisonne, et se teste.
 *
 * Vit à côté de `sw.modele.js` et non sous `modules/`, que Nuxt balaie pour y trouver
 * des modules. `modules/guide-nego-garde.ts` l'appelle, `tests/guide-nego/` l'éprouve.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

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

/** Le paquet de pdf.js, tel que l'installe `npm`. */
export const RACINE_PDFJS = join(dirname(fileURLToPath(import.meta.url)), '../node_modules/pdfjs-dist')

export const VERSION_PDFJS: string = JSON.parse(readFileSync(join(RACINE_PDFJS, 'package.json'), 'utf8')).version

/**
 * Sous `public/guide-nego/`, là où `modules/guide-nego-pdfjs.ts` copie les ressources.
 * La version dans le chemin : même adresse, même contenu — la coquille les reprend
 * d'un déploiement du site à l'autre, comme les fichiers à empreinte.
 */
export const DOSSIER_PDFJS = `pdfjs/${VERSION_PDFJS}`

/**
 * Ce que pdf.js lit par adresse (ADR-022, R4) : ni `cmaps/` ni `jbig2` (le guide n'en a
 * pas, T002), jamais `quickjs`. Les dossiers se prennent en entier, polices Liberation comprises.
 */
export const RESSOURCES_PDFJS = ['wasm/qcms_bg.wasm', 'wasm/openjpeg.wasm', 'iccs/', 'standard_fonts/']

/** Les fichiers des ressources de pdf.js, relatifs au paquet : la copie et la garde lisent la même liste. */
export function fichiersPdfjs(racine: string = RACINE_PDFJS): string[] {
  const lister = (chemin: string): string[] =>
    statSync(join(racine, chemin)).isDirectory()
      ? readdirSync(join(racine, chemin)).flatMap((nom) => lister(`${chemin.replace(/\/$/, '')}/${nom}`))
      : [chemin]
  return RESSOURCES_PDFJS.flatMap(lister).sort()
}

/** Ce dont la garde a besoin, en plus des fichiers de construction. */
export const FICHIERS_PUBLICS = [
  './',
  'manifest.webmanifest',
  'icones/192.png',
  'icones/512.png',
  'icones/192-masque.png',
  'icones/512-masque.png',
  'icones/180.png',
  ...fichiersPdfjs().map((fichier) => `${DOSSIER_PDFJS}/${fichier}`),
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
 * Les fichiers qu'un paquet désigne par `new URL('…', import.meta.url)`. Le travailleur
 * de pdf.js en est un : Vite le construit à part et ne le nomme pas dans son manifeste.
 */
export function fichiersDesignes(contenuDuPaquet: string): string[] {
  const motif = /new URL\(\s*[`'"]([\w.-]+\.\w+)[`'"]\s*,\s*import\.meta\.url\s*\)/g
  return [...contenuDuPaquet.matchAll(motif)].map((trouve) => trouve[1]!)
}

/** Ajoute aux fichiers gardés ceux qu'ils désignent, de proche en proche. `lire` rend `null` pour un fichier absent. */
export function avecLesFichiersDesignes(fichiers: string[], lire: (fichier: string) => string | null): string[] {
  const gardes = new Set(fichiers)
  const aLire = fichiers.filter((fichier) => fichier.endsWith('.js'))
  while (aLire.length > 0) {
    const contenu = lire(aLire.pop()!)
    if (contenu === null) continue
    for (const designe of fichiersDesignes(contenu)) {
      if (gardes.has(designe) || lire(designe) === null) continue
      gardes.add(designe)
      if (designe.endsWith('.js')) aLire.push(designe)
    }
  }
  return [...gardes].sort()
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

/**
 * Des adresses RELATIVES au service worker : le préfixe `/v2/` reste sans objet. Le
 * manifeste de construction de Nuxt en fait partie : sans lui, chaque navigation hors
 * connexion écrit une erreur (NUXT_E5002).
 */
export function listeDeGarde(
  fichiers: string[],
  dossierDeConstruction: string,
  empreinte: string,
  idDeConstruction: string,
  locale = 'fr',
): string[] {
  const construction = dossierDeConstruction.replace(/^\/+|\/+$/g, '')
  return [
    ...FICHIERS_PUBLICS,
    `../_i18n/${empreinte}/${locale}/messages.json`,
    `../${construction}/builds/meta/${idDeConstruction}.json`,
    `../${construction}/builds/latest.json`,
    ...fichiers.map((fichier) => `../${construction}/${fichier}`),
  ]
}
