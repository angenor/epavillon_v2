/**
 * Ce que l'application Guide Négo appelle.
 *
 * **Le bloc réutilise les routes du site, il ne les redouble pas.** La connexion,
 * l'inscription et les deux renvois de courriel sont ceux de l'ePavillon,
 * inchangés : la seule chose que l'application ajoute est l'objet `client`, et
 * elle l'ajoute **ici** plutôt que dans chaque écran. Un écran qui l'oublierait
 * ouvrirait une session de douze heures marquée « site », sans qu'aucune erreur
 * ne le dise — c'est le piège n° 1 de cette étape.
 *
 * Les routes du module `negotiation` — l'accès, le code d'invitation — s'ajoutent
 * ici avec le récit qui les demande.
 */
import type {
  AccessRequestView,
  AccessStateView,
  CreateAccessRequestPayload,
  MyThemes,
  RedeemResult,
} from '~/types/negotiation'
import type { LegalText, LegalTextKey } from '~/types/platform'
import type { AppelsEtiquetes, AvecEmpreinte } from './etiquete'
import type {
  PasswordResetRequestResult,
  RegisterPayload,
  RegisterResult,
  ResendVerificationResult,
  SessionClient,
} from '~/types/auth'
import { appareilDeclare } from '~/utils/guide-nego/appareil'

/**
 * Les méthodes du site que l'application réemprunte. Déclarées en contrat
 * plutôt qu'importées : la fabrique n'a pas à connaître tout `useApi()`, et la
 * liste dit exactement ce que Guide Négo lui doit.
 */
export interface AuthEmprunte {
  register: (payload: RegisterPayload) => Promise<RegisterResult>
  resendVerification: (
    email: string,
    client?: SessionClient,
  ) => Promise<ResendVerificationResult>
  requestPasswordReset: (
    email: string,
    client?: SessionClient,
  ) => Promise<PasswordResetRequestResult>
}

type Mocks = typeof import('~/mocks')

/** Les primitives de `useApi()` dont ce bloc a besoin. */
export interface Primitives {
  call: <T>(path: string, fromMocks: (m: Mocks) => T | Promise<T>) => Promise<T>
  send: <T>(
    path: string,
    body: object,
    fromMocks: (m: Mocks) => T | Promise<T>,
    method?: 'POST' | 'PUT' | 'PATCH' | 'DELETE',
  ) => Promise<T>
  /** Les appels qui rendent l'empreinte de l'état ; `lireEtiquete` sait aussi la relecture conditionnelle. */
  lireEtiquete: AppelsEtiquetes['lireEtiquete']
  ecrireEtiquete: AppelsEtiquetes['ecrireEtiquete']
}

export function createGuideNegoApi({
  auth,
  call,
  send,
  lireEtiquete,
  ecrireEtiquete,
}: { auth: AuthEmprunte } & Primitives) {
  return {
    // La CONNEXION n'est pas ici : elle passe par le store du site, qui tient la
    // session et son témoin — voir `useGnSession().connecter()`. Deux chemins
    // pour ouvrir une session seraient un chemin de trop.

    /** Inscription depuis l'application : le lien du courriel y ramènera. */
    inscription: (fiche: Omit<RegisterPayload, 'client'>) =>
      auth.register({ ...fiche, client: appareilDeclare() }),

    /** Renvoi du lien de vérification. Réponse invariable. */
    renvoyerLeCourriel: (adresse: string) =>
      auth.resendVerification(adresse, appareilDeclare()),

    /** Demande de nouveau mot de passe. Réponse invariable, compte ou non. */
    motDePasseOublie: (adresse: string) =>
      auth.requestPasswordReset(adresse, appareilDeclare()),

    /**
     * L'état d'accès, **en une seule lecture** : le mode d'admission, l'état,
     * ce que l'accès ouvre, les réseaux, la demande. Le parcours d'entrée, le
     * verrou et « Mon accès » lisent celle-ci et rien d'autre.
     */
    acces: (): Promise<AccessStateView> =>
      call('/negotiation/me/access', (m) => m.monAcces()),

    /**
     * Saisir le code reçu. **Les neuf issues sortent en 200** : le résultat est
     * une réponse, jamais une exception, et son `message` s'affiche tel quel.
     *
     * L'identifiant d'appareil part avec la saisie — comme information, pour
     * qu'un administrateur puisse lire une série d'échecs. Le compteur, lui, se
     * tient par personne.
     */
    saisirLeCode: (code: string): Promise<RedeemResult> =>
      send('/negotiation/invitation-codes/redeem', {
        code,
        device_id: appareilDeclare().device_id,
      }, (m) => m.saisirUnCode(code)),

    /**
     * Demander l'accès, quand le mode d'admission exige une approbation.
     *
     * **Une seule demande en attente par personne et par portée**, et c'est la
     * base qui le tient : deux appareils qui envoient ensemble ne produisent
     * qu'une ligne, et le second reçoit un conflit traduit en français.
     */
    demanderLAcces: (charge: CreateAccessRequestPayload = {}): Promise<AccessRequestView> =>
      send('/negotiation/access-requests', charge, (m) => m.demanderLAcces(charge)),

    /**
     * Retirer sa demande — « **annulée** », son propre fait, ce qui arrive
     * quand on reçoit un code et qu'on entre par lui. « Révoquée » est réservé
     * à un accès qu'un administrateur retire (FR-026).
     */
    annulerSaDemande: (requestId: string): Promise<void> =>
      send(
        `/negotiation/access-requests/${requestId}`,
        {},
        (m) => m.annulerSaDemande(requestId),
        'DELETE',
      ),

    /**
     * Les thématiques suivies, **avec l'empreinte de l'état**. Des codes, pas
     * de libellés : ceux-ci viennent de `reference.terms('negotiation_theme')`,
     * seule source, que l'application lit de toute façon.
     */
    mesThematiques: (): Promise<AvecEmpreinte<MyThemes>> =>
      lireEtiquete('/negotiation/me/themes', (m) => ({
        valeur: m.mesThematiques(),
        empreinte: m.empreinteDesThematiques(),
      })),

    /**
     * Remplace la liste **entière**. L'empreinte reçue part en `If-Match` :
     * absente, l'API accepte — l'écran en ligne vient de lire ; périmée, elle
     * rend `412` et l'intention s'abandonne au lieu d'écraser un choix plus
     * récent fait sur un autre appareil.
     */
    suivreDesThematiques: (
      codes: string[],
      empreinte?: string | null,
    ): Promise<AvecEmpreinte<MyThemes>> =>
      ecrireEtiquete(
        '/negotiation/me/themes',
        { codes },
        (m) => ({
          valeur: m.suivreDesThematiques(codes),
          empreinte: m.empreinteDesThematiques(),
        }),
        empreinte,
      ),

    /**
     * Un texte qui engage — la même route que le site, donc le même texte et la
     * même version. Sans session. Tant que l'IFDD ne l'a pas fourni, il part
     * « en attente », sans corps.
     */
    texte: (cle: LegalTextKey): Promise<LegalText> =>
      call(`/legal/${cle}`, (m) => m.texteJuridique(cle)),
  }
}
