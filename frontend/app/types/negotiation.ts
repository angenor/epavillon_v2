/**
 * Contrats de l'ADMISSION — ce qui circule entre Guide Négo et le module
 * `negotiation`.
 *
 * Ces types ne décrivent aucune table : ils nomment les deux réponses que
 * l'application lit. L'état d'accès est **dérivé du RBAC** côté API, jamais
 * stocké dans une colonne ; le client le reçoit tout composé et n'en recalcule
 * rien.
 *
 * POURQUOI LES REFUS SONT DES RÉPONSES ET NON DES EXCEPTIONS. Sept des neuf
 * issues d'un code sont des refus prévus par le parcours : code inconnu,
 * révoqué, épuisé, terminé, pas encore ouvert, trop d'essais, ou une demande
 * ouverte parce que le mode l'exige. L'API les rend en 200 avec leur
 * discriminant, le compilateur oblige l'écran à les traiter toutes, et aucune ne
 * laisse l'écran sans suite (FR-015).
 *
 * LE MESSAGE VIENT DE L'API, ET S'AFFICHE TEL QUEL (FR-020). Elle seule connaît
 * la date de révocation et le temps d'attente restant. Les titres, les aides et
 * les boutons autour restent de l'i18n.
 *
 * RIEN ICI NE NOMME NI NE SUPPOSE UN GENRE (SC-006). L'appartenance à un réseau
 * vient du code utilisé, et de rien d'autre.
 */

import type { IsoDateTime, Uuid } from './shared'

/** Les trois modes d'admission, réglés en base et relus à chaque tentative. */
export type AdmissionMode = 'code' | 'approval' | 'code_and_approval'

/** Les cinq états que « Mon accès » sait dire. */
export type AccessState = 'visitor' | 'pending' | 'granted' | 'rejected' | 'revoked'

/**
 * Ce que l'accès ouvre : une COP nommée, ou Guide Négo en entier.
 *
 * `type: 'global'` porte `id` et `name` à `null` — il n'y a pas d'espace à
 * nommer, et l'annoncer comme une COP dirait moins que la vérité.
 */
export interface AccessScopeView {
  type: 'global' | 'negotiation_space'
  id: Uuid | null
  name: string | null
}

export interface GrantedAccess {
  scope: AccessScopeView
  granted_at: IsoDateTime
  /** Libellé du code par lequel l'accès est venu ; nul si un administrateur l'a attribué. */
  source_code_label: string | null
}

/**
 * Une appartenance de réseau. **Elle n'ouvre aucun droit** à cette étape : elle
 * servira le canal réservé et les chiffres demandés par les bailleurs.
 */
export interface NetworkView {
  code: string
  label: string
}

/** « Annulée » est le fait de la personne, entrée par un code entre-temps. */
export type AccessRequestStatus = 'pending' | 'approved' | 'rejected' | 'cancelled'

export interface AccessRequestView {
  id: Uuid
  status: AccessRequestStatus
  submitted_at: IsoDateTime
  decided_at: IsoDateTime | null
  decision_reason: string | null
}

/** `GET /api/negotiation/me/access` — un écran, une lecture. */
export interface AccessStateView {
  admission_mode: AdmissionMode
  state: AccessState
  granted: GrantedAccess | null
  networks: NetworkView[]
  request: AccessRequestView | null
  /** `negotiation.report.validate`, portée globale. */
  can_validate_reports: boolean
  /** Signalements en attente, toutes éditions ; `null` sans la permission. */
  reports_to_review: number | null
}

// ---------------------------------------------------------------------------
// Saisir un code
// ---------------------------------------------------------------------------

export interface RedeemPayload {
  code: string
  /**
   * Déclaré par le téléphone. **Information, jamais borne du compteur** : la
   * limite d'essais se compte par personne, tous appareils confondus.
   */
  device_id?: string
}

export type RedeemIssue =
  | 'accepted'
  | 'pending_approval'
  | 'already_granted'
  | 'unknown'
  | 'revoked'
  | 'exhausted'
  | 'expired'
  | 'not_yet_valid'
  | 'throttled'

export interface RedeemResult {
  issue: RedeemIssue
  /** Composé par l'API, affiché **tel quel**. */
  message: string
  revoked_at: IsoDateTime | null
  retry_after_seconds: number | null
  granted: GrantedAccess | null
  networks: NetworkView[]
  request: AccessRequestView | null
}

// ---------------------------------------------------------------------------
// Demander l'accès
// ---------------------------------------------------------------------------

/**
 * `POST /api/negotiation/access-requests` — ce que l'écran de demande envoie.
 *
 * `space_id` absent vaut une demande de portée globale : à cette étape l'espace
 * réservé n'en a qu'un, et l'écran n'a rien à choisir.
 *
 * **Une seule demande en attente par personne et par portée**, et c'est la base
 * qui le tient : deux appareils qui envoient ensemble ne produisent qu'une
 * ligne, et le second reçoit `NEGOTIATION_ACCESS_REQUEST_PENDING`.
 */
export interface CreateAccessRequestPayload {
  space_id?: Uuid | null
  /** Facultatif : il aide l'IFDD à reconnaître une délégation qu'elle attend. */
  message?: string | null
}

// ---------------------------------------------------------------------------
// Les thématiques suivies (0c)
// ---------------------------------------------------------------------------

/**
 * `GET` et `PUT /api/negotiation/me/themes` — ce que la personne suit.
 *
 * **Des codes, jamais de libellés.** L'empreinte (`ETag`) se calcule sur les
 * codes triés : deux appareils de la même personne, l'un en français, l'autre
 * en anglais, voient la même empreinte pour un même état. Les libellés viennent
 * de la route publique des termes de `negotiation_theme`, seule source.
 */
export interface FollowedTheme {
  code: string
  followed_at: IsoDateTime
}

export interface MyThemes {
  themes: FollowedTheme[]
}

/**
 * Le corps du `PUT` : la liste **entière** des codes suivis, jamais un delta.
 * Rejouer le même corps donne le même état — c'est ce qui rend sûre la file
 * d'écritures différées. `If-Match` porte l'empreinte de l'état sur lequel le
 * choix a été pris ; un écart sort en 412 et l'intention s'abandonne.
 */
export interface ThemesPayload {
  codes: string[]
}
