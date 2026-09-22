/**
 * Contrats du BACK-OFFICE de l'admission — les codes d'invitation, leurs
 * usages, la file des demandes et le mode d'admission.
 *
 * Ces écrans appartiennent au back-office de l'ePavillon : ils en gardent les
 * composants et les jetons, et n'empruntent rien à Guide Négo (FR-045). Le
 * fichier n'est donc pas ré-exporté par `index.ts`, comme les autres contrats
 * d'écran.
 *
 * LE CODE EST RENDU EN CLAIR, ET C'EST VOULU. Il circule sur WhatsApp, recopié
 * à la main par tout un réseau : c'est un secret **partagé**, pas un secret
 * nominatif (FR-037). Le masquer dans la liste empêcherait un administrateur de
 * répondre à la seule question qu'on lui pose — « quel est le code en cours ? ».
 *
 * RÉVOQUER N'EST PAS RETIRER (ADR-006). Révoquer un code ferme la porte sans
 * toucher aux accès déjà accordés ; retirer un accès sort une personne sans
 * invalider le code du groupe. Deux gestes, deux routes, deux écrans — et
 * `granted_uses` est ce qui permet de dire à l'administrateur ce qu'il lui
 * resterait à faire.
 */

import type { IsoDateTime, Uuid } from './shared'
import type { AccessRequestStatus, AccessScopeView, AdmissionMode } from './negotiation'

/** Les cinq états d'un code, **dérivés par la vue** et jamais stockés. */
export type InvitationCodeState =
  | 'active'
  | 'revoked'
  | 'expired'
  | 'not_yet_valid'
  | 'exhausted'

/** Un réseau de négociation, lu dans la taxonomie — jamais dans l'i18n. */
export interface NetworkTermView {
  id: Uuid
  code: string
  label: string
}

/**
 * Un réseau et **le nombre de personnes qui en font partie** (SC-007).
 *
 * Ce compte ne se déduit pas des usages des codes : une même personne peut
 * entrer avec deux codes, et l'appartenance survit à la révocation du code qui
 * l'a apportée.
 */
export interface NetworkSummary extends NetworkTermView {
  /** Appartenances vivantes, personnes distinctes. */
  members_count: number
}

export interface SpaceOption {
  id: Uuid
  name: string
}

/** Une ligne de la liste des codes. */
export interface InvitationCodeRow {
  id: Uuid
  /** En clair : c'est un code partagé, pas un secret nominatif. */
  code: string
  label: string
  state: InvitationCodeState
  scope: AccessScopeView
  network: NetworkTermView | null
  used_count: number
  /** Nul : sans limite d'entrées. */
  max_uses: number | null
  valid_from: IsoDateTime
  valid_until: IsoDateTime | null
  revoked_at: IsoDateTime | null
  revoked_reason: string | null
  revoked_by_name: string | null
  created_at: IsoDateTime
  created_by_name: string | null
}

/**
 * `GET /api/admin/negotiation/invitation-codes` — l'écran en une réponse.
 *
 * `spaces` et `networks` servent **le filtre et le formulaire de création** :
 * le même appel les alimente, si bien que l'un ne peut pas proposer une portée
 * que l'autre ignore.
 */
export interface InvitationCodeListScreen {
  rows: InvitationCodeRow[]
  total: number
  spaces: SpaceOption[]
  networks: NetworkSummary[]
}

/**
 * `GET /api/admin/negotiation/invitation-codes/{id}` — la fiche.
 *
 * `granted_uses` compte les accès **encore ouverts** : c'est ce qu'il faut lire
 * avant de révoquer, puisque la révocation n'en retire aucun.
 */
export interface InvitationCodeDetail extends InvitationCodeRow {
  granted_uses: number
}

/** Qui est entré avec ce code, et si son accès tient encore. */
export interface InvitationCodeUseRow {
  person_id: Uuid
  display_name: string
  email: string
  used_at: IsoDateTime
  /** Lu dans le RBAC par la vue : la table des usages n'en porte aucune copie. */
  access_active: boolean
  access_revoked_at: IsoDateTime | null
  access_revoked_reason: string | null
}

/** `GET /api/admin/negotiation/invitation-codes/{id}/uses` */
export interface InvitationCodeUsesScreen {
  rows: InvitationCodeUseRow[]
  total: number
  granted_uses: number
}

/** La portée d'un code : une COP précise, ou Guide Négo en entier. */
export type InvitationCodeScopePayload =
  | { type: 'global' }
  | { type: 'negotiation_space'; id: Uuid }

/**
 * `POST /api/admin/negotiation/invitation-codes`
 *
 * **Le code n'y figure pas** : il est engendré par l'API. Laisser un
 * administrateur le choisir produirait des codes devinables — « COP31 »,
 * « IFDD2026 » — sur une porte que rien d'autre ne protège.
 */
export interface CreateInvitationCodePayload {
  label: string
  scope: InvitationCodeScopePayload
  /** Code de taxonomie du réseau, par exemple `women_negotiators`. */
  grants_network?: string | null
  /** Nul : sans limite d'entrées. */
  max_uses?: number | null
  valid_from?: IsoDateTime | null
  valid_until?: IsoDateTime | null
}

/** Le motif d'une révocation ou d'un retrait — toujours facultatif. */
export interface ReasonPayload {
  reason?: string | null
}

/** Ce que rendent les deux routes de retrait d'accès. */
export interface RevokeAllAccessResult {
  /** Accès qui viennent réellement de tomber : ceux déjà retirés ne comptent pas. */
  revoked: number
}

// ---------------------------------------------------------------------------
// La file des demandes
// ---------------------------------------------------------------------------

export interface AccessRequestRow {
  id: Uuid
  status: AccessRequestStatus
  person_id: Uuid
  display_name: string
  email: string
  /** Résolu dans la langue de l'écran : c'est ce qui fait reconnaître une délégation. */
  country: string | null
  scope: AccessScopeView
  /** Le code reconnu mais insuffisant à ouvrir, en mode « les deux ». */
  invitation_code: string | null
  invitation_code_label: string | null
  message: string | null
  submitted_at: IsoDateTime
  decided_at: IsoDateTime | null
  decided_by_name: string | null
  decision_reason: string | null
}

/**
 * `GET /api/admin/negotiation/access-requests`
 *
 * `pending` compte les demandes en attente **tous filtres confondus** : c'est
 * la pastille du menu, et elle ne doit pas tomber à zéro parce qu'on regarde
 * les refusées.
 */
export interface AccessRequestQueue {
  rows: AccessRequestRow[]
  total: number
  pending: number
}

/**
 * Le motif d'une décision, facultatif. Repris **tel quel** dans le courriel de
 * refus : c'est la seule chose que la personne lira pour comprendre.
 */
export interface DecideAccessRequestPayload {
  reason?: string | null
}

// ---------------------------------------------------------------------------
// Le mode d'admission
// ---------------------------------------------------------------------------

/**
 * Ce qu'un mode produit **pour la personne qui entre**, en faits et non en
 * phrases : l'écran compose son texte par ses fichiers de traduction, comme
 * tout écran du site.
 */
export interface AdmissionModeOption {
  mode: AdmissionMode
  /** La saisie d'un code est-elle proposée ? Fausse en « approbation seule ». */
  offers_code: boolean
  /** Un code juste ouvre-t-il aussitôt ? */
  code_opens: boolean
  /** Un administrateur doit-il trancher ? */
  needs_approval: boolean
}

/** `GET /api/admin/negotiation/admission` */
export interface AdmissionSettings {
  mode: AdmissionMode
  options: AdmissionModeOption[]
}

/**
 * `PUT /api/admin/negotiation/admission`
 *
 * **Prend effet à la tentative suivante**, sans mise en ligne : la valeur est
 * relue à chaque saisie de code, sans cache.
 */
export interface UpdateAdmissionModePayload {
  mode: AdmissionMode
}
