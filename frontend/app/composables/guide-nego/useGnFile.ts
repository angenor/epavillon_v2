/**
 * La file des écritures différées, branchée sur l'application.
 *
 * Les règles vivent dans `utils/guide-nego/file.ts`, pures et testées. Ce composable
 * n'apporte que ce que le navigateur seul fournit : le magasin IndexedDB, la personne
 * connectée, la traduction d'une erreur d'API en réponse, et l'avis à montrer.
 *
 * **Chaque écran qui écrit s'inscrit ici**, avec ce qu'il sait faire de sa clé :
 * envoyer l'intention, et relire l'état vrai après. La file, elle, ne connaît aucun
 * écran — c'est ce qui la fera servir au parcours (étape 2) et aux signalements (3b).
 */
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { magasinDesEcritures } from '~/utils/guide-nego/garde'
import {
  creerFile,
  type Expediteur,
  type File,
  type Intention,
  type Reponse,
  type Suite,
} from '~/utils/guide-nego/file'

/** Ce que la file dit à la personne : un choix périmé, ou refusé. */
export type AvisDeFile = Suite

/** Traduit ce que l'envoi a levé. Un succès est un envoi qui n'a rien levé. */
export function reponseDe(erreur: unknown): Reponse {
  const normalisee = normalizeApiError(erreur)
  if (normalisee instanceof ApiRequestError) {
    return { statut: 'refus', code: normalisee.status, message: normalisee.message }
  }
  return { statut: 'panne' }
}

// La file et ses expéditeurs portent des fonctions : ils ne vont pas dans un
// `useState`, que le rendu serveur sérialise. Ils vivent au niveau du module, sur le
// client seulement — le serveur n'a ni IndexedDB ni réseau à rattraper.
const expediteurs = new Map<string, Expediteur>()
// Une clé par objet — `favori-<id>` : un seul expéditeur sert toute la famille.
const familles = new Map<string, Expediteur>()
let file: File | null = null

function expediteurDe(cle: string): Expediteur | undefined {
  return expediteurs.get(cle) ?? [...familles].find(([prefixe]) => cle.startsWith(prefixe))?.[1]
}

function expediteur(envoyer: (intention: Intention) => Promise<unknown>, relire?: () => Promise<void> | void): Expediteur {
  return {
    envoyer: (intention) =>
      envoyer(intention).then(
        (): Reponse => ({ statut: 'succes' }),
        (erreur: unknown) => reponseDe(erreur),
      ),
    relire,
  }
}

export function useGnFile() {
  const session = useGnSession()
  const avis = useState<AvisDeFile | null>('gn-file-avis', () => null)

  if (import.meta.client && !file) {
    file = creerFile({
      magasin: magasinDesEcritures,
      expediteurs: expediteurDe,
      personne: () => session.compte.value.id,
      signaler: (suite) => (avis.value = suite),
    })
  }

  /**
   * Inscrit ce qu'un écran sait faire de sa clé. `envoyer` fait l'appel d'API et
   * ne rend rien : une exception est traduite en réponse, un retour normal vaut
   * succès.
   */
  function inscrire(
    cle: string,
    envoyer: (intention: Intention) => Promise<unknown>,
    relire?: () => Promise<void> | void,
  ): void {
    expediteurs.set(cle, expediteur(envoyer, relire))
  }

  /** Comme `inscrire`, pour toutes les clés qui commencent par `prefixe`. */
  function inscrireFamille(
    prefixe: string,
    envoyer: (intention: Intention) => Promise<unknown>,
    relire?: () => Promise<void> | void,
  ): void {
    familles.set(prefixe, expediteur(envoyer, relire))
  }

  function poser(cle: string, corps: unknown, empreinte: string | null): Promise<void> {
    const personne = session.compte.value.id
    if (!personne || !file) return Promise.resolve()
    return file.poser({
      cle,
      corps,
      empreinte,
      personne,
      prise_a: new Date().toISOString(),
    })
  }

  /** Fait partir ce qui attend. Deux déclencheurs à la fois n'envoient qu'une fois. */
  function partir(): Promise<Suite[]> {
    return file ? file.partir() : Promise.resolve([])
  }

  return {
    avis,
    effacerLAvis: () => (avis.value = null),
    inscrire,
    inscrireFamille,
    poser,
    partir,
  }
}
