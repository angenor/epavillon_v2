/**
 * Données simulées de l'AUTHENTIFICATION — `identity.accounts` et
 * `identity.one_time_tokens` (`030_identity.sql` § 2), plus la logique que
 * l'API Rust portera au prompt B1.
 *
 * POURQUOI CE FICHIER N'EXISTAIT PAS EN A0.3. L'en-tête de `people.ts` le dit :
 * « les comptes ne sont pas simulés, ils ne portent que des secrets et des dates
 * de connexion ». C'était vrai tant qu'aucun écran ne se connectait. Le prompt
 * A1 a besoin de trois choses que `people` ne porte pas : un compte peut être
 * VERROUILLÉ après des tentatives (`locked_until`), il peut exiger un SECOND
 * FACTEUR (`mfa_enabled_at`), et un lien reçu par courriel doit pouvoir être
 * valide, périmé ou déjà utilisé (`one_time_tokens`).
 *
 * AUCUN SECRET ICI, ET C'EST STRUCTUREL. `Account` ne déclare ni `password_hash`
 * ni `mfa_secret_encrypted` (voir l'en-tête de `types/identity.ts`) : le mot de
 * passe de démonstration ci-dessous n'est PAS un champ de compte, c'est une
 * constante de ce fichier, comparée en clair parce qu'aucune vérification réelle
 * n'a lieu ici. En production, Argon2id, côté API, et rien de tout cela ne
 * franchit la frontière — `COMMENT ON COLUMN accounts.password_hash` : « aucune
 * fonction SQL ne doit vérifier de mot de passe ».
 *
 * LES JETONS SONT DATÉS RELATIVEMENT À MAINTENANT. Un jeton de démonstration
 * portant une date fixe serait périmé le lendemain de son écriture, et l'écran
 * de vérification n'aurait plus aucun cas passant à montrer. Seule exception au
 * principe « les mocks sont figés » — elle est ici la seule façon de garder les
 * trois issues (valide, périmé, déjà utilisé) toutes les trois vraies.
 */

import type { Account, OneTimeToken, TokenPurpose } from '~/types/identity'
import type {
  LoginPayload,
  LoginResult,
  PasswordResetResult,
  RegisterPayload,
  RegisterResult,
  TokenCheckResult,
  TokenRejection,
  VerifyEmailResult,
} from '~/types/auth'
import type { Email } from '~/types/shared'
import { ACCOUNT, ONE_TIME_TOKEN, PERSON } from './ids'
import { people } from './people'

// ---------------------------------------------------------------------------
// Le mot de passe de démonstration
// ---------------------------------------------------------------------------

/**
 * Mot de passe commun à tous les comptes simulés. Il satisfait les trois
 * exigences de la plateforme — huit signes au moins, une majuscule, une
 * minuscule : un jeu de démonstration qui ne passerait pas sa propre validation
 * serait un piège pour la session suivante.
 *
 * Il contient volontairement « epavillon », que `evaluatePassword()` signale
 * comme fragment trop commun : la démonstration montre ainsi un mot de passe
 * CONFORME et pourtant mal noté, qui est exactement le comportement voulu — les
 * exigences opposent, l'indicateur conseille.
 */
export const DEMO_PASSWORD = 'ePavillon2027!'

// ---------------------------------------------------------------------------
// Comptes — `identity.accounts`
// ---------------------------------------------------------------------------

/** Trois quarts d'heure dans le futur : un verrouillage encore actif. */
const inMinutes = (minutes: number): string =>
  new Date(Date.now() + minutes * 60_000).toISOString()

const inDays = (days: number): string => inMinutes(days * 24 * 60)

function passwordAccount(
  id: string,
  person_id: string,
  fields: Partial<Omit<Account, 'id' | 'person_id' | 'provider'>> = {},
): Account {
  return {
    id,
    person_id,
    provider: 'password',
    // `ck_accounts_password_shape` : nul pour un compte mot de passe, c'est
    // l'adresse qui identifie la personne chez le fournisseur « nous-mêmes ».
    provider_subject: null,
    password_changed_at: fields.password_changed_at ?? '2026-06-01T09:00:00Z',
    mfa_enabled_at: fields.mfa_enabled_at ?? null,
    last_login_at: fields.last_login_at ?? '2026-08-12T07:40:00Z',
    failed_attempts: fields.failed_attempts ?? 0,
    locked_until: fields.locked_until ?? null,
    created_at: fields.created_at ?? '2026-01-14T08:30:00Z',
    updated_at: fields.updated_at ?? '2026-08-12T07:40:00Z',
  }
}

/**
 * Un compte mot de passe par personne — `ux_accounts_password_per_person`.
 * Quatre d'entre eux portent un cas particulier ; ce sont eux qui donnent sa
 * valeur au jeu, les autres se connectent sans histoire.
 */
export const accounts = [
  // Cas ordinaire : administratrice globale, connexion sans incident.
  passwordAccount(ACCOUNT(1), PERSON.bakayoko),

  // SECOND FACTEUR ACTIVÉ. La connexion s'arrête après le mot de passe et
  // demande un code — étape dont le prompt A1 réserve l'emplacement sans
  // l'implémenter. C'est le seul compte du jeu dans ce cas.
  passwordAccount(ACCOUNT(2), PERSON.perretAdmin, {
    mfa_enabled_at: '2026-04-18T11:05:00Z',
  }),

  // COMPTE VERROUILLÉ après cinq échecs. Le verrou porte sur le COMPTE, pas sur
  // la personne : `people.status` reste `active`.
  passwordAccount(ACCOUNT(3), PERSON.ouedraogo, {
    failed_attempts: 5,
    locked_until: inMinutes(12),
  }),

  // Personne SUSPENDUE (`people.status = 'suspended'`) : le compte, lui, est
  // intact. Les deux se vérifient séparément, et dans cet ordre.
  passwordAccount(ACCOUNT(4), PERSON.lambert),

  // ADRESSE NON VÉRIFIÉE : le mot de passe est bon, la connexion s'arrête là et
  // propose de renvoyer le lien.
  passwordAccount(ACCOUNT(5), PERSON.nouvelInscrit, {
    password_changed_at: '2026-08-14T16:20:00Z',
    last_login_at: null,
    created_at: '2026-08-14T16:20:00Z',
    updated_at: '2026-08-14T16:20:00Z',
  }),

  // AUCUNE ORGANISATION, adresse vérifiée : le compte qui sert à éprouver
  // l'écran de rattachement (A2). Ajouté avec lui.
  passwordAccount(ACCOUNT(12), PERSON.sy, {
    password_changed_at: '2026-08-16T09:30:00Z',
    last_login_at: '2026-08-16T09:35:00Z',
    created_at: '2026-08-16T09:30:00Z',
    updated_at: '2026-08-16T09:35:00Z',
  }),

  // Le reste de l'équipe et des référents d'organisation, sans particularité.
  passwordAccount(ACCOUNT(6), PERSON.tremblay),
  passwordAccount(ACCOUNT(7), PERSON.nkoDiop),
  passwordAccount(ACCOUNT(8), PERSON.lemoine),
  passwordAccount(ACCOUNT(9), PERSON.sowFall),
  passwordAccount(ACCOUNT(10), PERSON.kabore),
  passwordAccount(ACCOUNT(11), PERSON.ilboudo),

  // Référent de Verdéo Solutions, ajouté au prompt A5. Son organisation est la
  // seule du jeu à porter les six états d'un dossier — brouillon, déposé,
  // CORRECTIONS DEMANDÉES, refusé deux fois, annulé — et elle n'avait aucun
  // compte : le cas central de l'espace organisation, « corriger un dossier que
  // le comité renvoie », n'était donc atteignable par personne.
  passwordAccount(ACCOUNT(13), PERSON.moreau),

  // LE RESPONSABLE D'UN SEUL ÉVÉNEMENT, ENFIN CONNECTABLE. Ajouté au prompt
  // A12 : Estelle Ngo Bassong administre le cycle PACO 2027 et rien d'autre —
  // c'est le cas que le commanditaire décrit et le seul qui prouve que le
  // back-office se partage sans se dupliquer. Claire Perret tenait déjà le
  // périmètre restreint, mais son compte exige un second facteur, que le
  // prompt A1 n'a pas implémenté : personne ne pouvait donc OUVRIR le
  // back-office en périmètre restreint, et le filtrage n'était éprouvable que
  // par lecture de code.
  passwordAccount(ACCOUNT(14), PERSON.ngoBassong, {
    password_changed_at: '2026-08-03T08:30:00Z',
    last_login_at: '2026-08-17T14:05:00Z',
    created_at: '2026-08-03T08:25:00Z',
    updated_at: '2026-08-17T14:05:00Z',
  }),
] satisfies Account[]

// ---------------------------------------------------------------------------
// Jetons à usage unique — `identity.one_time_tokens`
// ---------------------------------------------------------------------------

/**
 * Un jeton tel qu'il circule : la chaîne du lien reçu par courriel, et la ligne
 * qu'elle désigne. La base ne stocke que `token_hash` — la chaîne en clair
 * n'existe QUE dans le courriel et dans ce fichier de démonstration.
 *
 * Les valeurs sont lisibles à dessein : `/verification-adresse?token=verification-expiree`
 * se compose à la main pour montrer un cas d'erreur sans attendre qu'un jeton
 * périme réellement.
 */
export interface DemoToken {
  /** La chaîne portée par l'URL du courriel. */
  token: string
  record: OneTimeToken
}

function demoToken(
  token: string,
  n: number,
  purpose: TokenPurpose,
  person_id: string,
  state: 'valid' | 'expired' | 'used',
): DemoToken {
  return {
    token,
    record: {
      id: ONE_TIME_TOKEN(n),
      person_id,
      purpose,
      // Le modèle laisse `payload` libre ; l'API y range de quoi composer le
      // courriel sans relire la personne. On s'en tient à l'adresse visée.
      payload: { email: personEmail(person_id) },
      expires_at: state === 'expired' ? inMinutes(-90) : inDays(2),
      consumed_at: state === 'used' ? inMinutes(-45) : null,
      created_at: inDays(-1),
    },
  }
}

function personEmail(personId: string): string {
  return people.find((p) => p.id === personId)?.primary_email ?? ''
}

export const oneTimeTokens: DemoToken[] = [
  // Vérification d'adresse — les trois issues de l'écran.
  demoToken('verification-valide', 1, 'email_verification', PERSON.nouvelInscrit, 'valid'),
  demoToken('verification-expiree', 2, 'email_verification', PERSON.nouvelInscrit, 'expired'),
  demoToken('verification-utilisee', 3, 'email_verification', PERSON.ilboudo, 'used'),

  // Réinitialisation de mot de passe — les mêmes trois issues.
  demoToken('reinitialisation-valide', 4, 'password_reset', PERSON.ouedraogo, 'valid'),
  demoToken('reinitialisation-expiree', 5, 'password_reset', PERSON.ouedraogo, 'expired'),
  demoToken('reinitialisation-utilisee', 6, 'password_reset', PERSON.kabore, 'used'),
]

/**
 * Résout un jeton pour une finalité donnée. L'ordre des refus n'est pas
 * indifférent : « déjà utilisé » se dit AVANT « périmé », parce qu'un jeton
 * consommé puis périmé raconte quelque chose de vrai — le travail est fait — là
 * où « le lien a expiré » enverrait redemander un courriel inutile.
 */
function resolveToken(
  token: string,
  purpose: TokenPurpose,
): { ok: true; entry: DemoToken } | { ok: false; reason: TokenRejection } {
  const entry = oneTimeTokens.find((t) => t.token === token && t.record.purpose === purpose)
  if (!entry) return { ok: false, reason: 'invalid' }
  if (entry.record.consumed_at !== null) return { ok: false, reason: 'already_used' }
  if (Date.parse(entry.record.expires_at) <= Date.now()) return { ok: false, reason: 'expired' }
  return { ok: true, entry }
}

// ---------------------------------------------------------------------------
// Ce que fera l'API — prompt B1
// ---------------------------------------------------------------------------

/**
 * Tentative de connexion.
 *
 * L'ORDRE DES CONTRÔLES EST LA RÈGLE DE DISCRÉTION ELLE-MÊME. Le mot de passe
 * est vérifié EN PREMIER : tant qu'il n'est pas juste, la seule réponse possible
 * est `invalid_credentials`, adresse inconnue ou non. Un compte verrouillé, une
 * personne suspendue, une adresse non vérifiée ne se disent qu'ENSUITE — à
 * quelqu'un qui vient de prouver son identité, et à qui l'on ne révèle donc
 * rien qu'il ne sache déjà.
 *
 * C'est aussi pour cela que le verrouillage ne se signale pas avant la
 * vérification : l'annoncer d'emblée transformerait le formulaire en oracle
 * d'existence de comptes.
 */
export function authenticate(payload: LoginPayload): LoginResult {
  const email = payload.email.trim().toLowerCase()
  // `platform.email` est un `citext` : la comparaison ignore la casse.
  const person = people.find((p) => p.primary_email.toLowerCase() === email)
  const account = person ? accounts.find((a) => a.person_id === person.id) : undefined

  if (!person || !account || payload.password !== DEMO_PASSWORD) {
    return { status: 'invalid_credentials' }
  }

  if (account.locked_until !== null && Date.parse(account.locked_until) > Date.now()) {
    return { status: 'locked', until: account.locked_until }
  }

  if (person.status === 'suspended' || person.status === 'blocked') {
    return { status: 'suspended', until: person.suspended_until }
  }

  if (person.email_verified_at === null) {
    return { status: 'email_unverified', email: person.primary_email }
  }

  if (account.mfa_enabled_at !== null) {
    // Le défi n'est pas simulé plus avant : l'écran s'arrête ici et le dit.
    return { status: 'mfa_required', challenge_id: account.id, method: 'totp' }
  }

  return { status: 'authenticated', person }
}

/**
 * Création de compte. RÉPONSE INVARIABLE, que l'adresse soit libre ou déjà
 * connue : c'est le courriel envoyé qui diffère — lien de vérification dans un
 * cas, rappel « vous avez déjà un compte » dans l'autre. Le formulaire, lui, ne
 * doit rien laisser déduire.
 *
 * Rien n'est écrit : les données simulées sont en lecture seule, et un compte
 * créé ne survivrait pas au rechargement de la page. La personne reste donc
 * introuvable ensuite, ce qui est cohérent — le compte n'est utilisable qu'une
 * fois l'adresse vérifiée, et la vérification passe par un lien que ce jalon
 * n'envoie pas.
 */
export function registerPerson(payload: RegisterPayload): RegisterResult {
  return { status: 'verification_sent', email: payload.email.trim() as Email }
}

/** Vérification d'une adresse depuis le lien reçu par courriel. */
export function verifyEmailToken(token: string): VerifyEmailResult {
  const result = resolveToken(token, 'email_verification')
  if (!result.ok) return { status: 'rejected', reason: result.reason }
  return { status: 'verified', email: personEmail(result.entry.record.person_id ?? '') as Email }
}

/** Contrôle du jeton AVANT d'afficher le formulaire de nouveau mot de passe. */
export function checkPasswordResetToken(token: string): TokenCheckResult {
  const result = resolveToken(token, 'password_reset')
  if (!result.ok) return { status: 'rejected', reason: result.reason }
  return { status: 'valid', email: personEmail(result.entry.record.person_id ?? '') as Email }
}

/**
 * Enregistrement du nouveau mot de passe. Le jeton est revérifié : entre
 * l'affichage du formulaire et son envoi, il a pu périmer — un formulaire ouvert
 * la veille au soir et validé le lendemain matin est un cas ordinaire.
 */
export function resetPassword(token: string): PasswordResetResult {
  const result = resolveToken(token, 'password_reset')
  if (!result.ok) return { status: 'rejected', reason: result.reason }
  return { status: 'reset', email: personEmail(result.entry.record.person_id ?? '') as Email }
}
