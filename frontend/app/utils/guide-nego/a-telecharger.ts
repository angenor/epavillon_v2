/**
 * « Télécharger au retour du réseau » : les demandes faites sans réseau, qui partent
 * quand il revient.
 *
 * **Ce n'est pas la file de 0c** (`file.ts`). Ce sont des lectures : ni compte à
 * protéger, ni empreinte à opposer. Elles partagent seulement ses déclencheurs —
 * ouverture, `online`, retour au premier plan — et une règle : deux déclencheurs à
 * la fois ne font partir qu'une fois.
 *
 * Une demande se retire au succès, et sur un refus (document dépublié, accès perdu)
 * ou une place manquante : la rejouer n'y changerait rien. Sur une panne, elle reste pour le déclencheur suivant.
 */
import type { DemandeDeTelechargement, Magasin } from './copies.ts'

/** `place` : le téléphone n'a pas pu tout garder — rejouer n'y changerait rien. */
export type IssueDeTelechargement = 'reussi' | 'refus' | 'place' | 'panne' | 'annule'

export interface DependancesATelecharger {
  magasin: Magasin<DemandeDeTelechargement>
  telecharger(demande: DemandeDeTelechargement): Promise<IssueDeTelechargement>
}

export interface FileATelecharger {
  demander(demande: DemandeDeTelechargement): Promise<void>
  partir(): Promise<Array<{ id: string; issue: IssueDeTelechargement }>>
}

export function creerFileATelecharger(deps: DependancesATelecharger): FileATelecharger {
  let enCours: Promise<Array<{ id: string; issue: IssueDeTelechargement }>> | null = null

  async function vider() {
    const suites: Array<{ id: string; issue: IssueDeTelechargement }> = []
    for (const demande of (await deps.magasin.lire()) ?? []) {
      // Relue juste avant : une déconnexion a pu retirer une demande réservée depuis.
      const encore = await deps.magasin.lireUne(demande.id)
      if (!encore) continue
      const issue = await deps.telecharger(encore).catch((): IssueDeTelechargement => 'panne')
      if (issue === 'reussi' || issue === 'refus' || issue === 'place') await deps.magasin.retirer(demande.id)
      suites.push({ id: demande.id, issue })
    }
    return suites
  }

  return {
    demander: (demande) => deps.magasin.poser(demande),
    partir() {
      enCours ??= vider().finally(() => (enCours = null))
      return enCours
    },
  }
}
