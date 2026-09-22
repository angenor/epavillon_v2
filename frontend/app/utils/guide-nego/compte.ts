/**
 * L'état du compte, tel que Guide Négo l'affiche — et ce qu'il en fait **hors
 * connexion**.
 *
 * Tout est pur ici, et c'est délibéré : la règle qui compte (quand réclamer une
 * reconnexion) se trompe silencieusement, et un test doit pouvoir la prendre
 * sans navigateur.
 */
import type { AuthenticatedPerson } from '~/types/auth'

export interface EtatDuCompte {
  connectee: boolean
  /** L'identifiant de la personne : c'est lui qui tient une intention à son compte. */
  id: string | null
  prenom: string | null
  nom: string | null
  adresse: string | null
  /** Une adresse non confirmée est la seule chose qui retient une connexion. */
  adresseConfirmee: boolean
  /** « Android · Chrome », tel que l'appareil s'est déclaré. */
  appareil: string | null
  /** Depuis quand cette session est ouverte, en ISO. */
  ouverteLe: string | null
}

export const COMPTE_DECONNECTE: EtatDuCompte = {
  connectee: false,
  id: null,
  prenom: null,
  nom: null,
  adresse: null,
  adresseConfirmee: false,
  appareil: null,
  ouverteLe: null,
}

/**
 * **Un objet simple, jamais la réponse brute.** Ce que rend cette fonction part
 * dans IndexedDB et dans l'état du rendu serveur : une instance de classe ne s'y
 * sérialise pas, et une réponse entière y garderait des champs dont l'écran n'a
 * pas l'usage.
 */
export function etatDuCompte(moi: AuthenticatedPerson | null): EtatDuCompte {
  if (!moi) return COMPTE_DECONNECTE

  return {
    connectee: true,
    id: moi.id,
    prenom: moi.first_name ?? null,
    nom: moi.last_name ?? null,
    adresse: moi.primary_email ?? null,
    adresseConfirmee: moi.email_verified_at !== null && moi.email_verified_at !== undefined,
    appareil: moi.session?.device_label ?? null,
    ouverteLe: moi.session?.issued_at ?? null,
  }
}

/**
 * Faut-il réclamer une reconnexion ? **Seulement avec du réseau** (FR-006 ter).
 *
 * C'est le geste réflexe d'un client web — jeton périmé, on renvoie à la
 * connexion — et il transforme une salle sans réseau en application inutilisable.
 * Une reconnexion sans réseau est impossible : la demander est une impasse, et
 * ce qui a été lu reste lisible en attendant.
 */
export function reconnexionAReclamer(
  etat: EtatDuCompte | null,
  enLigne: boolean,
  pret: boolean,
): boolean {
  if (!pret || !enLigne) return false
  return etat === null || !etat.connectee
}

/**
 * Ce que l'écran montre du compte hors connexion : ce qui a été lu, avec l'heure
 * de sa lecture. **Jamais un écran vide** — et jamais non plus la prétention
 * d'être connectée : `connectee` reste ce que la dernière lecture disait.
 */
export function compteLisibleHorsConnexion(
  etat: EtatDuCompte | null,
  luA: string | null,
): { etat: EtatDuCompte; luA: string | null } {
  return { etat: etat ?? COMPTE_DECONNECTE, luA }
}
