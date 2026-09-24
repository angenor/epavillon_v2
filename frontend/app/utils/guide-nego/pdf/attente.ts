/**
 * L'attente de la première page en ligne (FR-009 bis) : au bout de 3 s sans page,
 * deux sorties — lire le texte en attendant, ou télécharger. La page prête prend la
 * place du texte, sauf si la personne a choisi d'y rester ou changé de mode elle-même.
 */
import type { Minuterie } from './gestes.ts'

export type ModeAffiche = 'attente' | 'texte' | 'pages'
export interface EtatDAttente { mode: ModeAffiche; sorties: boolean; resteSurLeTexte: boolean; pagePrete: boolean }
export const DELAI_DES_SORTIES_MS = 3000

export function creerLAttente(options: {
  minuterie: Minuterie
  surChangement: (etat: EtatDAttente) => void
  delai?: number
}): {
  etat(): EtatDAttente
  demarrer(): void
  lireLeTexte(): void
  resterSurLeTexte(): void
  pagePrete(): void
  choisirLeMode(mode: 'texte' | 'pages'): void
  arreter(): void
} {
  const { minuterie, surChangement } = options
  const delai = options.delai ?? DELAI_DES_SORTIES_MS
  let etat: EtatDAttente = { mode: 'attente', sorties: false, resteSurLeTexte: false, pagePrete: false }
  let poignee: unknown = null

  function changer(partiel: Partial<EtatDAttente>): void {
    etat = { ...etat, ...partiel }
    surChangement({ ...etat })
  }

  function annuler(): void {
    if (poignee !== null) minuterie.annuler(poignee)
    poignee = null
  }

  return {
    etat: () => ({ ...etat }),
    demarrer() {
      annuler()
      poignee = minuterie.planifier(() => {
        poignee = null
        if (!etat.pagePrete) changer({ sorties: true })
      }, delai)
    },
    lireLeTexte: () => changer({ mode: 'texte' }),
    resterSurLeTexte: () => changer({ resteSurLeTexte: true }),
    pagePrete() {
      annuler()
      const remplace = etat.mode === 'attente' || (etat.mode === 'texte' && !etat.resteSurLeTexte)
      changer({ pagePrete: true, sorties: false, mode: remplace ? 'pages' : etat.mode })
    },
    choisirLeMode: (mode) => changer({ mode, resteSurLeTexte: true }),
    arreter: annuler,
  }
}
