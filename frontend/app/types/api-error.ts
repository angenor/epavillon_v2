/**
 * Le contrat d'erreur de l'API, DÉRIVÉ du document engendré.
 *
 * Rien n'est recopié ici : les soixante-cinq codes stables vivent dans le
 * catalogue du noyau Rust (`backend/crates/kernel/src/error.rs`), d'où l'OpenAPI
 * les tire, et d'où `app/types/api.ts` les tient à son tour. Un code ajouté
 * là-bas apparaît ici au prochain `make openapi` ; un code recopié à la main
 * aurait cessé d'être vrai au premier ajout.
 *
 * LE MESSAGE EST FRANÇAIS ET AFFICHABLE TEL QUEL. C'est l'API qui le compose :
 * elle seule sait pourquoi elle refuse, et un second catalogue de messages côté
 * site donnerait deux textes pour un même refus. Le site ne traduit que ce que
 * l'API n'a pas pu dire — voir `utils/api-error.ts`.
 */
import type { components } from './api'

/** Le corps rendu par l'API sur un status d'erreur. */
export type ApiErrorBody = components['schemas']['ApiError']

/** Les codes stables du catalogue. Le site branche là-dessus, jamais sur le text. */
export type ApiErrorCode = ApiErrorBody['code']

/**
 * Les codes qui disent « la session ne vaut plus rien ».
 *
 * `UNAUTHENTICATED` et `IDENTITY_SESSION_EXPIRED` se rattrapent par une rotation
 * du jeton ; les deux autres non — une session close et un jeton rejoué exigent
 * une nouvelle connexion, et insister ferait rejouer la détection à chaque appel.
 */
export const RECOVERABLE_SESSION_CODES = [
  'UNAUTHENTICATED',
  'IDENTITY_SESSION_EXPIRED',
] as const satisfies readonly ApiErrorCode[]

export const LOST_SESSION_CODES = [
  'IDENTITY_SESSION_REVOKED',
  'IDENTITY_REFRESH_REUSED',
] as const satisfies readonly ApiErrorCode[]
