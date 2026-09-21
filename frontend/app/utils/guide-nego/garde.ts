/**
 * La garde des lectures : ce que l'application a lu du réseau, et quand.
 *
 * Toute donnée affichée hors connexion dit l'heure de sa lecture (ADR-003). Si le
 * téléphone refuse ou vide le stockage, l'application fonctionne sans garde : aucune
 * de ces fonctions ne lève.
 */
export interface LectureGardee<T> {
  cle: string
  valeur: T
  /** Instant ISO de la lecture réussie. */
  lu_a: string
  empreinte: string | null
}

const BASE = 'guide-nego'
const MAGASIN = 'lectures'

function ouvrir(): Promise<IDBDatabase | null> {
  return new Promise((resolve) => {
    try {
      const demande = indexedDB.open(BASE, 1)
      demande.onupgradeneeded = () => demande.result.createObjectStore(MAGASIN, { keyPath: 'cle' })
      demande.onsuccess = () => resolve(demande.result)
      demande.onerror = () => resolve(null)
      demande.onblocked = () => resolve(null)
    } catch {
      resolve(null)
    }
  })
}

function executer<R>(
  mode: IDBTransactionMode,
  operation: (magasin: IDBObjectStore) => IDBRequest<R>,
): Promise<R | null> {
  return ouvrir().then(
    (base) =>
      new Promise((resolve) => {
        if (!base) return resolve(null)
        try {
          const demande = operation(base.transaction(MAGASIN, mode).objectStore(MAGASIN))
          demande.onsuccess = () => resolve(demande.result ?? null)
          demande.onerror = () => resolve(null)
        } catch {
          resolve(null)
        }
      }),
  )
}

export function lireGarde<T>(cle: string): Promise<LectureGardee<T> | null> {
  return executer<LectureGardee<T>>('readonly', (magasin) => magasin.get(cle))
}

export async function ecrireGarde<T>(lecture: LectureGardee<T>): Promise<void> {
  // IndexedDB clone par l'algorithme structuré : un proxy réactif le ferait échouer.
  const simple = JSON.parse(JSON.stringify(lecture)) as LectureGardee<T>
  await executer('readwrite', (magasin) => magasin.put(simple))
}

export function lireToutesLesGardes(): Promise<LectureGardee<unknown>[]> {
  return executer<LectureGardee<unknown>[]>('readonly', (magasin) => magasin.getAll()).then(
    (lectures) => lectures ?? [],
  )
}
