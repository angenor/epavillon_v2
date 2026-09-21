import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { createResolver, defineNuxtModule } from '@nuxt/kit'

/**
 * Engendre `guide-nego/sw.js` : la liste EXACTE des fichiers de Guide Négo, et rien du
 * site.
 *
 * POURQUOI UN MODULE LOCAL ET PAS UN MODULE PWA
 * ---------------------------------------------
 * Un module de la place vise l'application entière : sa portée est la racine, et son
 * précache prend tout ce que la construction produit, faute de savoir ce qui est à
 * Guide Négo. Ici, une seule chose est demandée — la fermeture des imports de la mise
 * en page `guide-nego`, de ses pages et du paquet de locale `fr` —, et le manifeste de
 * construction la donne. Voir ADR-019 et research R5.
 *
 * `en` n'est pas gardé : il n'est pas servi à cette étape, et ces octets se paieraient
 * sur un réseau saturé. Il rejoindra la liste avec le choix de langue.
 *
 * En développement, il n'y a pas de manifeste de construction : pas de service worker
 * non plus, et le hors-connexion se vérifie sur une construction (quickstart).
 */

const PORTEE = 'guide-nego'

/** Ce dont la garde a besoin, en plus des fichiers de construction. */
const FICHIERS_PUBLICS = [
  './',
  'manifest.webmanifest',
  'icones/192.png',
  'icones/512.png',
  'icones/192-masque.png',
  'icones/512-masque.png',
  'icones/180.png',
]

/**
 * L'adresse où le client va chercher ses traductions.
 *
 * `@nuxtjs/i18n` v10 ne les met PAS dans le bundle quand le site rend côté serveur : il
 * compte sur le rendu pour les injecter, et le client les redemande à une route,
 * `/_i18n/<empreinte>/fr/messages.json`. Sous `ssr: false`, aucun rendu ne les a
 * injectées : sans cette adresse dans la liste, l'application s'ouvre hors connexion en
 * affichant ses clés au lieu de ses textes. L'empreinte suit le contenu des fichiers de
 * traduction, donc on la lit dans le bundle qui vient d'être écrit.
 */
function adresseDesMessages(dossierClient: string, fichiers: Iterable<string>): string {
  // Le bundle est minifié : les chaînes peuvent être entre guillemets ou entre accents graves.
  const motif = /_i18n[`"'][\s\S]{0,400}?[`"']?fr[`"']?\s*:\s*[`"']([0-9a-f]{6,16})[`"']/
  for (const fichier of fichiers) {
    if (!fichier.endsWith('.js')) continue
    const chemin = join(dossierClient, fichier)
    if (!existsSync(chemin)) continue
    const empreinte = readFileSync(chemin, 'utf8').match(motif)?.[1]
    if (empreinte) return `../_i18n/${empreinte}/fr/messages.json`
  }
  throw new Error(
    'guide-nego-garde : l’adresse des traductions est introuvable dans le bundle. ' +
      '@nuxtjs/i18n a-t-il changé sa façon de les servir ? Sans elle, l’application ' +
      's’ouvrirait hors connexion en affichant ses clés.',
  )
}

/** La mise en page, les pages, et le paquet de la locale servie. L'entrée est à part. */
function estRacineDeGuideNego(cle: string, meta: { src?: string }): boolean {
  const source = meta.src ?? cle
  return (
    source.includes('layouts/guide-nego.vue') ||
    source.includes('pages/guide-nego/') ||
    /locales\/(\.generated\/)?fr(\.ts)?$/.test(source)
  )
}

export default defineNuxtModule({
  meta: { name: 'guide-nego-garde', configKey: 'guideNegoGarde' },
  setup(_options, nuxt) {
    if (nuxt.options.dev) return

    const resolver = createResolver(import.meta.url)
    const modele = resolver.resolve('../guide-nego/sw.modele.js')
    // Nitro copie ses fichiers publics pendant sa propre construction, après celle du
    // client : le service worker, écrit au manifeste, y est déjà. Le dossier se crée
    // AU MOMENT D'ÉCRIRE — Nuxt vide `buildDir` après le montage des modules.
    const dossier = join(nuxt.options.buildDir, 'guide-nego-public')

    nuxt.hook('nitro:config', (config) => {
      config.publicAssets ||= []
      config.publicAssets.push({ dir: dossier, baseURL: `/${PORTEE}`, maxAge: 0 })
    })

    nuxt.hook('build:manifest', (manifeste) => {
      const vus = new Set<string>()
      const fichiers = new Set<string>()

      /**
       * Les imports dynamiques de L'ENTRÉE sont toutes les pages du site : les suivre
       * garderait le site entier. On ne les suit que depuis Guide Négo, dont les
       * dynamiques sont ses propres écrans.
       */
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

      if (fichiers.size === 0) {
        throw new Error(
          'guide-nego-garde : la liste de garde est vide — la mise en page ou les pages de Guide Négo ' +
            'ont-elles changé de chemin ? Une application qui ne garde rien ne s’ouvre pas en salle.',
        )
      }

      // Des adresses RELATIVES au service worker : le préfixe `/v2/` reste sans objet
      // (research R4). `sw.js` vit dans `guide-nego/`, la construction dans `_nuxt/`.
      const construction = nuxt.options.app.buildAssetsDir.replace(/^\/+|\/+$/g, '')
      const dossierClient = join(nuxt.options.buildDir, 'dist/client', construction)
      const liste = [
        ...FICHIERS_PUBLICS,
        adresseDesMessages(dossierClient, fichiers),
        ...[...fichiers].sort().map((fichier) => `../${construction}/${fichier}`),
      ]

      const source = readFileSync(modele, 'utf8')
        .replace("'__VERSION__'", JSON.stringify(nuxt.options.runtimeConfig.app.buildId))
        .replace('__LISTE__', JSON.stringify(liste, null, 2))

      mkdirSync(dossier, { recursive: true })
      writeFileSync(join(dossier, 'sw.js'), source, 'utf8')
      console.info(`guide-nego-garde : sw.js engendré, ${liste.length} fichiers gardés.`)
    })
  },
})
