/**
 * Les deux façons dont un appel à l'API peut échouer, et rien d'autre.
 *
 *   · L'API A RÉPONDU, ET ELLE REFUSE — `ApiRequestError`. Elle porte son code
 *     stable, son message français composé par l'API, le champ fautif s'il y en
 *     a un, et la référence d'incident à citer. Le message s'affiche TEL QUEL :
 *     il vient du catalogue du noyau, et le réécrire côté site donnerait deux
 *     textes pour un même refus.
 *
 *   · L'API N'A PAS RÉPONDU — `ApiUnreachableError`. Panne réseau, service
 *     arrêté, passerelle qui rend du HTML, délai dépassé. Là, et là seulement,
 *     le text est du ressort du site : l'API n'a rien pu dire. Il vient des
 *     fichiers de traduction, jamais d'une chaîne écrite ici.
 *
 * La distinction n'est pas cosmétique : la première se raconte à la personne
 * (« cette adresse est déjà utilisée »), la seconde commande le mode dégradé.
 */
import type { ApiErrorBody, ApiErrorCode } from '~/types/api-error'
import { LOST_SESSION_CODES, RECOVERABLE_SESSION_CODES } from '~/types/api-error'

/** Les motifs d'injoignabilité, traduits par le site sous la clé `api.unreachable.<motif>`. */
export type ApiUnreachableReason = 'network' | 'timeout' | 'malformed' | 'gateway'

/**
 * Accès refusé — l'écran affiche son état « accès refusé », pas un message.
 *
 * DEUX ORIGINES, une seule conséquence. Le site la lève quand une édition sort
 * du périmètre d'administration (`assertEventInScope`), et le client la lève sur
 * un `FORBIDDEN` de l'API. Les autres refus en 403 — « seul un référent peut
 * faire cela », « ce dossier ne vous est pas confié » — restent des
 * `ApiRequestError` : ils nomment une raison précise, qu'un écran « accès
 * refusé » ferait disparaître.
 */
export class ForbiddenError extends Error {
  /**
   * **La seule propriété qui survit au rendu serveur.**
   *
   * `useAsyncData` ne transmet pas l'erreur levée : h3 la réemballe, et son
   * `toJSON()` ne conserve que `statusCode`, `statusMessage` et `data`. Le nom
   * de la classe, lui, est perdu — un écran qui teste `error.name ===
   * 'ForbiddenError'` après hydratation ne verra jamais rien, et affichera une
   * panne à la place de son état « accès refusé ».
   */
  readonly statusCode = 403

  constructor(message = "Cette édition n'est pas dans votre périmètre d'administration.") {
    super(message)
    this.name = 'ForbiddenError'
  }
}

/** Un refus de l'API : elle a répondu, elle dit pourquoi. */
export class ApiRequestError extends Error {
  readonly code: ApiErrorCode
  readonly status: number
  readonly field: string | null
  readonly requestId: string | null

  constructor(body: ApiErrorBody, status: number) {
    super(body.message)
    this.name = 'ApiRequestError'
    this.code = body.code
    this.status = status
    this.field = body.field ?? null
    this.requestId = body.request_id ?? null
  }
}

/** L'API n'a pas répondu. Aucun message d'API à afficher : le site parle seul. */
export class ApiUnreachableError extends Error {
  readonly reason: ApiUnreachableReason
  readonly status: number | null

  constructor(reason: ApiUnreachableReason, status: number | null, cause?: unknown) {
    super(`API injoignable (${reason})`)
    this.name = 'ApiUnreachableError'
    this.reason = reason
    this.status = status
    this.cause = cause
  }
}

/** Un corps d'erreur de l'API, ou rien. Un `unknown` venu du réseau ne se croit pas sur parole. */
function apiErrorBodyOf(data: unknown): ApiErrorBody | null {
  if (typeof data !== 'object' || data === null) return null
  const candidate = data as Record<string, unknown>
  if (typeof candidate.code !== 'string' || typeof candidate.message !== 'string') return null
  return {
    code: candidate.code as ApiErrorCode,
    message: candidate.message,
    ...(typeof candidate.field === 'string' ? { field: candidate.field } : {}),
    ...(typeof candidate.request_id === 'string' ? { request_id: candidate.request_id } : {}),
  }
}

/**
 * Traduit ce que `$fetch` a levé en l'une des deux erreurs ci-dessus.
 *
 * UN STATUT SANS CORPS D'API N'EST PAS UN REFUS D'API : c'est un 502 de
 * passerelle, un 404 de serveur mal configuré, une page d'erreur en HTML. Le
 * traiter comme un refus ferait afficher « La ressource demandée est
 * introuvable » alors que c'est l'API entière qui manque — et personne ne
 * saurait qu'il faut la démarrer.
 */
export function normalizeApiError(error: unknown): ApiRequestError | ApiUnreachableError {
  if (error instanceof ApiRequestError || error instanceof ApiUnreachableError) return error

  const raw = error as { status?: number; statusCode?: number; data?: unknown; name?: string; message?: string }
  const status = raw?.status ?? raw?.statusCode ?? null

  if (status !== null) {
    const body = apiErrorBodyOf(raw.data)
    if (body) return new ApiRequestError(body, status)
    // Un 5xx sans corps d'API vient d'un intermédiaire, pas de l'API.
    if (status >= 500) return new ApiUnreachableError('gateway', status, error)
    return new ApiUnreachableError('malformed', status, error)
  }

  const text = `${raw?.name ?? ''} ${raw?.message ?? ''}`.toLowerCase()
  if (text.includes('timeout') || text.includes('aborted')) {
    return new ApiUnreachableError('timeout', null, error)
  }
  return new ApiUnreachableError('network', null, error)
}

/**
 * Est-ce un refus d'accès — **y compris après le rendu serveur** ?
 *
 * Un écran ne peut PAS tester `instanceof ForbiddenError` sur ce que rend
 * `useAsyncData` : au rendu serveur, l'erreur traverse une sérialisation qui ne
 * garde ni la classe ni le nom. Seul `statusCode` survit. Ce test couvre les
 * trois formes que l'erreur peut prendre selon l'endroit d'où on la regarde.
 */
export function isForbiddenError(error: unknown): boolean {
  if (error instanceof ForbiddenError) return true
  if (typeof error !== 'object' || error === null) return false
  const candidate = error as { name?: unknown; statusCode?: unknown }
  return candidate.name === 'ForbiddenError' || candidate.statusCode === 403
}

/** La session est-elle perdue au point qu'il faille se reconnecter ? */
export function isSessionLost(error: unknown): boolean {
  return (
    error instanceof ApiRequestError &&
    (LOST_SESSION_CODES as readonly string[]).includes(error.code)
  )
}

/** Une rotation du jeton peut-elle rattraper ce refus ? */
export function isSessionRecoverable(error: unknown): boolean {
  return (
    error instanceof ApiRequestError &&
    (RECOVERABLE_SESSION_CODES as readonly string[]).includes(error.code)
  )
}

/**
 * Le text à montrer, en français.
 *
 * Deux sources, et une seule règle pour choisir : **si l'API a parlé, c'est elle
 * qui a reason**. Elle seule connaît le motif exact du refus, et son catalogue
 * est déjà français. Le site ne prend la parole que lorsqu'elle s'est tue.
 *
 * `translate` est la fonction de traduction de l'écran appelant : ce fichier ne
 * peut pas appeler `useI18n()`, il n'est pas dans un `setup`.
 */
export function apiErrorMessage(
  error: unknown,
  translate: (cle: string) => string,
): string {
  const normalized = normalizeApiError(error)
  if (normalized instanceof ApiRequestError) return normalized.message
  return translate(`api.unreachable.${normalized.reason}`)
}

/**
 * Un échec de chargement, **réduit à ce qui se sérialise**.
 *
 * UNE INSTANCE DE CLASSE NE TRAVERSE PAS LE RENDU SERVEUR. Le payload de Nuxt
 * est composé par `devalue`, qui refuse tout ce qui n'est pas un objet simple :
 * une `ApiUnreachableError` posée dans l'état d'un store fait échouer la
 * sérialisation de la page ENTIÈRE, et le visiteur reçoit un 500 au lieu de
 * l'écran dégradé qu'on avait écrit pour lui. Le défaut est d'autant plus
 * vicieux qu'il n'apparaît que lorsque l'API est en panne — c'est-à-dire au
 * seul moment où ce chemin sert.
 *
 * Les stores retiennent donc ceci, et jamais l'erreur elle-même.
 */
export interface LoadFailure {
  /** Message français composé par l'API. Nul quand elle n'a pas répondu. */
  message: string | null
  /** Code stable de l'API, ou motif d'injoignabilité. */
  code: string
  /** L'API a-t-elle répondu ? Faux = panne de liaison, pas refus. */
  answered: boolean
  requestId: string | null
}

export function toLoadFailure(error: unknown): LoadFailure {
  const normalized = normalizeApiError(error)
  if (normalized instanceof ApiRequestError) {
    return {
      message: normalized.message,
      code: normalized.code,
      answered: true,
      requestId: normalized.requestId,
    }
  }
  return { message: null, code: normalized.reason, answered: false, requestId: null }
}

/** Le texte à montrer pour un échec retenu par un store. */
export function loadFailureMessage(
  failure: LoadFailure,
  translate: (cle: string) => string,
): string {
  return failure.message ?? translate(`api.unreachable.${failure.code}`)
}

/** La référence d'incident, quand l'API en a donné une. */
export function incidentReference(error: unknown): string | null {
  return error instanceof ApiRequestError ? error.requestId : null
}
