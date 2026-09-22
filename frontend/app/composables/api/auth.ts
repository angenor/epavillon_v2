/**
 * AUTHENTIFICATION (A1) — sa part de `useApi()`.
 *
 * Sorti de `useApi.ts` le 22/09 pour tenir le fichier commun sous le garde-fou
 * de mille lignes, comme `api/invitation.ts` avant lui. Les neuf méthodes sont
 * celles d'avant, signatures et comportements inchangés ; les écrans les
 * appellent toujours par `useApi().auth`.
 *
 * Les cinq écrans d'authentification passent par ici, et par rien d'autre.
 * Deux règles s'y jouent, l'une et l'autre invisibles depuis les pages :
 *
 *  · DISCRÉTION — `register` et `requestPasswordReset` rendent TOUJOURS la
 *    même réponse, adresse connue ou non. Rien dans le contrat ne permet
 *    d'écrire un écran bavard, même par inadvertance.
 *  · SESSION — l'API pose deux cookies `HttpOnly` que le navigateur renvoie
 *    seuls (`credentials: 'include'`, voir `api/http.ts`). `GET /auth/me`
 *    N'ACCEPTE AUCUN IDENTIFIANT : c'est la session qui dit qui parle. Le
 *    paramètre de `session()` ne sert donc qu'aux données simulées, qui
 *    n'ont pas de session à consulter — il n'est jamais envoyé.
 *
 * Le bloc de Guide Négo le réemprunte tel quel, pour y joindre son objet
 * `client` en un seul endroit au lieu de le confier à chaque écran.
 */

import type {
  AuthenticatedPerson,
  LoginPayload,
  LoginResult,
  PasswordResetRequestResult,
  PasswordResetResult,
  RegisterPayload,
  RegisterResult,
  ResendVerificationResult,
  SessionClient,
  TokenCheckResult,
  VerifyEmailResult,
} from '~/types/auth'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export function createAuthApi({ call, send }: ApiTransport) {
  return {
    login: (payload: LoginPayload): Promise<LoginResult> =>
      send('/auth/login', payload, (m) => m.authenticate(payload)),

    logout: (): Promise<{ status: 'signed_out' }> =>
      send('/auth/logout', {}, () => ({ status: 'signed_out' as const })),

    /**
     * Personne connectée, ou `null` si la session n'existe plus.
     *
     * `GET /auth/me` ne rend JAMAIS 401 — le site l'appelle déconnecté à
     * chaque navigation, et un statut d'erreur y ferait afficher un écran en
     * panne au lieu d'un état déconnecté. L'identifiant reçu ici ne part pas
     * dans la requête : il ne sert qu'à retrouver la personne dans les mocks.
     *
     * La réponse porte aussi **la session courante** — de quel appareil, depuis
     * quand —, ce qui évite une requête de plus au profil de l'application.
     */
    session: (personId: Uuid | null): Promise<AuthenticatedPerson | null> =>
      call('/auth/me', (m) =>
        personId === null ? null : (m.people.find((p) => p.id === personId) ?? null),
      ),

    register: (payload: RegisterPayload): Promise<RegisterResult> =>
      send('/auth/register', payload, (m) => m.registerPerson(payload)),

    /** Vérification de l'adresse depuis le lien reçu par courriel. */
    verifyEmail: (token: string): Promise<VerifyEmailResult> =>
      send('/auth/verify-email', { token }, (m) => m.verifyEmailToken(token)),

    /** Renvoi du lien de vérification. Réponse invariable. */
    resendVerification: (
      email: string,
      client?: SessionClient,
    ): Promise<ResendVerificationResult> =>
      send('/auth/verify-email/resend', { email, client }, () => ({ status: 'sent' as const })),

    /** Demande de réinitialisation. Réponse invariable, compte existant ou non. */
    requestPasswordReset: (
      email: string,
      client?: SessionClient,
    ): Promise<PasswordResetRequestResult> =>
      send('/auth/password-reset', { email, client }, () => ({ status: 'sent' as const })),

    /** Contrôle du jeton AVANT d'afficher le formulaire de nouveau mot de passe. */
    checkPasswordResetToken: (token: string): Promise<TokenCheckResult> =>
      call('/auth/password-reset/check', (m) => m.checkPasswordResetToken(token), { token }),

    resetPassword: (token: string, password: string): Promise<PasswordResetResult> =>
      send('/auth/password-reset/confirm', { token, password }, (m) => m.resetPassword(token)),
  }
}
