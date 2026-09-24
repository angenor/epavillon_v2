import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import {
  chargerPdfjs,
  fonctionsManquantes,
  ouvrirLeDocument,
  TAILLE_DES_MORCEAUX,
  type Environnement,
  type FonctionRequise,
  type Importeur,
  type Pdfjs,
} from '../../app/utils/guide-nego/pdf/charger.ts'
import { ErreurDeReseau } from '../../app/utils/guide-nego/pdf/transport.ts'

/** Un travailleur qui connaît les modules lit `type` dans ses options. */
class TravailleurModerne {
  constructor(_adresse: string, options: { type?: string }) {
    void options.type
    throw new SyntaxError('adresse invalide')
  }
}
class TravailleurAncien {
  constructor() {
    throw new SyntaxError('adresse invalide')
  }
}

const COMPLET: Environnement = {
  Function,
  Worker: TravailleurModerne,
  structuredClone,
  Path2D: class {},
}

const sansSyntaxe = function () {
  throw new SyntaxError('Unexpected token {')
}

const CAS: [FonctionRequise, Environnement][] = [
  ['blocs-statiques', { ...COMPLET, Function: sansSyntaxe }],
  ['module-dans-worker', { ...COMPLET, Worker: TravailleurAncien }],
  ['module-dans-worker', { ...COMPLET, Worker: undefined }],
  ['structuredClone', { ...COMPLET, structuredClone: undefined }],
  ['Path2D', { ...COMPLET, Path2D: undefined }],
]

const importeurInterdit: Importeur = () => assert.fail('pdf.js ne doit pas se charger')

test('un navigateur complet ne manque de rien', () => {
  assert.deepEqual(fonctionsManquantes(COMPLET), [])
})

for (const [manquante, env] of CAS) {
  test(`${manquante} manquant : indisponible, sans rien charger`, async () => {
    assert.deepEqual(fonctionsManquantes(env), [manquante])
    const chargement = await chargerPdfjs(env, importeurInterdit)
    assert.deepEqual(chargement, { etat: 'indisponible', cause: 'fonctions-manquantes', manquantes: [manquante] })
  })
}

test('une politique qui interdit l’évaluation ne passe pas pour une syntaxe manquante', () => {
  const interdit = function () {
    throw new EvalError('unsafe-eval')
  }
  assert.deepEqual(fonctionsManquantes({ ...COMPLET, Function: interdit }), [])
})

test('la sonde des blocs statiques compile vraiment la syntaxe', () => {
  const lus: string[] = []
  const espion = function (code: string) {
    lus.push(code)
  }
  fonctionsManquantes({ ...COMPLET, Function: espion })
  assert.match(lus[0]!, /static\s*\{\s*\}/)
})

test('rien n’est lu dans navigator.userAgent', async () => {
  const avant = Object.getOwnPropertyDescriptor(globalThis, 'navigator')
  let lu = false
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      get userAgent() {
        lu = true
        return 'Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)'
      },
    },
  })
  try {
    fonctionsManquantes(COMPLET)
    await chargerPdfjs({ ...COMPLET, Path2D: undefined }, importeurInterdit)
    await chargerPdfjs(COMPLET, async () => Promise.reject(new Error('hors ligne')))
    assert.equal(lu, false)
  } finally {
    if (avant) Object.defineProperty(globalThis, 'navigator', avant)
  }
  const source = readFileSync(fileURLToPath(new URL('../../app/utils/guide-nego/pdf/charger.ts', import.meta.url)), 'utf8')
  assert.doesNotMatch(source, /userAgent/)
})

test('un import manqué rend indisponible', async () => {
  const erreur = new TypeError('Failed to fetch dynamically imported module')
  const chargement = await chargerPdfjs(COMPLET, async () => Promise.reject(erreur))
  assert.deepEqual(chargement, { etat: 'indisponible', cause: 'import', erreur })
})

test('les remplacements sont posés avant l’import de pdf.js', async () => {
  const promesse = Object.getOwnPropertyDescriptor(Promise, 'withResolvers')!
  delete (Promise as { withResolvers?: unknown }).withResolvers
  let vu = 'pas appelé'
  try {
    const chargement = await chargerPdfjs(COMPLET, async () => {
      vu = typeof Promise.withResolvers
      return { pdfjs: {} as Pdfjs, visionneur: {} as never }
    })
    assert.equal(chargement.etat, 'pret')
    assert.equal(vu, 'function')
  } finally {
    Object.defineProperty(Promise, 'withResolvers', promesse)
  }
})

/** Un pdf.js simulé : ce que `getDocument` a reçu, et une tâche qu'on tient. */
function fauxPdfjs() {
  const recus: Record<string, unknown>[] = []
  let detruite = false
  let ouvrir: (pdf: unknown) => void = () => undefined
  class PDFDataRangeTransport {
    length: number
    constructor(taille: number) {
      this.length = taille
    }
    onDataRange() {}
    requestDataRange() {}
    abort() {}
  }
  const pdfjs = {
    version: '6.3.289',
    PDFDataRangeTransport,
    getDocument(parametres: Record<string, unknown>) {
      recus.push(parametres)
      return { promise: new Promise((ok) => (ouvrir = ok)), destroy: async () => void (detruite = true) }
    },
  } as unknown as Pdfjs
  return { pdfjs, recus, ouvrir: (pdf: unknown) => ouvrir(pdf), detruite: () => detruite }
}

const RESSOURCES = '/v2/guide-nego/pdfjs/6.3.289/'

test('une copie gardée s’ouvre par ses octets, sans réseau', async () => {
  const { pdfjs, recus } = fauxPdfjs()
  const octets = new Uint8Array([37, 80, 68, 70])
  const ouvert = await ouvrirLeDocument(pdfjs, { octets }, { ressources: RESSOURCES, recuperer: () => assert.fail('réseau') })
  assert.equal(ouvert.plages, null)
  assert.equal(recus[0]!.data, octets)
  assert.equal(recus[0]!.wasmUrl, `${RESSOURCES}wasm/`)
  assert.equal(recus[0]!.iccUrl, `${RESSOURCES}iccs/`)
  assert.equal(recus[0]!.standardFontDataUrl, `${RESSOURCES}standard_fonts/`)
})

test('en ligne : par le transport, morceaux de 256 Ko, sans lecture d’avance ni lecture par URL', async () => {
  const { pdfjs, recus, ouvrir } = fauxPdfjs()
  const recuperer = async () => new Response(null, { status: 200, headers: { 'Content-Length': '2884088' } })
  const ouvert = await ouvrirLeDocument(pdfjs, { adresse: 'https://exemple.org/f' }, { ressources: RESSOURCES, recuperer })
  const parametres = recus[0]!
  assert.equal((parametres.range as { length: number }).length, 2884088)
  assert.equal(parametres.rangeChunkSize, TAILLE_DES_MORCEAUX)
  assert.equal(TAILLE_DES_MORCEAUX, 262144)
  assert.equal(parametres.disableAutoFetch, true)
  assert.equal(parametres.url, undefined, 'jamais la lecture par URL de pdf.js, qui prend le fichier entier')
  assert.notEqual(ouvert.plages, null)
  ouvrir('document')
  assert.equal(await ouvert.document, 'document')
})

test('le réseau qui lâche avant l’ouverture rend une erreur de réseau et arrête pdf.js', async () => {
  const { pdfjs, recus, detruite } = fauxPdfjs()
  let appel = 0
  const recuperer = async () =>
    appel++ === 0
      ? new Response(null, { status: 200, headers: { 'Content-Length': '1000' } })
      : Promise.reject(new TypeError('Failed to fetch'))
  const ouvert = await ouvrirLeDocument(pdfjs, { adresse: 'https://exemple.org/f' }, { ressources: RESSOURCES, recuperer })
  ;(recus[0]!.range as { requestDataRange(a: number, b: number): void }).requestDataRange(0, 1000)
  await assert.rejects(ouvert.document, ErreurDeReseau)
  assert.equal(detruite(), true)
})
