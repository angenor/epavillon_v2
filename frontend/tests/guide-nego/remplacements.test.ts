import { test } from 'node:test'
import assert from 'node:assert/strict'
import { registerHooks } from 'node:module'
import { createContext, runInContext } from 'node:vm'
import { installerLesRemplacements } from '../../app/utils/guide-nego/pdf/remplacements.ts'

/**
 * Un iPhone sous iOS 16.4 à 17.3, simulé dans un royaume neuf : sans `Promise.withResolvers`
 * ni itérateur de flux. La page a une `window` ; le travailleur n'a que `self` et `postMessage`.
 */
const FLUX = `
  class ReadableStream {
    constructor(morceaux) { this.morceaux = morceaux; this.annule = false; this.libere = false }
    getReader() {
      let i = 0
      return {
        read: async () => (i < this.morceaux.length ? { done: false, value: this.morceaux[i++] } : { done: true }),
        cancel: async () => { this.annule = true },
        releaseLock: () => { this.libere = true },
      }
    }
  }
  globalThis.ReadableStream = ReadableStream
  delete Promise.withResolvers
`

type Contexte = 'page' | 'travailleur'

/** L'objet global du royaume, où vivent ses `Promise` et ses flux — pas le bac qu'on lui a donné. */
const globalDe = (royaume: object) => runInContext('globalThis', royaume)

function ancienRoyaume(contexte: Contexte) {
  const bac = contexte === 'page' ? { window: {} } : { postMessage() {}, onmessage: null }
  const royaume = createContext(bac)
  runInContext(`${FLUX}; globalThis.self = globalThis`, royaume)
  return royaume
}

const executer = async (royaume: object, code: string): Promise<unknown> => JSON.parse(await runInContext(code, royaume))

for (const contexte of ['page', 'travailleur'] as const) {
  test(`${contexte} : les deux absences sont simulées, puis comblées`, async () => {
    const royaume = ancienRoyaume(contexte)
    assert.equal(runInContext('typeof Promise.withResolvers', royaume), 'undefined')
    assert.equal(runInContext('typeof ReadableStream.prototype[Symbol.asyncIterator]', royaume), 'undefined')

    const poses = installerLesRemplacements(globalDe(royaume))
    assert.deepEqual(poses, ['Promise.withResolvers', 'ReadableStream[Symbol.asyncIterator]'])

    const resolu = await executer(
      royaume,
      `(async () => { const { promise, resolve } = Promise.withResolvers(); resolve(42); return JSON.stringify(await promise) })()`,
    )
    assert.equal(resolu, 42)
    const rejete = await executer(
      royaume,
      `(async () => { const { promise, reject } = Promise.withResolvers(); reject(new Error('non')); try { await promise } catch (e) { return JSON.stringify(e.message) } })()`,
    )
    assert.equal(rejete, 'non')

    const lu = await executer(
      royaume,
      `(async () => { const r = []; const f = new ReadableStream([1, 2, 3]); for await (const m of f) r.push(m); return JSON.stringify({ r, annule: f.annule, libere: f.libere }) })()`,
    )
    assert.deepEqual(lu, { r: [1, 2, 3], annule: false, libere: true })
  })

  test(`${contexte} : une sortie anticipée annule le flux, comme l'itérateur natif`, async () => {
    const royaume = ancienRoyaume(contexte)
    installerLesRemplacements(globalDe(royaume))
    const lu = await executer(
      royaume,
      `(async () => { const f = new ReadableStream([1, 2, 3]); for await (const m of f) break; return JSON.stringify({ annule: f.annule, libere: f.libere }) })()`,
    )
    assert.deepEqual(lu, { annule: true, libere: true })
  })
}

test('une fonction présente n’est jamais remplacée', () => {
  const royaume = createContext({})
  runInContext(
    `globalThis.maison = function () { return 'natif' }
     Promise.withResolvers = maison
     globalThis.ReadableStream = class {}
     ReadableStream.prototype[Symbol.asyncIterator] = maison`,
    royaume,
  )
  assert.deepEqual(installerLesRemplacements(globalDe(royaume)), [])
  assert.equal(runInContext('Promise.withResolvers === maison', royaume), true)
  assert.equal(runInContext('ReadableStream.prototype[Symbol.asyncIterator] === maison', royaume), true)
})

test('sans ReadableStream, seul Promise.withResolvers est posé', () => {
  const royaume = createContext({})
  runInContext('delete Promise.withResolvers', royaume)
  assert.deepEqual(installerLesRemplacements(globalDe(royaume)), [
    'Promise.withResolvers',
  ])
})

test('l’amorce du travailleur installe les remplacements AVANT d’évaluer pdf.js', async () => {
  // pdf.js est remplacé par un module qui note ce qu'il trouve en s'évaluant.
  const faux = `globalThis.vuParPdfjs = { withResolvers: typeof Promise.withResolvers, iterateur: typeof ReadableStream.prototype[Symbol.asyncIterator] }`
  const crochets = registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'pdfjs-dist/legacy/build/pdf.worker.min.mjs') {
        return { url: `data:text/javascript,${encodeURIComponent(faux)}`, shortCircuit: true }
      }
      return nextResolve(specifier, context)
    },
  })
  const promesse = Object.getOwnPropertyDescriptor(Promise, 'withResolvers')!
  const iterateur = Object.getOwnPropertyDescriptor(ReadableStream.prototype, Symbol.asyncIterator)!
  const global = globalThis as { self?: unknown; vuParPdfjs?: unknown }
  try {
    delete (Promise as { withResolvers?: unknown }).withResolvers
    delete (ReadableStream.prototype as { [Symbol.asyncIterator]?: unknown })[Symbol.asyncIterator]
    global.self = globalThis
    await import('../../app/utils/guide-nego/pdf/travailleur.ts')
    assert.deepEqual(global.vuParPdfjs, { withResolvers: 'function', iterateur: 'function' })
  } finally {
    crochets.deregister()
    Object.defineProperty(Promise, 'withResolvers', promesse)
    Object.defineProperty(ReadableStream.prototype, Symbol.asyncIterator, iterateur)
    delete global.self
  }
})
