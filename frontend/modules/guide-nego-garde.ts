import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { createResolver, defineNuxtModule } from '@nuxt/kit'
import {
  avecLesFichiersDesignes,
  empreinteDesMessages,
  fichiersDeConstruction,
  listeDeGarde,
  type ManifesteDeConstruction,
} from '../guide-nego/liste-de-garde'

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
 * Le calcul lui-même vit dans `guide-nego/liste-de-garde.ts`, où il se teste. Ici : lire
 * le manifeste, écrire le fichier.
 *
 * En développement, il n'y a pas de manifeste de construction : pas de service worker
 * non plus, et le hors-connexion se vérifie sur une construction (quickstart).
 */

const PORTEE = 'guide-nego'

/** Cherche l'empreinte des traductions dans les paquets qui viennent d'être écrits. */
function empreinteDansLesPaquets(dossierClient: string, fichiers: string[]): string {
  for (const fichier of fichiers) {
    if (!fichier.endsWith('.js')) continue
    const chemin = join(dossierClient, fichier)
    if (!existsSync(chemin)) continue
    const empreinte = empreinteDesMessages(readFileSync(chemin, 'utf8'))
    if (empreinte) return empreinte
  }
  throw new Error(
    'guide-nego-garde : l’adresse des traductions est introuvable dans le paquet. ' +
      '@nuxtjs/i18n a-t-il changé sa façon de les servir ? Sans elle, l’application ' +
      's’ouvrirait hors connexion en affichant ses clés.',
  )
}

export default defineNuxtModule({
  meta: { name: 'guide-nego-garde', configKey: 'guideNegoGarde' },
  setup(_options, nuxt) {
    if (nuxt.options.dev) return

    const resolver = createResolver(import.meta.url)
    const modele = resolver.resolve('../guide-nego/sw.modele.js')
    // Le fichier s'écrit dans `buildDir` puis se COPIE dans la sortie publique, une
    // fois Nitro passé. Le déclarer comme dossier de fichiers publics servait le
    // service worker, mais faisait rendre 404 à toute adresse de `guide-nego/` qui
    // n'était pas un fichier — Nitro cessait de laisser la main au rendu de page, et
    // une première ouverture sur un lien profond échouait. Le service worker, en
    // servant la page vide de son cache, masquait la panne à partir de la deuxième.
    // Le dossier se crée AU MOMENT D'ÉCRIRE : Nuxt vide `buildDir` après le montage
    // des modules.
    const dossier = join(nuxt.options.buildDir, 'guide-nego-public')

    nuxt.hook('nitro:build:public-assets', (nitro) => {
      const engendre = join(dossier, 'sw.js')
      if (!existsSync(engendre)) return
      const destination = join(nitro.options.output.publicDir, PORTEE, 'sw.js')
      mkdirSync(dirname(destination), { recursive: true })
      copyFileSync(engendre, destination)
    })

    nuxt.hook('build:manifest', (manifeste) => {
      const construction = nuxt.options.app.buildAssetsDir
      const dossierClient = join(nuxt.options.buildDir, 'dist/client', construction.replace(/^\/+|\/+$/g, ''))
      const lire = (fichier: string) =>
        existsSync(join(dossierClient, fichier)) ? readFileSync(join(dossierClient, fichier), 'utf8') : null
      const fichiers = avecLesFichiersDesignes(fichiersDeConstruction(manifeste as ManifesteDeConstruction), lire)

      if (fichiers.length === 0) {
        throw new Error(
          'guide-nego-garde : la liste de garde est vide — la mise en page ou les pages de Guide Négo ' +
            'ont-elles changé de chemin ? Une application qui ne garde rien ne s’ouvre pas en salle.',
        )
      }

      const liste = listeDeGarde(fichiers, construction, empreinteDansLesPaquets(dossierClient, fichiers))

      const source = readFileSync(modele, 'utf8')
        .replace("'__VERSION__'", JSON.stringify(nuxt.options.runtimeConfig.app.buildId))
        .replace('__LISTE__', JSON.stringify(liste, null, 2))

      mkdirSync(dossier, { recursive: true })
      writeFileSync(join(dossier, 'sw.js'), source, 'utf8')
      console.info(`guide-nego-garde : sw.js engendré, ${liste.length} fichiers gardés.`)
    })
  },
})
