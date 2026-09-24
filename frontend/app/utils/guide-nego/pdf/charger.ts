/**
 * La seule porte de pdf.js (ADR-022). Build `legacy`, chargé à la demande ; son
 * travailleur part de l'amorce `travailleur.ts`, qui pose les remplacements avant lui.
 *
 * Un téléphone trop ancien se reconnaît à ses fonctions, jamais à l'identifiant de
 * son navigateur (FR-012 bis) : il reçoit `indisponible` avant tout chargement.
 */
import type { PDFDocumentLoadingTask, PDFDocumentProxy } from 'pdfjs-dist/legacy/build/pdf.min.mjs'
import { assetUrl } from '../../asset.ts'
import { installerLesRemplacements } from './remplacements.ts'
import { brancherLeTransport, lireLaTaille, type Recuperer, type SuiviDesPlages } from './transport.ts'

export type Pdfjs = typeof import('pdfjs-dist/legacy/build/pdf.min.mjs')
export type Visionneur = typeof import('pdfjs-dist/legacy/web/pdf_viewer.mjs')

export type FonctionRequise = 'blocs-statiques' | 'module-dans-worker' | 'structuredClone' | 'Path2D'

/** Ce que la détection lit de l'objet global. */
export interface Environnement {
  Function?: unknown
  Worker?: unknown
  structuredClone?: unknown
  Path2D?: unknown
}

export type Chargement =
  | { etat: 'pret'; pdfjs: Pdfjs; visionneur: Visionneur }
  | { etat: 'indisponible'; cause: 'fonctions-manquantes'; manquantes: FonctionRequise[] }
  | { etat: 'indisponible'; cause: 'import'; erreur: unknown }

export type Importeur = () => Promise<{ pdfjs: Pdfjs; visionneur: Visionneur }>

export const TAILLE_DES_MORCEAUX = 256 * 1024

type ConstructeurDeWorker = new (adresse: string, options: WorkerOptions) => { terminate(): void }

/** Le build legacy contient des blocs `static {}` : en dessous d'iOS 16.4, le fichier entier est refusé. */
function blocsStatiques(env: Environnement): boolean {
  if (typeof env.Function !== 'function') return false
  try {
    new (env.Function as FunctionConstructor)('class A { static {} }')
    return true
  } catch (erreur) {
    // Une politique de sécurité qui interdit l'évaluation ne dit rien de la syntaxe.
    return (erreur as { name?: string } | null)?.name !== 'SyntaxError'
  }
}

/** Un navigateur qui connaît les travailleurs modules lit `type` ; l'adresse invalide empêche tout lancement. */
function moduleDansWorker(env: Environnement): boolean {
  if (typeof env.Worker !== 'function') return false
  let lu = false
  const options = {
    get type(): WorkerType {
      lu = true
      return 'module'
    },
  }
  try {
    new (env.Worker as ConstructeurDeWorker)('http://[', options).terminate()
  } catch {
    // Attendu : seule compte la lecture de `type`.
  }
  return lu
}

export function fonctionsManquantes(env: Environnement): FonctionRequise[] {
  const manquantes: FonctionRequise[] = []
  if (!blocsStatiques(env)) manquantes.push('blocs-statiques')
  if (!moduleDansWorker(env)) manquantes.push('module-dans-worker')
  if (typeof env.structuredClone !== 'function') manquantes.push('structuredClone')
  if (typeof env.Path2D !== 'function') manquantes.push('Path2D')
  return manquantes
}

const importerPdfjs: Importeur = async () => {
  const pdfjs = await import('pdfjs-dist/legacy/build/pdf.min.mjs')
  pdfjs.GlobalWorkerOptions.workerPort ??= new Worker(new URL('./travailleur.ts', import.meta.url), { type: 'module' })
  // Le visionneur lit `globalThis.pdfjsLib` à son évaluation.
  ;(globalThis as { pdfjsLib?: Pdfjs }).pdfjsLib = pdfjs
  const visionneur = await import('pdfjs-dist/legacy/web/pdf_viewer.mjs')
  return { pdfjs, visionneur }
}

let dejaCharge: Promise<Chargement> | null = null

async function charger(importer: Importeur): Promise<Chargement> {
  installerLesRemplacements(globalThis)
  try {
    const { pdfjs, visionneur } = await importer()
    return { etat: 'pret', pdfjs, visionneur }
  } catch (erreur) {
    return { etat: 'indisponible', cause: 'import', erreur }
  }
}

export function chargerPdfjs(
  env: Environnement = globalThis,
  importer: Importeur = importerPdfjs,
): Promise<Chargement> {
  const manquantes = fonctionsManquantes(env)
  if (manquantes.length > 0) {
    return Promise.resolve({ etat: 'indisponible', cause: 'fonctions-manquantes', manquantes })
  }
  if (importer !== importerPdfjs) return charger(importer)
  // Un import manqué se retente à l'ouverture suivante ; un import réussi ne se refait pas.
  dejaCharge ??= charger(importer).then((chargement) => {
    if (chargement.etat !== 'pret') dejaCharge = null
    return chargement
  })
  return dejaCharge
}

/** `octets` : la copie gardée. `adresse` : la route du fichier, lue par plages. */
export type SourceDuDocument = { octets: Uint8Array } | { adresse: string }

export interface OptionsDOuverture {
  /** Le dossier des ressources de pdf.js ; par défaut celui que copie `modules/guide-nego-pdfjs.ts`. */
  ressources?: string
  recuperer?: Recuperer
  surChangement?: (suivi: SuiviDesPlages) => void
}

export interface DocumentOuvert {
  tache: PDFDocumentLoadingTask
  /** Rejeté par une `ErreurDeReseau` si le réseau lâche avant l'ouverture, par l'erreur de pdf.js sinon. */
  document: Promise<PDFDocumentProxy>
  /** `null` pour une copie gardée : rien ne passe par le réseau. */
  plages: SuiviDesPlages | null
}

export async function ouvrirLeDocument(
  pdfjs: Pdfjs,
  source: SourceDuDocument,
  options: OptionsDOuverture = {},
): Promise<DocumentOuvert> {
  const ressources = options.ressources ?? assetUrl(`/guide-nego/pdfjs/${pdfjs.version}/`)
  const commun = {
    wasmUrl: `${ressources}wasm/`,
    iccUrl: `${ressources}iccs/`,
    standardFontDataUrl: `${ressources}standard_fonts/`,
    isEvalSupported: false,
  }

  if ('octets' in source) {
    // pdf.js transfère le tampon à son travailleur : les octets donnés ne se relisent pas.
    const tache = pdfjs.getDocument({ ...commun, data: source.octets })
    return { tache, document: tache.promise, plages: null }
  }

  const recuperer = options.recuperer ?? fetch
  const taille = await lireLaTaille(source.adresse, recuperer)
  const range = new pdfjs.PDFDataRangeTransport(taille, null)
  const plages = brancherLeTransport(range, source.adresse, { recuperer, surChangement: options.surChangement })
  const tache = pdfjs.getDocument({
    ...commun,
    range,
    rangeChunkSize: TAILLE_DES_MORCEAUX,
    disableAutoFetch: true,
  })
  let ouvert = false
  const document = Promise.race([
    tache.promise.then((pdf) => {
      ouvert = true
      return pdf
    }),
    plages.echec.then((erreur): never => {
      if (!ouvert) void tache.destroy()
      throw erreur
    }),
  ])
  return { tache, document, plages }
}
