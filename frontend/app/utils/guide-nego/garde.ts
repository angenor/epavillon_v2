/**
 * La garde : ce que l'application a lu du réseau, et ce qu'elle a à lui écrire.
 *
 * Deux magasins dans une même base. `lectures` porte ce qui a été lu, avec l'heure
 * de sa lecture (ADR-003). `ecritures` porte les intentions prises sans réseau, qui
 * repartent à son retour — voir `file.ts`. Si le téléphone refuse ou vide le stockage,
 * l'application fonctionne sans garde : aucune de ces fonctions ne lève.
 */
import { magasinEnMemoire as copiesEnMemoire, type Copie, type DemandeDeTelechargement, type Magasin } from './copies.ts'
import { magasinEnMemoire, type Intention, type MagasinEcritures } from './file.ts'

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
const COPIES = 'copies'
const A_TELECHARGER = 'a-telecharger'
// Version 2 : le magasin des écritures, à l'étape 0c. Version 3 : les fiches des
// copies et les téléchargements demandés sans réseau, à l'étape 1. Aucune montée ne
// touche à ce que portent les magasins d'avant.
const VERSION = 3

/**
 * `refusee` : le téléphone n'offre pas IndexedDB (navigation privée) — rien n'a pu
 * être gardé avant, le repli en mémoire est sans risque. `bloquee` : un autre onglet
 * tient une version plus ancienne ouverte — ce qui est gardé existe mais ne se lit
 * pas, et **rien ne doit se conclure d'une lecture vide**.
 */
type EtatDeLaBase = IDBDatabase | 'refusee' | 'bloquee'

function ouvrirLaBase(): Promise<EtatDeLaBase> {
  return new Promise((resolve) => {
    let tranche = false
    const trancher = (etat: EtatDeLaBase) => {
      if (tranche) {
        // Débloquée après coup : cette connexion n'a plus d'usage.
        if (typeof etat !== 'string') etat.close()
        return
      }
      tranche = true
      resolve(etat)
    }
    try {
      const demande = indexedDB.open(BASE, VERSION)
      demande.onupgradeneeded = () => {
        const base = demande.result
        if (!base.objectStoreNames.contains(LECTURES)) base.createObjectStore(LECTURES, { keyPath: 'cle' })
        if (!base.objectStoreNames.contains(ECRITURES)) base.createObjectStore(ECRITURES, { keyPath: 'cle' })
        if (!base.objectStoreNames.contains(COPIES)) base.createObjectStore(COPIES, { keyPath: 'id' })
        if (!base.objectStoreNames.contains(A_TELECHARGER)) base.createObjectStore(A_TELECHARGER, { keyPath: 'id' })
      }
      demande.onsuccess = () => {
        // Une version plus récente, dans un autre onglet, doit pouvoir monter.
        demande.result.onversionchange = () => demande.result.close()
        trancher(demande.result)
      }
      demande.onerror = () => trancher('refusee')
      demande.onblocked = () => trancher('bloquee')
    } catch {
      trancher('refusee')
    }
  })
}

const ouvrir = (): Promise<IDBDatabase | null> =>
  ouvrirLaBase().then((etat) => (typeof etat === 'string' ? null : etat))

/** Une transaction, jugée à sa validation et non à sa requête ; la connexion se ferme après. */
function transaction<R>(
  base: IDBDatabase,
  magasin: string,
  mode: IDBTransactionMode,
  operation: (magasin: IDBObjectStore) => IDBRequest<R>,
): Promise<{ ok: true; valeur: R | null } | { ok: false }> {
  return new Promise((resolve) => {
    try {
      const tx = base.transaction(magasin, mode)
      const demande = operation(tx.objectStore(magasin))
      tx.oncomplete = () => {
        base.close()
        resolve({ ok: true, valeur: demande.result ?? null })
      }
      tx.onabort = tx.onerror = () => {
        base.close()
        resolve({ ok: false })
      }
    } catch {
      base.close()
      resolve({ ok: false })
    }
  })
}

function executer<R>(
  magasin: string,
  mode: IDBTransactionMode,
  operation: (magasin: IDBObjectStore) => IDBRequest<R>,
): Promise<R | null> {
  return ouvrir().then((base) =>
    base ? transaction(base, magasin, mode, operation).then((r) => (r.ok ? r.valeur : null)) : null,
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

/** Vide les données lues. La coquille et la file ne sont pas touchées. */
export async function viderLesGardes(): Promise<void> {
  await executer(LECTURES, 'readwrite', (magasin) => magasin.clear())
}

const ecrituresIndexedDb: MagasinEcritures = {
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

// Stockage refusé : les intentions vivent le temps de la visite, et partent quand même.
// Sans ce secours, un choix fait en ligne se perdrait en silence.
const ecrituresEnMemoire = magasinEnMemoire()

async function ecrituresDisponibles(): Promise<MagasinEcritures> {
  const base = await ouvrir()
  base?.close()
  return base ? ecrituresIndexedDb : ecrituresEnMemoire
}

/** Le magasin des écritures, tel que `file.ts` l'attend. */
export const magasinDesEcritures: MagasinEcritures = {
  lire: () => ecrituresDisponibles().then((m) => m.lire()),
  lireUne: (cle) => ecrituresDisponibles().then((m) => m.lireUne(cle)),
  poser: (intention) => ecrituresDisponibles().then((m) => m.poser(intention)),
  retirer: (cle) => ecrituresDisponibles().then((m) => m.retirer(cle)),
  vider: () => ecrituresDisponibles().then((m) => m.vider()),
}

function magasinIndexe<T extends { id: string }>(nom: string): Magasin<T> {
  // IndexedDB refusée : le magasin vit le temps de la visite, comme les écritures.
  const enMemoire = copiesEnMemoire<T>()

  async function surLaBase<R>(
    mode: IDBTransactionMode,
    operation: (m: IDBObjectStore) => IDBRequest<R>,
    repli: (m: Magasin<T>) => Promise<R | null>,
  ): Promise<{ ok: true; valeur: R | null } | { ok: false }> {
    const etat = await ouvrirLaBase()
    if (etat === 'refusee') return { ok: true, valeur: await repli(enMemoire) }
    if (etat === 'bloquee') return { ok: false }
    return transaction(etat, nom, mode, operation)
  }

  // Une écriture qui n'a pas eu lieu lève : « complet ou rien » en dépend.
  const ecrire = async <R>(operation: (m: IDBObjectStore) => IDBRequest<R>, repli: (m: Magasin<T>) => Promise<void>) => {
    const r = await surLaBase<R>('readwrite', operation, async (m) => {
      await repli(m)
      return null
    })
    if (!r.ok) throw new Error(`Écriture refusée dans « ${nom} »`)
  }

  return {
    lire: async () => {
      const r = await surLaBase<T[]>('readonly', (m) => m.getAll(), (m) => m.lire())
      return r.ok ? (r.valeur ?? []) : null
    },
    lireUne: async (id) => {
      const r = await surLaBase<T>('readonly', (m) => m.get(id), (m) => m.lireUne(id))
      return r.ok ? r.valeur : null
    },
    poser: (valeur) => ecrire((m) => m.put(simple(valeur)), (m) => m.poser(valeur)),
    retirer: (id) => ecrire((m) => m.delete(id), (m) => m.retirer(id)),
    vider: () => ecrire((m) => m.clear(), (m) => m.vider()),
  }
}

/** Les fiches des documents gardés sur le téléphone. */
export const magasinDesCopies = magasinIndexe<Copie>(COPIES)

/** Les téléchargements demandés sans réseau. */
export const magasinATelecharger = magasinIndexe<DemandeDeTelechargement>(A_TELECHARGER)
