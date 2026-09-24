/**
 * Ce que ce téléphone retient de la lecture : la taille du texte, la page atteinte
 * dans chaque document, les documents déjà ouverts et les derniers ouverts.
 *
 * Quelques kilo-octets en `localStorage`, jamais effacés par « Tout retirer » : ce
 * ne sont ni des lectures ni des copies (R11). Tout passe par un `Stockage` injecté
 * — celui de `stockage.ts` en usage, qui ne lève pas en navigation privée.
 */
import type { CauseDeBascule } from './pdf/bascule.ts'

// Préfixées `gn.`, comme toutes les clés de Guide Négo (`stockage.ts`).
export const CLE_TAILLE = 'gn.lecture-taille'
export const CLE_PROGRESSION = 'gn.lecture-progression'
export const CLE_OUVERTS = 'gn.documents-ouverts'
export const CLE_RECENTS = 'gn.documents-recents'
/** L'issue de `navigator.storage.persist()` : « Mes documents » dit un refus. */
export const CLE_PERSISTANCE = 'gn.stockage-persistant'
export const CLE_BASCULES = 'gn.lecture-bascules'

export interface Stockage {
  lire(cle: string): string | null
  poser(cle: string, valeur: string): void
}

/** 17, 20 ou 24 px, interligne 1,5 (R14). */
export const TAILLES = [17, 20, 24] as const
export type TailleDuTexte = (typeof TAILLES)[number]

export interface Progression {
  page: number
  /** Instant ISO de la dernière page notée. */
  a: string
}

export interface Recent {
  id: string
  a: string
}

const RECENTS_GARDES = 5
const JOURS_NOUVEAU = 7

function lireJson<T>(stockage: Stockage, cle: string, repli: T): T {
  try {
    const brut = stockage.lire(cle)
    const lu: unknown = brut ? JSON.parse(brut) : null
    // Une valeur d'une autre forme — `null`, un nombre, un tableau pour un objet — vaut son absence.
    return lu !== null && typeof lu === 'object' && Array.isArray(lu) === Array.isArray(repli) ? (lu as T) : repli
  } catch {
    return repli
  }
}

const ecrireJson = (stockage: Stockage, cle: string, valeur: unknown): void =>
  stockage.poser(cle, JSON.stringify(valeur))

export function lireTaille(stockage: Stockage): TailleDuTexte {
  const lue = Number(stockage.lire(CLE_TAILLE))
  return (TAILLES as readonly number[]).includes(lue) ? (lue as TailleDuTexte) : TAILLES[0]
}

export const poserTaille = (stockage: Stockage, taille: TailleDuTexte): void =>
  stockage.poser(CLE_TAILLE, String(taille))

// Par document ET par version : une autre version s'ouvre au début (FR-039).
const cleDeProgression = (id: string, version: string) => `${id}@${version}`

export function lireProgression(stockage: Stockage, id: string, version: string): Progression | null {
  return lireJson<Record<string, Progression>>(stockage, CLE_PROGRESSION, {})[cleDeProgression(id, version)] ?? null
}

export function noterProgression(stockage: Stockage, id: string, version: string, page: number, a: string): void {
  const toutes = lireJson<Record<string, Progression>>(stockage, CLE_PROGRESSION, {})
  toutes[cleDeProgression(id, version)] = { page, a }
  ecrireJson(stockage, CLE_PROGRESSION, toutes)
}

/** À la relecture de la liste : la progression d'une version qui n'est plus servie s'oublie. */
export function oublierLesVersionsDisparues(stockage: Stockage, servies: Array<{ id: string; version: string }>): void {
  const gardees = new Set(servies.map((d) => cleDeProgression(d.id, d.version)))
  const toutes = lireJson<Record<string, Progression>>(stockage, CLE_PROGRESSION, {})
  const restantes = Object.fromEntries(Object.entries(toutes).filter(([cle]) => gardees.has(cle)))
  if (Object.keys(restantes).length !== Object.keys(toutes).length) ecrireJson(stockage, CLE_PROGRESSION, restantes)
}

/** Sa fiche ouverte, un document n'est plus « Nouveau » ; il n'entre pas pour autant dans les récents. */
export function marquerVu(stockage: Stockage, id: string, a: string): void {
  const ouverts = lireJson<Record<string, string>>(stockage, CLE_OUVERTS, {})
  if (ouverts[id]) return
  ouverts[id] = a
  ecrireJson(stockage, CLE_OUVERTS, ouverts)
}

/** Ouvrir un document le marque ouvert, et le range en tête des derniers ouverts. */
export function noterOuverture(stockage: Stockage, id: string, a: string): void {
  marquerVu(stockage, id, a)
  const recents = lireJson<Recent[]>(stockage, CLE_RECENTS, []).filter((r) => r.id !== id)
  ecrireJson(stockage, CLE_RECENTS, [{ id, a }, ...recents].slice(0, RECENTS_GARDES))
}

export const lireRecents = (stockage: Stockage): Recent[] => lireJson<Recent[]>(stockage, CLE_RECENTS, [])

/**
 * « Nouveau » : publié il y a moins de sept jours **et** jamais ouvert sur ce
 * téléphone. Ouvert une fois, il ne l'est plus — ici seulement.
 */
export function estNouveau(stockage: Stockage, document: { id: string; published_at: string }, maintenant: Date): boolean {
  const publie = Date.parse(document.published_at)
  if (!Number.isFinite(publie)) return false
  const recent = maintenant.getTime() - publie < JOURS_NOUVEAU * 24 * 60 * 60 * 1000
  return recent && !lireJson<Record<string, string>>(stockage, CLE_OUVERTS, {})[document.id]
}

/** Les bascules du lecteur sur le texte (FR-012 bis) : compter seulement, jamais relu pour décider. */
export interface CompteDesBascules {
  nombre: number
  cause: CauseDeBascule
  a: string
}

export function lireLesBascules(stockage: Stockage): CompteDesBascules | null {
  const { nombre, cause, a } = lireJson<Partial<Record<keyof CompteDesBascules, unknown>>>(stockage, CLE_BASCULES, {})
  const lisible = typeof nombre === 'number' && Number.isInteger(nombre) && nombre > 0 && typeof a === 'string'
  return lisible && (cause === 'erreur' || cause === 'delai') ? { nombre, cause, a } : null
}

export function compterUneBascule(stockage: Stockage, cause: CauseDeBascule, a: string): CompteDesBascules {
  const compte = { nombre: (lireLesBascules(stockage)?.nombre ?? 0) + 1, cause, a }
  ecrireJson(stockage, CLE_BASCULES, compte)
  return compte
}
