/**
 * La seconde sécurité du lecteur (FR-012 bis, ADR-022) : si pdf.js échoue, ou ne
 * dessine pas la première page après 8 s de travail, le lecteur bascule sur le texte.
 * Le décompte se suspend tant qu'une plage est en route : le réseau lent ne fait
 * jamais basculer. Rien n'est lu dans un stockage : un échec passé ne condamne
 * pas l'ouverture suivante.
 */
import type { Minuterie } from './gestes.ts'

export type CauseDeBascule = 'erreur' | 'delai'
export const DELAI_DE_BASCULE_MS = 8000

export function creerLaSurveillance(options: {
  minuterie: Minuterie
  maintenant: () => number
  surBascule: (cause: CauseDeBascule) => void
  delai?: number
}): { demarrer(): void; reseau(enAttente: boolean): void; erreur(): void; rendue(): void; arreter(): void } {
  const { minuterie, maintenant, surBascule } = options
  const delai = options.delai ?? DELAI_DE_BASCULE_MS
  let demarree = false
  let finie = false
  let enAttenteDuReseau = false
  let ecoule = 0
  let depuis: number | null = null
  let poignee: unknown = null

  function courir(): void {
    depuis = maintenant()
    poignee = minuterie.planifier(() => {
      poignee = null
      finie = true
      surBascule('delai')
    }, Math.max(0, delai - ecoule))
  }

  function suspendre(): void {
    if (depuis !== null) ecoule += maintenant() - depuis
    depuis = null
    if (poignee !== null) minuterie.annuler(poignee)
    poignee = null
  }

  function finir(): void {
    suspendre()
    finie = true
  }

  return {
    demarrer() {
      if (demarree || finie) return
      demarree = true
      if (!enAttenteDuReseau) courir()
    },
    reseau(enAttente) {
      if (finie || enAttente === enAttenteDuReseau) return
      enAttenteDuReseau = enAttente
      if (!demarree) return
      if (enAttente) suspendre()
      else courir()
    },
    erreur() {
      if (finie) return
      finir()
      surBascule('erreur')
    },
    rendue: () => { if (!finie) finir() },
    arreter: () => { if (!finie) finir() },
  }
}
