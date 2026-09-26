/**
 * Les thématiques suivies, sans API — **des codes, jamais de libellés**.
 *
 * Les libellés vivent dans `mocks/reference.ts`, avec le reste du vocabulaire
 * `negotiation_theme` : une seule source ici comme en base, sans quoi le jeu
 * d'exemple mentirait sur le point que le contrat protège.
 *
 * Le jeu part **vide** : c'est l'état d'une personne qui vient d'entrer, et
 * c'est lui qui fait paraître l'écran de premier choix.
 */
import type { FollowedTheme, MyThemes } from '~/types/negotiation'

let suivies: FollowedTheme[] = []
let allumees: string[] = []

export function mesThematiques(): MyThemes {
  return { themes: suivies.map((t) => ({ ...t })), notify: [...allumees] }
}

/** Remplacement en bloc, comme l'API : rejouer le même corps ne change rien. */
export function suivreDesThematiques(codes: string[]): MyThemes {
  const voulus = [...new Set(codes.map((c) => c.trim()).filter(Boolean))]
  const deja = new Map(suivies.map((t) => [t.code, t]))
  suivies = voulus.map(
    (code) => deja.get(code) ?? { code, followed_at: new Date().toISOString() },
  )
  allumees = allumees.filter((code) => voulus.includes(code))
  return mesThematiques()
}

/** Les thématiques dont on est prévenu, parmi les suivies ; `null` : une n'est pas suivie. */
export function notifierDesThematiques(codes: string[]): MyThemes | null {
  if (codes.some((code) => !suivies.some((t) => t.code === code))) return null
  allumees = suivies.map((t) => t.code).filter((code) => codes.includes(code))
  return mesThematiques()
}

/**
 * L'empreinte **sur les codes triés**, comme l'API la calcule : deux appareils
 * dans deux langues voient la même pour un même état. Elle est lisible plutôt
 * que hachée — rien ici n'a besoin d'être opaque.
 */
export function empreinteDesThematiques(): string {
  const codes = [...suivies.map((t) => t.code), ...allumees.map((c) => `!${c}`)]
  return `"${codes.sort().join('.')}"`
}

export function reinitialiserLesThematiques(): void {
  suivies = []
  allumees = []
}
