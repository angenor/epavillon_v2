/**
 * Contrats d'AUTHENTIFICATION — ce qui circule entre l'écran et l'API.
 *
 * Ces types ne décrivent AUCUNE table : ils nomment les requêtes et les réponses
 * des cinq écrans du prompt A1. Ils s'appuient sur `030_identity.sql` sans le
 * recopier — `identity.people`, `accounts`, `one_time_tokens` sont déjà dérivés
 * dans `types/identity.ts`, et les secrets n'en franchissent jamais la frontière.
 *
 * POURQUOI DES RÉSULTATS EN UNION PLUTÔT QUE DES EXCEPTIONS. Une connexion qui
 * échoue n'est pas une panne : c'est une réponse. Six issues sont prévues, le
 * compilateur oblige l'écran à les traiter toutes, et le jour où le second
 * facteur sera branché — arbitré HORS du prompt B1 le 20/08 —, l'oubli d'une
 * branche ne compilera pas.
 *
 * DISCRÉTION — RÈGLE DE L'ÉCRAN, PAS DU TYPE. « Ne jamais révéler si une adresse
 * existe » se joue dans ce que l'API accepte de renvoyer :
 *   · `invalid_credentials` est la SEULE issue possible quand le mot de passe
 *     est faux ou l'adresse inconnue — les deux cas sont indiscernables ;
 *   · `locked`, `suspended`, `email_unverified` et `mfa_required` supposent
 *     TOUS que le mot de passe ait été vérifié, donc que l'identité soit prouvée.
 *     Les renvoyer n'apprend rien à qui ne connaît pas déjà le mot de passe ;
 *   · l'inscription et la demande de réinitialisation renvoient TOUJOURS la même
 *     issue, compte existant ou non. C'est le courriel envoyé qui diffère.
 */

import type { Person } from './identity'
import type { CountryId, Email, IsoDateTime, TimeZoneName, Uuid } from './shared'

// ---------------------------------------------------------------------------
// Connexion
// ---------------------------------------------------------------------------

export interface LoginPayload {
  email: string
  password: string
  /**
   * Session longue sur cet appareil. Détermine `identity.sessions.expires_at` :
   * quelques heures sans, plusieurs semaines avec.
   */
  remember_me: boolean
}

/**
 * Issue d'une tentative de connexion.
 *
 * `mfa_required` est PRÉVU, pas implémenté : `identity.accounts.mfa_enabled_at`
 * existe dans le modèle, l'écran matérialise l'emplacement de la saisie du code
 * et annonce que l'étape n'est pas encore branchée. Voir `AuthSecondFactor`.
 */
export type LoginResult =
  | { status: 'authenticated'; person: Person }
  | { status: 'mfa_required'; challenge_id: Uuid; method: 'totp' }
  | { status: 'invalid_credentials' }
  | { status: 'locked'; until: IsoDateTime }
  | { status: 'suspended'; until: IsoDateTime | null }
  | { status: 'email_unverified'; email: Email }

// ---------------------------------------------------------------------------
// Création de compte
// ---------------------------------------------------------------------------

/**
 * Le STRICT MINIMUM, et rien de plus : prénom, nom, adresse, pays, mot de passe.
 * Le rattachement à une organisation est une étape à part entière (prompt A2) —
 * `people.primary_organization_id` reste donc nul à la création.
 *
 * `preferred_locale` et `timezone` ne sont pas demandés : la langue est celle de
 * l'interface au moment de l'inscription, le fuseau celui que déclare le
 * navigateur. Deux champs de formulaire en moins, deux colonnes remplies quand
 * même (`people.preferred_locale`, `people.timezone`, toutes deux NOT NULL).
 */
export interface RegisterPayload {
  first_name: string
  last_name: string
  email: string
  country_id: CountryId
  password: string
  preferred_locale: string
  timezone: TimeZoneName
}

/**
 * Réponse à une création de compte. Une seule issue de succès, TOUJOURS la même,
 * que l'adresse soit libre ou déjà prise : dans le second cas, l'API envoie un
 * courriel de rappel de compte existant au lieu d'un lien de vérification, et
 * l'écran ne voit pas la différence.
 */
export interface RegisterResult {
  status: 'verification_sent'
  email: Email
}

// ---------------------------------------------------------------------------
// Jetons à usage unique — `identity.one_time_tokens`
// ---------------------------------------------------------------------------

/**
 * Pourquoi un jeton a été refusé. Le modèle ne distingue pas « inconnu » de
 * « déjà consommé » (`consumed_at`) ni de « périmé » (`expires_at`), mais
 * l'écran, lui, ne propose pas la même suite : un jeton expiré se redemande,
 * un jeton déjà consommé signifie que le travail est fait.
 */
export type TokenRejection = 'invalid' | 'expired' | 'already_used'

export type VerifyEmailResult =
  | { status: 'verified'; email: Email }
  | { status: 'rejected'; reason: TokenRejection }

/** Renvoi du lien de vérification. Issue unique : rien ne doit se déduire. */
export interface ResendVerificationResult {
  status: 'sent'
}

/** Demande de réinitialisation. Issue unique, adresse connue ou non. */
export interface PasswordResetRequestResult {
  status: 'sent'
}

/** Contrôle d'un jeton AVANT d'afficher le formulaire de nouveau mot de passe. */
export type TokenCheckResult =
  | { status: 'valid'; email: Email }
  | { status: 'rejected'; reason: TokenRejection }

export type PasswordResetResult =
  | { status: 'reset'; email: Email }
  | { status: 'rejected'; reason: TokenRejection }

// ---------------------------------------------------------------------------
// Fournisseurs d'identité — HORS PÉRIMÈTRE, et c'est une décision
//
// L'ENUM `identity.auth_provider` accepte `google`, `microsoft`, `linkedin` et
// `oidc` en plus de `password` : la base restera capable de porter une connexion
// fédérée le jour où elle se justifiera. **Les écrans, eux, n'en proposent
// aucune** — décision du commanditaire le 17/08.
//
// Ce qui a été pesé : se connecter par un annuaire d'entreprise suppose que
// l'ePavillon soit déclaré comme application autorisée chez chaque institution
// (identifiant client, secret, adresse de retour). Sur des centaines
// d'organisations francophones, la démarche ne passe pas à l'échelle. Et le
// bénéfice se confond, pour qui lit l'écran, avec le rattachement à une
// organisation — qui n'a RIEN à voir : le droit de déposer au nom d'une
// organisation vient de `org.memberships` et du rôle `org_member`, jamais du
// fournisseur d'authentification.
//
// Aucun type n'est donc déclaré ici. `AuthProvider` (types/identity.ts) reste la
// seule trace de ce que la base accepte.
// ---------------------------------------------------------------------------
