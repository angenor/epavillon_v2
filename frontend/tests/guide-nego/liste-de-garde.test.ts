import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  empreinteDesMessages,
  fichiersDeConstruction,
  listeDeGarde,
  type ManifesteDeConstruction,
} from '../../guide-nego/liste-de-garde.ts'

/** Une entrée du site, une mise en page et une page de Guide Négo, et leurs imports. */
const manifeste: ManifesteDeConstruction = {
  'app/entry.js': {
    file: 'entry.hash.js',
    css: ['entry.hash.css'],
    isEntry: true,
    imports: ['commun.js'],
    // Les pages du SITE, atteintes dynamiquement depuis l'entrée.
    dynamicImports: ['pages/admin/tableau-de-bord.vue', 'pages/index.vue'],
  },
  'commun.js': { file: 'commun.hash.js' },
  'pages/admin/tableau-de-bord.vue': { file: 'admin.hash.js', imports: ['mocks.js'] },
  'pages/index.vue': { file: 'accueil.hash.js' },
  'mocks.js': { file: 'mocks.hash.js' },
  'layouts/guide-nego.vue': {
    file: 'layout-gn.hash.js',
    css: ['gn.hash.css'],
    assets: ['pictogrammes.hash.svg', 'Atkinson.hash.woff2'],
    imports: ['commun.js'],
  },
  'pages/guide-nego/index.vue': { file: 'gn-accueil.hash.js', dynamicImports: ['messages-gn.js'] },
  'messages-gn.js': { file: 'messages-gn.hash.js' },
}

test('les fichiers de Guide Négo sont gardés, avec leurs styles et leurs médias', () => {
  const fichiers = fichiersDeConstruction(manifeste)
  for (const attendu of [
    'layout-gn.hash.js',
    'gn.hash.css',
    'pictogrammes.hash.svg',
    'Atkinson.hash.woff2',
    'gn-accueil.hash.js',
  ]) {
    assert.ok(fichiers.includes(attendu), `${attendu} devrait être gardé`)
  }
})

test('l’entrée est gardée, mais pas les pages du site qu’elle charge à la demande', () => {
  const fichiers = fichiersDeConstruction(manifeste)
  assert.ok(fichiers.includes('entry.hash.js'), 'l’entrée est gardée')
  assert.ok(fichiers.includes('commun.hash.js'), 'ses imports statiques aussi')
  assert.ok(!fichiers.includes('admin.hash.js'), 'le back-office du site ne doit pas entrer')
  assert.ok(!fichiers.includes('accueil.hash.js'), 'l’accueil du site non plus')
  assert.ok(!fichiers.includes('mocks.hash.js'), 'ni ce qu’elles entraînent')
})

test('les imports dynamiques de Guide Négo, eux, sont suivis', () => {
  assert.ok(fichiersDeConstruction(manifeste).includes('messages-gn.hash.js'))
})

test('une liste vide se voit — elle ne se devine pas', () => {
  assert.deepEqual(fichiersDeConstruction({}), [])
})

test('l’empreinte des traductions se lit, guillemets ou accents graves', () => {
  const minifie = 'r=Se(_(e),`/_i18n`,{fr:`465112eb`,en:`465112eb`}[e],e,`messages.json`)'
  const lisible = 'joinURL(cdnPrefix(), "/_i18n", { "fr": "a1b2c3d4", "en": "a1b2c3d4" }[locale])'
  assert.equal(empreinteDesMessages(minifie), '465112eb')
  assert.equal(empreinteDesMessages(lisible), 'a1b2c3d4')
})

test('un paquet qui ne parle pas des traductions ne rend aucune empreinte', () => {
  assert.equal(empreinteDesMessages('const a = 1'), null)
})

test('toutes les adresses de la liste sont relatives au service worker', () => {
  const liste = listeDeGarde(['gn.hash.js'], '/_nuxt/', '465112eb')
  for (const adresse of liste) {
    assert.ok(!adresse.startsWith('/'), `${adresse} ne doit pas partir de la racine`)
  }
  assert.ok(liste.includes('../_nuxt/gn.hash.js'))
  assert.ok(liste.includes('../_i18n/465112eb/fr/messages.json'))
  assert.ok(liste.includes('./'), 'la page vide sert toute navigation')
  assert.ok(liste.includes('manifest.webmanifest'))
})

test('le dossier de construction s’écrit sans barres superflues', () => {
  assert.ok(listeDeGarde(['a.js'], '_nuxt', 'abc123').includes('../_nuxt/a.js'))
})
