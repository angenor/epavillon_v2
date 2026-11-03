/**
 * La part de la présence qui se teste sans navigateur ni Nuxt : zone d'une
 * route, identifiant d'onglet, tracé de la dernière heure.
 */
import type { PresenceSample, PresenceZone } from '~/types/admin-presence'
import { baseRouteName } from './feature-modules.ts'

const LIVE = new Set([
  'activity-edition-session-room',
  'track-edition-track-room',
  'track-edition-track-live',
])
const DAY = new Set(['track-edition-track', 'track-edition-track-my-registration'])
// Le dépôt d'un dossier se fait au nom de l'organisation : même espace.
const ORGANIZATION = new Set(['proposal-form'])

const startsWith = (name: string, prefix: string): boolean =>
  name === prefix || name.startsWith(`${prefix}-`)

/** Zone de la page, d'après le nom de route Nuxt (chemin de fichier dans `pages/`). */
export function presenceZoneOf(routeName: unknown): PresenceZone {
  const name = baseRouteName(routeName)
  if (!name) return 'other'
  if (LIVE.has(name)) return 'live'
  if (name === 'programme') return 'programme'
  if (DAY.has(name)) return 'day'
  if (name === 'activity-edition-session') return 'activity'
  if (startsWith(name, 'organization') || ORGANIZATION.has(name)) return 'organization'
  if (startsWith(name, 'admin')) return 'admin'
  if (startsWith(name, 'guide-nego') || name === 'negotiations') return 'negotiation'
  return 'other'
}

/** Même règle que l'API : 8 à 64 caractères parmi lettres, chiffres, `-` et `_`. */
export const isValidTabId = (id: string | null | undefined): id is string =>
  typeof id === 'string' && /^[A-Za-z0-9_-]{8,64}$/.test(id)

export const PRESENCE_TAB_KEY = 'epavillon.presence.tab'

/**
 * L'identifiant de cet onglet : relu de `sessionStorage` (propre à l'onglet),
 * régénéré s'il manque ou ne convient pas. Sans stockage, un identifiant par
 * chargement de page.
 */
export function presenceTabId(
  storage: Pick<Storage, 'getItem' | 'setItem'> | null,
  make: () => string,
): string {
  let stored: string | null = null
  try {
    stored = storage?.getItem(PRESENCE_TAB_KEY) ?? null
  } catch {
    stored = null
  }
  if (isValidTabId(stored)) return stored
  const id = make()
  try {
    storage?.setItem(PRESENCE_TAB_KEY, id)
  } catch {
    // Stockage refusé : l'identifiant vaut pour ce chargement.
  }
  return id
}

/** Largeur d'une barre, en pour cent de la zone la plus fréquentée ; jamais invisible. */
export function presenceBarWidth(count: number, max: number): number {
  if (max <= 0 || count <= 0) return 0
  return Math.max(2, Math.round((count / max) * 100))
}

/** Secondes écoulées depuis le relevé, jamais négatives (horloges décalées). */
export function secondsSince(at: string, now: number): number {
  const time = Date.parse(at)
  if (Number.isNaN(time)) return 0
  return Math.max(0, Math.round((now - time) / 1000))
}

export interface SparklinePaths {
  line: string
  area: string
}

/**
 * Tracé de la dernière heure dans un cadre `width` × `height` : la ligne et la
 * surface dessous. Le plus haut point touche le haut moins `pad`, zéro le bas.
 * Moins de deux points : rien à tracer.
 */
export function sparklinePaths(
  samples: Pick<PresenceSample, 'total'>[],
  width: number,
  height: number,
  pad = 4,
): SparklinePaths | null {
  if (samples.length < 2) return null
  const max = Math.max(1, ...samples.map((s) => s.total))
  const step = width / (samples.length - 1)
  const round = (n: number): number => Math.round(n * 10) / 10
  const points = samples.map((s, i) => {
    const x = round(i * step)
    const y = round(height - (s.total / max) * (height - pad))
    return `${x} ${y}`
  })
  const line = `M${points.join(' L')}`
  return { line, area: `${line} L${round(width)} ${height} L0 ${height} Z` }
}
