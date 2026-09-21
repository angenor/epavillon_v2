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
