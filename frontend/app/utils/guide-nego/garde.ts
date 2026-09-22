/**
 * La garde : ce que l'application a lu du réseau, et ce qu'elle a à lui écrire.
 *
 * Deux magasins dans une même base. `lectures` porte ce qui a été lu, avec l'heure
 * de sa lecture (ADR-003). `ecritures` porte les intentions prises sans réseau, qui
 * repartent à son retour — voir `file.ts`. Si le téléphone refuse ou vide le stockage,
 * l'application fonctionne sans garde : aucune de ces fonctions ne lève.
 */
import type { Intention, MagasinEcritures } from './file'

export interface LectureGardee<T> {
  cle: string
  valeur: T
  /** Instant ISO de la lecture réussie. */
  lu_a: string
  empreinte: string | null
}

const BASE = 'guide-nego'
const LECTURES = 'lectures'
const ECRITURES = 'ecritures'
// Version 2 : le magasin des écritures, ajouté à l'étape 0c. La montée ne touche pas
// au magasin des lectures ni à ce qu'il porte.
const VERSION = 2

function ouvrir(): Promise<IDBDatabase | null> {
  return new Promise((resolve) => {
    try {
      const demande = indexedDB.open(BASE, VERSION)
      demande.onupgradeneeded = () => {
        const base = demande.result
        if (!base.objectStoreNames.contains(LECTURES)) base.createObjectStore(LECTURES, { keyPath: 'cle' })
        if (!base.objectStoreNames.contains(ECRITURES)) base.createObjectStore(ECRITURES, { keyPath: 'cle' })
      }
      demande.onsuccess = () => resolve(demande.result)
      demande.onerror = () => resolve(null)
      demande.onblocked = () => resolve(null)
    } catch {
      resolve(null)
    }
  })
}

function executer<R>(
  magasin: string,
  mode: IDBTransactionMode,
  operation: (magasin: IDBObjectStore) => IDBRequest<R>,
): Promise<R | null> {
  return ouvrir().then(
    (base) =>
      new Promise((resolve) => {
        if (!base) return resolve(null)
        try {
          const demande = operation(base.transaction(magasin, mode).objectStore(magasin))
          demande.onsuccess = () => resolve(demande.result ?? null)
          demande.onerror = () => resolve(null)
        } catch {
          resolve(null)
        }
      }),
  )
}

/** IndexedDB clone par l'algorithme structuré : un proxy réactif le ferait échouer. */
const simple = <T>(valeur: T): T => JSON.parse(JSON.stringify(valeur)) as T

export function lireGarde<T>(cle: string): Promise<LectureGardee<T> | null> {
  return executer<LectureGardee<T>>(LECTURES, 'readonly', (magasin) => magasin.get(cle))
}

export async function ecrireGarde<T>(lecture: LectureGardee<T>): Promise<void> {
  await executer(LECTURES, 'readwrite', (magasin) => magasin.put(simple(lecture)))
}

export function lireToutesLesGardes(): Promise<LectureGardee<unknown>[]> {
  return executer<LectureGardee<unknown>[]>(LECTURES, 'readonly', (magasin) => magasin.getAll()).then(
    (lectures) => lectures ?? [],
  )
}

/**
 * Vide les données lues, sauf les clés à `garder`. La coquille et la file ne sont
 * pas touchées : un choix pas encore envoyé ne se perd pas en libérant de la place.
 */
export async function viderLesGardes(garder: readonly string[] = []): Promise<void> {
  const cles = await executer<IDBValidKey[]>(LECTURES, 'readonly', (magasin) => magasin.getAllKeys())
  for (const cle of cles ?? []) {
    if (typeof cle === 'string' && garder.includes(cle)) continue
    await executer(LECTURES, 'readwrite', (magasin) => magasin.delete(cle))
  }
}

/** Le magasin des écritures, tel que `file.ts` l'attend. */
export const magasinEcrituresIndexedDb: MagasinEcritures = {
  lire: () =>
    executer<Intention[]>(ECRITURES, 'readonly', (magasin) => magasin.getAll()).then(
      (intentions) => intentions ?? [],
    ),
  lireUne: (cle) => executer<Intention>(ECRITURES, 'readonly', (magasin) => magasin.get(cle)),
  poser: async (intention) => {
    await executer(ECRITURES, 'readwrite', (magasin) => magasin.put(simple(intention)))
  },
  retirer: async (cle) => {
    await executer(ECRITURES, 'readwrite', (magasin) => magasin.delete(cle))
  },
  vider: async () => {
    await executer(ECRITURES, 'readwrite', (magasin) => magasin.clear())
  },
}
