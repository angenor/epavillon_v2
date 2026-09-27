import { test } from 'node:test'
import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { registerHooks } from 'node:module'
import { dirname, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import type { GlossaryEntry } from '../../app/types/negotiation-savoir.ts'
import { chercher, distanceBornee, preparerIndex, type ACherche } from '../../app/utils/guide-nego/recherche-floue.ts'

// Le jeu d'exemple s'écrit pour Nuxt : `~/` et les imports sans extension se résolvent ici.
const APP = fileURLToPath(new URL('../../app/', import.meta.url))
registerHooks({
  resolve(specifier, context, nextResolve) {
    const parent = context.parentURL?.startsWith('file:') ? dirname(fileURLToPath(context.parentURL)) : null
    let chemin = specifier.startsWith('~/')
      ? resolve(APP, specifier.slice(2))
      : specifier.startsWith('.') && parent
        ? resolve(parent, specifier)
        : null
    if (chemin && !/\.[cm]?[jt]s$/.test(chemin)) {
      chemin = [`${chemin}.ts`, `${chemin}/index.ts`].find((c) => existsSync(c)) ?? chemin
    }
    return nextResolve(chemin ? pathToFileURL(chemin).href : specifier, context)
  },
})

const { paquetDuSavoir } = await import('../../app/mocks/negotiation-savoir.ts')
const LEXIQUE = paquetDuSavoir('fr').valeur.glossary

const aChercher = (e: GlossaryEntry): ACherche<GlossaryEntry> => ({
  valeur: e,
  noms: [e.term, e.acronym, ...e.variants],
  traductions: [e.translation],
})

// Une faute de frappe par terme, et la traduction française quand elle diffère du terme.
const SAISIES: [terme: string, faute: string, francais: string | null][] = [
  ['contact group', 'contcat group', 'groupe de contact'],
  ['informal consultations', 'informal consultatoins', 'consultations informelles'],
  ['informal informals', 'infromal informals', null],
  ['huddle', 'hudle', 'aparte'],
  ['co-facilitators', 'co-faciltators', 'co-facilitateurs'],
  ['informal stocktaking plenary', 'stocktaking plenry', 'plénière informelle de bilan'],
  ['bracketed text', 'braketed text', 'texte entre crochets'],
  ['agreed language', 'agreed langage', 'formulation convenue'],
  ['ad ref.', 'ad reff', 'ad referendum'],
  ['PP / OP', 'pp opp', 'alinéa du préambule'],
  ['bis, ter, alt', 'bis ter atl', null],
  ['L document', 'L documnet', 'document L'],
  ['non-paper', 'non papr', 'note informelle'],
  ['landing zone', 'landing zoen', "zone d'atterrissage"],
  ['conclusions', 'conclusins', null],
  ['global goal on adaptation', 'global goal on adaptaton', 'objectif mondial adaptation'],
  ['nationally determined contribution', 'nationaly determined contribution', 'contribution déterminée'],
  ['loss and damage', 'loss and damge', 'pertes et prejudices'],
  ['global stocktake', 'global stoktake', 'bilan mondial'],
]

test('les dix-neuf termes, avec une faute ou en français, sont dans les trois premiers (SC-003)', () => {
  assert.equal(LEXIQUE.length, 19)
  const index = preparerIndex(LEXIQUE.map(aChercher))
  for (const [terme, faute, francais] of SAISIES) {
    for (const saisie of [faute, francais]) {
      if (!saisie) continue
      const premiers = chercher(index, saisie).trouves.slice(0, 3).map((t) => t.valeur.term)
      assert.ok(premiers.includes(terme), `« ${saisie} » → ${premiers.join(', ')}`)
    }
  }
})

test('les rangs : égalité, début, sous-chaîne, approché', () => {
  const index = preparerIndex(LEXIQUE.map(aChercher))
  const premier = (saisie: string) => chercher(index, saisie).trouves[0]
  assert.equal(premier('Contact Group')?.rang, 1)
  assert.equal(premier('GGA')?.rang, 1)
  assert.equal(premier('contact gr')?.rang, 2)
  assert.equal(premier('stocktaking')?.rang, 2)
  assert.equal(premier('ontact')?.rang, 3)
  assert.equal(premier('contcat group')?.rang, 4)
})

test('« vous cherchiez peut-être » et « aussi dans les traductions »', () => {
  const index = preparerIndex(LEXIQUE.map(aChercher))
  const faute = chercher(index, 'contcat group')
  assert.equal(faute.vousCherchiezPeutEtre, true)
  assert.equal(faute.trouves[0]?.valeur.term, 'contact group')

  const juste = chercher(index, 'contact group')
  assert.equal(juste.vousCherchiezPeutEtre, false)
  assert.equal(juste.aussiDansLesTraductions, false)

  const francais = chercher(index, 'groupe de contact')
  assert.equal(francais.aussiDansLesTraductions, true)
  assert.equal(francais.trouves[0]?.nature, 'traduction')

  assert.deepEqual(chercher(index, '  ').trouves, [])
  assert.deepEqual(chercher(index, 'zzzzzz').trouves, [])
})

test('la FAQ : la question avant la réponse', () => {
  const index = preparerIndex([
    { valeur: 'dans la réponse', noms: ['Qu’est-ce qu’un document L ?'], corps: ['Le groupe de contact en produit un.'] },
    { valeur: 'dans la question', noms: ['Quelle différence entre un groupe de contact et des consultations ?'], corps: ['…'] },
  ])
  const trouves = chercher(index, 'groupe de contact').trouves
  assert.deepEqual(
    trouves.map((t) => [t.valeur, t.nature]),
    [
      ['dans la question', 'nom'],
      ['dans la réponse', 'corps'],
    ],
  )
})

test('la distance bornée : transposition, et abandon au-delà du seuil', () => {
  assert.equal(distanceBornee('contcat', 'contact', 1), 1)
  assert.equal(distanceBornee('hudle', 'huddle', 1), 1)
  assert.equal(distanceBornee('adaptaton', 'adaptation', 2), 1)
  assert.equal(distanceBornee('abc', 'xyz', 1), 2)
  assert.equal(distanceBornee('court', 'beaucoup-plus-long', 2), 3)
})

test('500 entrées : chaque requête sous 16 ms', () => {
  const MOTS = ['alpha', 'bravo', 'charlie', 'delta', 'echo', 'foxtrot', 'golf', 'hotel', 'india', 'juliett', 'kilo', 'lima']
  const gonfle: GlossaryEntry[] = Array.from({ length: 500 }, (_, i) => {
    const base = LEXIQUE[i % LEXIQUE.length] as GlossaryEntry
    const suffixe = `${MOTS[i % MOTS.length]}${Math.floor(i / MOTS.length)}`
    return {
      ...base,
      id: `${base.id}-${i}`,
      term: `${base.term} ${suffixe}`,
      variants: base.variants.map((v) => `${v} ${suffixe}`),
      translation: `${base.translation} ${suffixe}`,
    }
  })
  const index = preparerIndex(gonfle.map(aChercher))
  const requetes = SAISIES.flatMap(([terme, faute, francais]) => [terme, faute, francais ?? terme])
  for (const r of requetes) chercher(index, r)

  const durees = requetes.map((r) => {
    const debut = performance.now()
    chercher(index, r)
    return performance.now() - debut
  })
  const pire = Math.max(...durees)
  const moyenne = durees.reduce((a, b) => a + b, 0) / durees.length
  console.log(`500 entrées, ${requetes.length} requêtes : moyenne ${moyenne.toFixed(2)} ms, pire ${pire.toFixed(2)} ms`)
  assert.ok(pire < 16, `pire requête : ${pire.toFixed(2)} ms`)
})
