/**
 * Les documents gardés sur le téléphone — ce qui les garde, les vérifie et les efface.
 *
 * Deux caches, et non un : `gn-documents-reserves` s'efface d'un geste à la
 * déconnexion, sans trier (R11). Ni l'un ni l'autre ne porte le préfixe
 * `gn-coquille-` : le ménage du service worker ne les touche pas.
 *
 * La fiche de chaque copie vit dans le magasin `copies` ; c'est elle que lit
 * « Sur le téléphone ». Mais **le navigateur peut vider un cache sans prévenir**
 * quand l'appareil manque de place : la fiche seule ne prouve rien. D'où
 * `verifierLesCopies`, à chaque ouverture, et `copieIntacte`, avant de lire.
 *
 * Tout est injecté — caches, magasins — pour que chaque règle se prouve sur des faux.
 */
import type { ReadingMode } from '~/types/negotiation-documents'

export const CACHE_PUBLICS = 'gn-documents-publics'
export const CACHE_RESERVES = 'gn-documents-reserves'

export const cacheDe = (reserve: boolean): string => (reserve ? CACHE_RESERVES : CACHE_PUBLICS)

/**
 * La clé d'une ressource : son adresse **absolue**, puisque Cache Storage rend ses
 * clés ainsi. La base de l'API est relative en production (`/v2/api`) ; sans API, le
 * jeu d'exemple se range sous une adresse du site qui ne se confond avec aucune autre.
 */
export function cleDeCopie(chemin: string, baseDeLApi: string, adresseDeLaPage: string): string {
  const adresse = baseDeLApi ? `${baseDeLApi.replace(/\/$/, '')}${chemin}` : `/gn-exemples${chemin}`
  return new URL(adresse, adresseDeLaPage).href
}

export interface Copie {
  id: string
  version: string
  /** L'empreinte de la forme lisible gardée : une autre, c'est un autre fichier. */
  reading_etag: string | null
  mode: ReadingMode
  reserve: boolean
  /** Instant ISO du dernier octet reçu. */
  gardee_a: string
  octets: number
  /** Clés de cache : la forme lisible d'abord, puis les images gardées. */
  cles: string[]
  /** Pages dont l'image est gardée. */
  pages_images: number[]
}

/**
 * Un magasin indexé par `id`, en IndexedDB ou en mémoire. `lire` rend `null` quand
 * le magasin ne se lit pas — à ne jamais prendre pour un magasin vide. Une écriture
 * qui n'a pas eu lieu lève.
 */
export interface Magasin<T extends { id: string }> {
  lire(): Promise<T[] | null>
  lireUne(id: string): Promise<T | null>
  poser(valeur: T): Promise<void>
  retirer(id: string): Promise<void>
  vider(): Promise<void>
}

export function magasinEnMemoire<T extends { id: string }>(): Magasin<T> {
  const entrees = new Map<string, T>()
  return {
    lire: async () => [...entrees.values()],
    lireUne: async (id) => entrees.get(id) ?? null,
    poser: async (valeur) => void entrees.set(valeur.id, valeur),
    retirer: async (id) => void entrees.delete(id),
    vider: async () => entrees.clear(),
  }
}

/** Ce que le code attend de Cache Storage — des clés en adresses, pas en `Request`. */
export interface CacheDeDocuments {
  lire(cle: string): Promise<Response | null>
  /** Sans lire le corps : une réponse lue et gardée retient ses octets, même son cache supprimé. */
  contient(cle: string): Promise<boolean>
  poser(cle: string, reponse: Response): Promise<void>
  retirer(cle: string): Promise<void>
  cles(): Promise<string[]>
}

export interface LesCaches {
  ouvrir(nom: string): Promise<CacheDeDocuments>
  supprimer(nom: string): Promise<void>
}

export interface DemandeDeTelechargement {
  id: string
  reserve: boolean
  /** La taille annoncée de la copie, pour la progression ; nulle si l'API ne la dit pas. */
  octets: number | null
  /** Instant ISO de la demande faite sans réseau. */
  demande_a: string
}

export interface Depots {
  caches: LesCaches
  copies: Magasin<Copie>
  aTelecharger: Magasin<DemandeDeTelechargement>
}

/** Une ressource reçue en entier, prête à être gardée. */
export interface Entree {
  cle: string
  reponse: Response
}

/**
 * Garde une copie **complète ou rien** : un `put` qui échoue — la place manque —
 * défait ceux qui l'ont précédé, et la fiche n'est posée qu'en dernier.
 */
export async function garderUneCopie(depots: Depots, copie: Copie, entrees: Entree[]): Promise<boolean> {
  const cache = await depots.caches.ouvrir(cacheDe(copie.reserve))
  const ancienne = await depots.copies.lireUne(copie.id)
  const posees: string[] = []
  try {
    for (const entree of entrees) {
      await cache.poser(entree.cle, entree.reponse)
      posees.push(entree.cle)
    }
    await depots.copies.poser(copie)
    return true
  } catch {
    // Une entrée posée a pu écraser celle d'une copie précédente : elle n'est plus
    // entière non plus, et s'efface avec le reste.
    for (const cle of new Set([...posees, ...(ancienne?.cles ?? [])])) await cache.retirer(cle).catch(() => undefined)
    await depots.copies.retirer(copie.id).catch(() => undefined)
    return false
  }
}

/** « Retirer du téléphone » : ses entrées de cache, sa fiche, sa demande en attente. */
export async function retirerUneCopie(depots: Depots, id: string): Promise<void> {
  const copie = await depots.copies.lireUne(id)
  if (copie) {
    const cache = await depots.caches.ouvrir(cacheDe(copie.reserve))
    for (const cle of copie.cles) await cache.retirer(cle)
  }
  await depots.copies.retirer(id)
  await depots.aTelecharger.retirer(id)
}

// Entrée par entrée d'abord : Chrome ne rend la place d'un cache supprimé qu'une fois
// la page déchargée, celle d'une entrée aussitôt (SC-005).
async function viderLeCache(depots: Depots, nom: string): Promise<void> {
  const cache = await depots.caches.ouvrir(nom)
  for (const cle of await cache.cles()) await cache.retirer(cle)
  await depots.caches.supprimer(nom)
}

/** « Tout retirer » : les documents seulement — ni les lectures, ni les réglages, ni la file de 0c. */
export async function toutRetirer(depots: Depots): Promise<void> {
  await viderLeCache(depots, CACHE_PUBLICS)
  await viderLeCache(depots, CACHE_RESERVES)
  await depots.copies.vider()
  await depots.aTelecharger.vider()
}

/**
 * Déconnexion ou accès perdu : les réservés s'effacent, les publics restent (SC-006).
 * Jamais appelé sur une API muette — c'est à l'appelant de n'avoir que des réponses.
 */
export async function effacerLesReserves(depots: Depots): Promise<void> {
  // Le cache d'abord : c'est lui qui rend un réservé lisible. Une fiche restée faute
  // d'avoir pu lire le magasin tombera à la prochaine vérification, sans son cache.
  await viderLeCache(depots, CACHE_RESERVES)
  for (const copie of (await depots.copies.lire()) ?? []) {
    if (copie.reserve) await depots.copies.retirer(copie.id)
  }
  for (const demande of (await depots.aTelecharger.lire()) ?? []) {
    if (demande.reserve) await depots.aTelecharger.retirer(demande.id)
  }
}

/** Ce que la bibliothèque dit d'un document, pour juger sa copie. */
export interface DocumentServi {
  id: string
  restricted: boolean
  accessible: boolean
  reading_etag: string | null
}

export interface Reconciliation {
  /** Dépublié, ou devenu réservé pour une personne sans accès (FR-034). */
  aEffacer: string[]
  /** Passé de public à réservé, ou l'inverse, lisible : ses entrées changent de cache. */
  aDeplacer: string[]
  /** Une autre empreinte : un autre fichier. Rien ne se retélécharge en silence. */
  autreVersion: string[]
}

export function reconcilier(copies: Copie[], documents: DocumentServi[]): Reconciliation {
  const servis = new Map(documents.map((d) => [d.id, d]))
  const r: Reconciliation = { aEffacer: [], aDeplacer: [], autreVersion: [] }
  for (const copie of copies) {
    const servi = servis.get(copie.id)
    if (!servi || (servi.restricted && !servi.accessible)) {
      r.aEffacer.push(copie.id)
      continue
    }
    if (servi.restricted !== copie.reserve) r.aDeplacer.push(copie.id)
    if (servi.reading_etag && copie.reading_etag && servi.reading_etag !== copie.reading_etag) {
      r.autreVersion.push(copie.id)
    }
  }
  return r
}

export async function appliquerLaReconciliation(depots: Depots, documents: DocumentServi[]): Promise<Reconciliation> {
  const r = reconcilier((await depots.copies.lire()) ?? [], documents)
  for (const id of r.aEffacer) await retirerUneCopie(depots, id)
  for (const id of r.aDeplacer) {
    const copie = await depots.copies.lireUne(id)
    if (!copie) continue
    const avant = await depots.caches.ouvrir(cacheDe(copie.reserve))
    const apres = await depots.caches.ouvrir(cacheDe(!copie.reserve))
    await deplacer(avant, apres, copie.cles)
    await depots.copies.poser({ ...copie, reserve: !copie.reserve })
  }
  return r
}

// `Cache` n'a pas de déplacement : lire, poser ailleurs, retirer.
async function deplacer(avant: CacheDeDocuments, apres: CacheDeDocuments, cles: string[]): Promise<void> {
  for (const cle of cles) {
    const reponse = await avant.lire(cle)
    if (reponse) await apres.poser(cle, reponse)
    await avant.retirer(cle)
  }
}

/**
 * Toutes les entrées d'une copie sont-elles encore là ? Le navigateur a pu en
 * vider une partie : sans elles, le lecteur montrerait une page blanche.
 */
export async function copieIntacte(depots: Depots, copie: Copie): Promise<boolean> {
  const cache = await depots.caches.ouvrir(cacheDe(copie.reserve))
  for (const cle of copie.cles) {
    if (!(await cache.contient(cle))) return false
  }
  return true
}

/**
 * À chaque ouverture : une copie incomplète redevient « non téléchargée », et une
 * entrée que plus aucune fiche ne désigne s'efface. Rend les documents perdus.
 */
export async function verifierLesCopies(depots: Depots): Promise<string[]> {
  const perdues: string[] = []
  const copies = await depots.copies.lire()
  // Un magasin illisible n'est pas un magasin vide : sans fiches, on ne conclut rien.
  if (copies === null) return perdues
  for (const copie of copies) {
    if (await copieIntacte(depots, copie)) continue
    perdues.push(copie.id)
    await retirerUneCopie(depots, copie.id)
  }
  const designees = new Set(copies.filter((c) => !perdues.includes(c.id)).flatMap((c) => c.cles))
  for (const nom of [CACHE_PUBLICS, CACHE_RESERVES]) {
    const cache = await depots.caches.ouvrir(nom)
    for (const cle of await cache.cles()) {
      if (!designees.has(cle)) await cache.retirer(cle)
    }
  }
  return perdues
}

export type Persistance = 'accordee' | 'refusee' | 'indisponible'

/** `navigator.storage` tel qu'il est, ou absent. */
export interface StockageDuNavigateur {
  persisted?: () => Promise<boolean>
  persist?: () => Promise<boolean>
}

/**
 * Sans ce droit, le navigateur peut effacer les copies quand le téléphone manque de
 * place, sans rien dire. Demandé au téléchargement, **tout de suite** : certains
 * navigateurs n'y répondent que dans le geste de la personne. Déjà accordé, `persist`
 * rend vrai sans rien demander.
 */
export async function demanderLaPersistance(stockage: StockageDuNavigateur | undefined): Promise<Persistance> {
  try {
    if (!stockage?.persist) return 'indisponible'
    return (await stockage.persist()) ? 'accordee' : 'refusee'
  } catch {
    return 'indisponible'
  }
}

/** Relue à l'ouverture : un navigateur peut accorder plus tard ce qu'il a refusé. */
export async function relireLaPersistance(stockage: StockageDuNavigateur | undefined): Promise<Persistance> {
  try {
    if (!stockage?.persisted) return 'indisponible'
    return (await stockage.persisted()) ? 'accordee' : 'refusee'
  } catch {
    return 'indisponible'
  }
}
