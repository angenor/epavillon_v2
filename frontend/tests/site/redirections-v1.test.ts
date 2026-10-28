import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import { join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { redirectionV1, type TableRedirectionsV1 } from '../../server/utils/redirections-v1.ts'

const ACTIVITE_PUBLIEE = '11111111-1111-4111-8111-111111111111'
const ACTIVITE_NON_PUBLIEE = '22222222-2222-4222-8222-222222222222'
const EDITION = '33333333-3333-4333-8333-333333333333'
const INCONNU = '44444444-4444-4444-8444-444444444444'

const table: TableRedirectionsV1 = {
  activites: {
    [ACTIVITE_PUBLIEE]: '/programmations/cdp30-belem-2025/une-activite',
    [ACTIVITE_NON_PUBLIEE]: '/programmations?edition=cdp30-belem-2025',
  },
  editions: {
    [EDITION]: { page: '/evenements/cdp30-belem-2025', programmation: '/programmations?edition=cdp30-belem-2025' },
  },
  journees: {
    'journee-jeunesse': '/programmations?edition=cdp30-belem-2025&vue=liste&jour=2025-11-12',
  },
}

const r = (chemin: string, requete = '') => redirectionV1(chemin, requete, table)

test('activité : séance publiée, non publiée, inconnue, barre finale, sous-pages', () => {
  assert.equal(r(`/activities/${ACTIVITE_PUBLIEE}`), '/programmations/cdp30-belem-2025/une-activite')
  assert.equal(r(`/activities/${ACTIVITE_PUBLIEE}/`), '/programmations/cdp30-belem-2025/une-activite')
  assert.equal(r(`/activities/${ACTIVITE_PUBLIEE.toUpperCase()}`), '/programmations/cdp30-belem-2025/une-activite')
  assert.equal(r(`/activities/preview/${ACTIVITE_PUBLIEE}`), '/programmations/cdp30-belem-2025/une-activite')
  assert.equal(r(`/activities/${ACTIVITE_PUBLIEE}/manage`), '/programmations/cdp30-belem-2025/une-activite')
  assert.equal(r(`/activities/${ACTIVITE_NON_PUBLIEE}`), '/programmations?edition=cdp30-belem-2025')
  assert.equal(r(`/activities/${INCONNU}`), '/programmations')
  assert.equal(r('/activities/pas-un-identifiant'), '/programmations')
  assert.equal(r('/activities'), '/programmations')
})

test('édition : page de l’événement et programmation par année', () => {
  assert.equal(r(`/events/${EDITION}`), '/evenements/cdp30-belem-2025')
  assert.equal(r(`/events/${EDITION}/edit`), '/evenements/cdp30-belem-2025')
  assert.equal(r(`/events/${INCONNU}`), '/programmations')
  assert.equal(r('/events/create'), '/mon-organisation')
  assert.equal(r('/events/dashboard'), '/mon-organisation')
  assert.equal(r(`/programmations/2025/${EDITION}`), '/programmations?edition=cdp30-belem-2025')
  assert.equal(r(`/programmations/2025/${INCONNU}`), '/programmations')
})

test('journées spéciales et séminaire de Chypre', () => {
  assert.equal(r('/programmations/2025/journee-jeunesse'), '/programmations?edition=cdp30-belem-2025&vue=liste&jour=2025-11-12')
  assert.equal(r('/programmations/2025/journee-finance'), null, 'absente de la table : la v2 répond')
  assert.equal(r('/programmations/2025/seminaire-chypre'), '/programmations')
  assert.equal(r('/programmations/2025/constructor'), null)
})

test('pages simples et espaces fermés', () => {
  const attendu: Record<string, string> = {
    '/about': '/',
    '/login': '/connexion',
    '/register': '/inscription',
    '/verify-email': '/connexion',
    '/auth/callback': '/connexion',
    '/forgot-password': '/mot-de-passe-oublie',
    '/reset-password': '/mot-de-passe-oublie',
    '/profile': '/mon-organisation',
    '/organization/setup': '/mon-organisation',
    '/community': '/communaute',
    '/organizations': '/communaute',
    '/organizations/abc': '/communaute',
    '/directory/abc': '/communaute',
    '/nego': '/negociations',
    '/nego/climat': '/negociations',
    '/ai/chatbot': '/negociations',
    '/formations': '/maintenance/training',
    '/formations/abc': '/maintenance/training',
    '/messages': '/maintenance/messaging',
    '/notifications': '/',
    '/fphn2026/v2': '/',
    '/admin/users/abc': '/admin',
    '/admin/activities': '/admin',
  }
  for (const [v1, v2] of Object.entries(attendu)) assert.equal(r(v1), v2, v1)
})

test('chaîne de requête : gardée, sauf les adresses v1 ; la cible l’emporte', () => {
  assert.equal(r('/community', '?utm_source=x'), '/communaute?utm_source=x')
  assert.equal(r('/login', '?redirect=%2Factivities%2Fabc&utm_medium=mail'), '/connexion?utm_medium=mail')
  assert.equal(
    r(`/programmations/2025/${EDITION}`, '?edition=autre&fbclid=1'),
    '/programmations?edition=cdp30-belem-2025&fbclid=1',
  )
})

test('PACO, accueil et fichiers ne sont jamais redirigés', () => {
  for (const chemin of ['/', '/paco', '/paco/join', '/paco/admin', '/_nuxt/x.js', '/favicon.svg', '/admin', '/maintenance/training'])
    assert.equal(r(chemin), null, chemin)
})

// Toute adresse de page de la v2, dans les deux langues, avec des paramètres
// plausibles : aucune ne doit être prise pour une adresse v1. Le slug d'édition
// porte toujours le sigle, jamais une année seule.
function cheminsV2(): string[] {
  const racine = fileURLToPath(new URL('../../app/pages', import.meta.url))
  const fichiers = (dossier: string): string[] =>
    readdirSync(dossier, { withFileTypes: true }).flatMap((entree) =>
      entree.isDirectory() ? fichiers(join(dossier, entree.name)) : [join(dossier, entree.name)],
    )
  const modeles: string[] = []
  for (const fichier of fichiers(racine).filter((f) => f.endsWith('.vue'))) {
    const declares = [...readFileSync(fichier, 'utf8').matchAll(/(fr|en): '(\/[^']*)'/g)].map(
      ([, langue, chemin = '']) => (langue === 'en' ? `/en${chemin}` : chemin),
    )
    const parFichier = `/${relative(racine, fichier).replace(/\.vue$/, '').replace(/(^|\/)index$/, '')}`
    modeles.push(...(declares.length ? declares : [parFichier, `/en${parFichier}`]))
  }
  const exemples = ['2025', ACTIVITE_PUBLIEE, 'journee-jeunesse', 'seminaire-chypre', 'climat', 'constructor']
  return modeles.flatMap((modele) =>
    exemples.map((exemple) => modele.replace('[edition]', 'cdp30-belem-2025').replace(/\[[^\]]+\]/g, exemple)),
  )
}

test('aucune adresse de la v2 n’est interceptée', () => {
  const chemins = cheminsV2()
  assert.ok(chemins.includes(`/programmations/cdp30-belem-2025/${ACTIVITE_PUBLIEE}`))
  assert.ok(chemins.includes('/en/events/2025'))
  for (const chemin of chemins) assert.equal(r(chemin), null, chemin)
})

test('aucune cible ne se redirige encore : pas de boucle', () => {
  const generee: TableRedirectionsV1 = JSON.parse(
    readFileSync(new URL('../../server/utils/redirections-v1.json', import.meta.url), 'utf8'),
  )
  const cibles = [
    ...Object.values(generee.activites),
    ...Object.values(generee.editions).flatMap((edition) => [edition.page, edition.programmation]),
    ...Object.values(generee.journees),
    ...['/about', '/login', '/nego', '/formations', '/messages', '/events/create', '/admin/users'].map(
      (chemin) => r(chemin) ?? '',
    ),
  ]
  assert.ok(Object.keys(generee.activites).length > 0)
  for (const cible of cibles) {
    const [chemin = '', requete = ''] = cible.split('?')
    assert.equal(redirectionV1(chemin, requete, generee), null, cible)
  }
})
