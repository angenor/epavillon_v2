/**
 * Les onglets de la barre basse.
 *
 * « Échanges » n'a pas de drapeau à lui : il paraît en QUATRIÈME position dès que
 * `negotiation.channels` ouvre la fonctionnalité, et disparaît avec elle. Ressources
 * ferme toujours la marche.
 */
export interface Onglet {
  /** Sert de clé de traduction dans `components/gn-barre-onglets.json`. */
  cle: string
  route: string
  picto: string
}

const ACCUEIL: Onglet = { cle: 'accueil', route: '/guide-nego', picto: 'home' }
const NEGOCIATIONS: Onglet = { cle: 'negociations', route: '/guide-nego/negociations', picto: 'nego' }
const FRANCOPHONIE: Onglet = { cle: 'francophonie', route: '/guide-nego/francophonie', picto: 'franco' }
const ECHANGES: Onglet = { cle: 'echanges', route: '/guide-nego/echanges', picto: 'chat' }
const RESSOURCES: Onglet = { cle: 'ressources', route: '/guide-nego/ressources', picto: 'res' }

export function onglets(echangesOuverts: boolean): Onglet[] {
  return echangesOuverts
    ? [ACCUEIL, NEGOCIATIONS, FRANCOPHONIE, ECHANGES, RESSOURCES]
    : [ACCUEIL, NEGOCIATIONS, FRANCOPHONIE, RESSOURCES]
}

/**
 * L'onglet dont l'écran est ouvert. L'accueil, racine de tous les autres, ne
 * s'allume que sur son adresse exacte ; un écran secondaire n'allume rien.
 */
export function ongletActif(chemin: string, liste: Onglet[]): Onglet | null {
  const sans = chemin.replace(/\/$/, '') || ACCUEIL.route
  return (
    liste.find(
      (onglet) => sans === onglet.route || (onglet !== ACCUEIL && sans.startsWith(`${onglet.route}/`)),
    ) ?? null
  )
}
