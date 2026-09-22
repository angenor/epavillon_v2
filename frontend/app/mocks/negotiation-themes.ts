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

export function mesThematiques(): MyThemes {
  return { themes: suivies.map((t) => ({ ...t })) }
}

/** Remplacement en bloc, comme l'API : rejouer le même corps ne change rien. */
export function suivreDesThematiques(codes: string[]): MyThemes {
  const voulus = [...new Set(codes.map((c) => c.trim()).filter(Boolean))]
  const deja = new Map(suivies.map((t) => [t.code, t]))
  suivies = voulus.map(
    (code) => deja.get(code) ?? { code, followed_at: new Date().toISOString() },
  )
  return mesThematiques()
}

/**
 * L'empreinte **sur les codes triés**, comme l'API la calcule : deux appareils
 * dans deux langues voient la même pour un même état. Elle est lisible plutôt
 * que hachée — rien ici n'a besoin d'être opaque.
 */
export function empreinteDesThematiques(): string {
  return `"${suivies.map((t) => t.code).sort().join('.')}"`
}

export function reinitialiserLesThematiques(): void {
  suivies = []
}
