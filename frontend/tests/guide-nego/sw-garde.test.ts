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
  'pdfjs/6.3.289/wasm/qcms_bg.wasm',
  'pdfjs/6.3.289/standard_fonts/LiberationSans-Regular.ttf',
  '../_i18n/abc123/fr/messages.json',
  '../_nuxt/entry.hash.js',
  '../_nuxt/travailleur.hash.js',
  '../_nuxt/Atkinson.hash.woff2',
  '../_nuxt/builds/meta/v1.json',
  '../_nuxt/builds/latest.json',
]

interface FauxCache {
  contenu: Map<string, string>
}

function monter(version: string, caches_ = new Map<string, FauxCache>()) {
  const demandees: string[] = []
  const ecouteurs = new Map<string, (evenement: unknown) => void>()
  let priseDeMain = false
  const reseau = { enLigne: true }

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
    match: async (adresse: string, options?: { cacheName?: string }) =>
      caches_.get(options?.cacheName ?? '')?.contenu.get(String(adresse)),
  }

  const contexte = {
    self: {
      location: new URL('https://exemple.org/v2/guide-nego/sw.js'),
      addEventListener: (nom: string, ecouteur: (evenement: unknown) => void) => ecouteurs.set(nom, ecouteur),
      skipWaiting: () => void (priseDeMain = true),
      clients: { claim: async () => undefined },
    },
    caches: cacheApi,
    fetch: async (requete: { url: string }) => {
      if (!reseau.enLigne) throw new TypeError('Failed to fetch')
      return `réseau:${requete.url}`
    },
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

  /** Une requête de la page : ce que le service worker répond, ou `null` s'il laisse passer. */
  const servir = async (url: string): Promise<unknown> => {
    let reponse: Promise<unknown> | null = null
    ecouteurs.get('fetch')?.({ request: { method: 'GET', mode: 'cors', url }, respondWith: (p: Promise<unknown>) => (reponse = p) })
    return reponse
  }

  return { caches_, demandees, declencher, servir, reseau, priseDeMain: () => priseDeMain }
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
          [absolu('../_nuxt/travailleur.hash.js'), 'v1'],
          [absolu('../_nuxt/Atkinson.hash.woff2'), 'v1'],
          [absolu('pdfjs/6.3.289/wasm/qcms_bg.wasm'), 'v1'],
          [absolu('pdfjs/6.3.289/standard_fonts/LiberationSans-Regular.ttf'), 'v1'],
          [absolu('./'), 'v1'],
          [absolu('../_nuxt/builds/latest.json'), 'v1'],
        ]),
      },
    ],
  ])
  const sw = monter('v2', anciens)
  await sw.declencher('install')

  // Un nom à empreinte ne change pas de contenu : on ne le redemande pas.
  assert.ok(
    !sw.demandees.some((a) => a.includes('_nuxt') && !a.includes('/builds/')),
    'aucun fichier de construction redemandé',
  )
  // La page vide, les traductions, le manifeste et les icônes changent sous la même adresse ;
  // le manifeste de construction de Nuxt aussi : il porte l'identifiant de la version.
  assert.deepEqual(sw.demandees.sort(), [
    absolu('../_i18n/abc123/fr/messages.json'),
    absolu('../_nuxt/builds/latest.json'),
    absolu('../_nuxt/builds/meta/v1.json'),
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

test('un déploiement laisse les documents téléchargés : gn-documents-publics et gn-documents-reserves', async () => {
  const caches_ = new Map([
    ['gn-coquille-v1', { contenu: new Map() }],
    ['gn-documents-publics', { contenu: new Map([['https://api/negotiation/documents/g/reading', '{}']]) }],
    ['gn-documents-reserves', { contenu: new Map([['https://api/negotiation/documents/r/reading', '{}']]) }],
  ])
  const sw = monter('v2', caches_)
  await sw.declencher('activate')
  assert.deepEqual([...caches_.keys()].sort(), ['gn-documents-publics', 'gn-documents-reserves'])
  assert.equal(caches_.get('gn-documents-publics')?.contenu.size, 1, 'rien n’est vidé dedans')
})

test('la version en attente ne prend la main que si la page le demande', async () => {
  const sw = monter('v2')
  assert.equal(sw.priseDeMain(), false)
  await sw.declencher('message')
  assert.equal(sw.priseDeMain(), true)
})

test('un déploiement du site ne fait pas reprendre pdf.js : sa version est dans le chemin', async () => {
  const anciens = new Map([
    [
      'gn-coquille-v1',
      {
        contenu: new Map(
          [
            'pdfjs/6.3.289/wasm/qcms_bg.wasm',
            'pdfjs/6.3.289/standard_fonts/LiberationSans-Regular.ttf',
            '../_nuxt/travailleur.hash.js',
          ].map((adresse) => [absolu(adresse), 'v1']),
        ),
      },
    ],
  ])
  const sw = monter('v2', anciens)
  await sw.declencher('install')
  assert.ok(!sw.demandees.some((a) => a.includes('/pdfjs/') || a.includes('travailleur')), 'rien de pdf.js redemandé')
  assert.equal(sw.caches_.get('gn-coquille-v2')?.contenu.get(absolu('pdfjs/6.3.289/wasm/qcms_bg.wasm')), 'v1')
})

test('une autre version de pdf.js est une autre adresse : elle se demande', async () => {
  const anciens = new Map([
    ['gn-coquille-v1', { contenu: new Map([[absolu('pdfjs/6.2.0/wasm/qcms_bg.wasm'), 'ancienne']]) }],
  ])
  const sw = monter('v2', anciens)
  await sw.declencher('install')
  assert.ok(sw.demandees.includes(absolu('pdfjs/6.3.289/wasm/qcms_bg.wasm')))
})

test('le manifeste de construction de Nuxt se garde : hors connexion, il répond sans erreur', async () => {
  const sw = monter('v1')
  await sw.declencher('install')
  sw.reseau.enLigne = false
  assert.equal(await sw.servir(absolu('../_nuxt/builds/meta/v1.json')), `réseau:${absolu('../_nuxt/builds/meta/v1.json')}`)
})

test('latest.json : le réseau d’abord, la copie de cette version ensuite, jamais réécrite', async () => {
  const sw = monter('v1')
  await sw.declencher('install')
  const garde = sw.caches_.get('gn-coquille-v1')?.contenu.get(absolu('../_nuxt/builds/latest.json'))
  // Nuxt le relit d'heure en heure avec l'heure dans l'adresse.
  const adresse = `${absolu('../_nuxt/builds/latest.json')}?1727200000000`
  assert.equal(await sw.servir(adresse), `réseau:${adresse}`, 'en ligne, la version publiée')
  assert.equal(sw.caches_.get('gn-coquille-v1')?.contenu.get(absolu('../_nuxt/builds/latest.json')), garde, 'la copie gardée ne change pas')
  sw.reseau.enLigne = false
  assert.equal(await sw.servir(adresse), garde, 'hors connexion, celle de cette version : aucune mise à jour annoncée')
})
