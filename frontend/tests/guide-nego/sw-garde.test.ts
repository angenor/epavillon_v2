import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createContext, runInContext } from 'node:vm'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

/**
 * Le service worker éprouvé hors du navigateur : on lui donne de faux `caches` et un
 * faux `fetch`, et on regarde ce qu'il demande au réseau.
 *
 * C'est le seul moyen de prouver ce qui coûte le plus cher en salle — ne pas
 * retélécharger un mégaoctet à chaque déploiement du site — sans un vrai téléphone.
 */
const MODELE = join(dirname(fileURLToPath(import.meta.url)), '../../guide-nego/sw.modele.js')

const LISTE = [
  './',
  'manifest.webmanifest',
  'icones/192.png',
  '../_i18n/abc123/fr/messages.json',
  '../_nuxt/entry.hash.js',
  '../_nuxt/Atkinson.hash.woff2',
]

interface FauxCache {
  contenu: Map<string, string>
}

function monter(version: string, caches_ = new Map<string, FauxCache>()) {
  const demandees: string[] = []
  const ecouteurs = new Map<string, (evenement: unknown) => void>()
  let priseDeMain = false

  const cacheApi = {
    keys: async () => [...caches_.keys()],
    delete: async (cle: string) => caches_.delete(cle),
    open: async (cle: string) => {
      if (!caches_.has(cle)) caches_.set(cle, { contenu: new Map() })
      const cache = caches_.get(cle)!
      return {
        match: async (adresse: string) => cache.contenu.get(String(adresse)),
        put: async (adresse: string, reponse: string) => void cache.contenu.set(String(adresse), reponse),
        addAll: async (requetes: { url: string }[]) => {
          for (const requete of requetes) {
            demandees.push(requete.url)
            cache.contenu.set(requete.url, `réseau:${requete.url}`)
          }
        },
      }
    },
    match: async () => undefined,
  }

  const contexte = {
    self: {
      location: new URL('https://exemple.org/v2/guide-nego/sw.js'),
      addEventListener: (nom: string, ecouteur: (evenement: unknown) => void) => ecouteurs.set(nom, ecouteur),
      skipWaiting: () => void (priseDeMain = true),
      clients: { claim: async () => undefined },
    },
    caches: cacheApi,
    fetch: async (requete: { url: string }) => `réseau:${requete.url}`,
    Request: class {
      url: string
      constructor(url: string) {
        this.url = String(url)
      }
    },
    URL,
    console,
  }

  createContext(contexte)
  const source = readFileSync(MODELE, 'utf8')
    .replace("'__VERSION__'", JSON.stringify(version))
    .replace('__LISTE__', JSON.stringify(LISTE))
  runInContext(source, contexte)

  const declencher = async (nom: string) => {
    let attendu: Promise<unknown> = Promise.resolve()
    ecouteurs.get(nom)?.({ waitUntil: (promesse: Promise<unknown>) => (attendu = promesse), data: 'prendre-la-main' })
    await attendu
  }

  return { caches_, demandees, declencher, priseDeMain: () => priseDeMain }
}

const absolu = (adresse: string) => new URL(adresse, 'https://exemple.org/v2/guide-nego/sw.js').href

test('première garde : tout vient du réseau', async () => {
  const sw = monter('v1')
  await sw.declencher('install')
  assert.equal(sw.demandees.length, LISTE.length)
  assert.equal(sw.caches_.get('gn-coquille-v1')?.contenu.size, LISTE.length)
})

test('déploiement suivant : les fichiers de construction se reprennent, le reste se redemande', async () => {
  const anciens = new Map([
    [
      'gn-coquille-v1',
      {
        contenu: new Map([
          [absolu('../_nuxt/entry.hash.js'), 'v1'],
          [absolu('../_nuxt/Atkinson.hash.woff2'), 'v1'],
          [absolu('./'), 'v1'],
        ]),
      },
    ],
  ])
  const sw = monter('v2', anciens)
  await sw.declencher('install')

  // Un nom à empreinte ne change pas de contenu : on ne le redemande pas.
  assert.ok(!sw.demandees.some((a) => a.includes('_nuxt')), 'aucun fichier de construction redemandé')
  // La page vide, les traductions, le manifeste et les icônes changent sous la même adresse.
  assert.deepEqual(sw.demandees.sort(), [
    absolu('../_i18n/abc123/fr/messages.json'),
    absolu('./'),
    absolu('icones/192.png'),
    absolu('manifest.webmanifest'),
  ])
  assert.equal(sw.caches_.get('gn-coquille-v2')?.contenu.size, LISTE.length, 'le cache reste complet')
})

test('une installation interrompue reprend où elle s’était arrêtée', async () => {
  const caches_ = new Map([['gn-coquille-v1', { contenu: new Map([[absolu('./'), 'déjà là']]) }]])
  const sw = monter('v1', caches_)
  await sw.declencher('install')
  assert.ok(!sw.demandees.includes(absolu('./')), 'ce qui est déjà gardé n’est pas redemandé')
  assert.equal(sw.demandees.length, LISTE.length - 1)
})

test('le ménage ne touche QUE les caches de la coquille', async () => {
  const caches_ = new Map([
    ['gn-coquille-v1', { contenu: new Map() }],
    ['gn-coquille-v2', { contenu: new Map() }],
    // Ce que l'étape 1 gardera : des documents téléchargés, qui ne doivent pas partir
    // au premier déploiement du site.
    ['gn-documents', { contenu: new Map() }],
    ['autre-application', { contenu: new Map() }],
  ])
  const sw = monter('v2', caches_)
  await sw.declencher('activate')
  assert.deepEqual([...caches_.keys()].sort(), ['autre-application', 'gn-coquille-v2', 'gn-documents'])
})

test('la version en attente ne prend la main que si la page le demande', async () => {
  const sw = monter('v2')
  assert.equal(sw.priseDeMain(), false)
  await sw.declencher('message')
  assert.equal(sw.priseDeMain(), true)
})
