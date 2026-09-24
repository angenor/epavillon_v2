/**
 * Les gestes du lecteur de pages (contrat `lecteur.md` § Gestes) : un toucher
 * simple n'est émis qu'après 300 ms sans second toucher, sinon c'est un double
 * toucher. Le pincement, lui, passe par `TouchManager` et annule le toucher en attente.
 */

export interface Toucher { x: number; y: number; t: number }
export type Geste = { type: 'simple'; x: number; y: number } | { type: 'double'; x: number; y: number }
export interface Minuterie { planifier(fn: () => void, ms: number): unknown; annuler(poignee: unknown): void }

export const DELAI_DOUBLE_TOUCHER = 300
export const DISTANCE_DOUBLE_TOUCHER = 24
export const PLAFOND_DE_GROSSISSEMENT = 4

export function creerLeLecteurDeGestes(options: {
  minuterie: Minuterie
  surGeste: (g: Geste) => void
  delai?: number
  distance?: number
}): { toucher(t: Toucher): void; annulerEnCours(): void; arreter(): void } {
  const { minuterie, surGeste } = options
  const delai = options.delai ?? DELAI_DOUBLE_TOUCHER
  const distance = options.distance ?? DISTANCE_DOUBLE_TOUCHER
  let enAttente: Toucher | null = null
  let poignee: unknown = null

  function oublier(): void {
    if (poignee !== null) minuterie.annuler(poignee)
    poignee = null
    enAttente = null
  }

  function mettreEnAttente(t: Toucher): void {
    enAttente = t
    poignee = minuterie.planifier(() => {
      poignee = null
      const premier = enAttente
      enAttente = null
      if (premier) surGeste({ type: 'simple', x: premier.x, y: premier.y })
    }, delai)
  }

  return {
    toucher(t) {
      const premier = enAttente
      if (!premier) return mettreEnAttente(t)
      oublier()
      const proche = Math.hypot(t.x - premier.x, t.y - premier.y) < distance
      if (t.t - premier.t < delai && proche) return surGeste({ type: 'double', x: t.x, y: t.y })
      surGeste({ type: 'simple', x: premier.x, y: premier.y })
      mettreEnAttente(t)
    },
    annulerEnCours: oublier,
    arreter: oublier,
  }
}

/** `largeur` est l'échelle qui fait tenir la page dans la largeur de l'écran. */
export function echelleApresDoubleToucher(courante: number, largeur: number, plafond = PLAFOND_DE_GROSSISSEMENT): number {
  // 1 % de marge : l'échelle recalculée par pdf.js n'est jamais exactement celle de la largeur.
  if (courante > largeur * 1.01) return largeur
  return Math.min(courante * 2, largeur * plafond)
}

export function bornerLEchelle(echelle: number, largeur: number, plafond = PLAFOND_DE_GROSSISSEMENT): number {
  return Math.min(Math.max(echelle, largeur), largeur * plafond)
}
