/**
 * Contrats de l'ESPACE ORGANISATION (A5) — requêtes, réponses et compositions.
 *
 * Ce fichier ne décrit AUCUNE table : il décrit ce que l'écran demande et ce
 * qu'on lui répond. Les tables vivent dans `types/org.ts`,
 * `types/programme/*.ts` et `types/engagement.ts`, et rien ici ne les recopie.
 *
 * POURQUOI DES COMPOSITIONS PLUTÔT QUE DES LECTURES SÉPARÉES. L'écran répond à
 * deux questions — « où en est chacun de mes dossiers ? » et « qu'est-ce qui
 * attend une action de ma part ? » — et aucune des deux ne se lit dans une
 * table. La première demande le dossier, son édition, son journal de transitions
 * et le compte de ses demandes de correction ouvertes ; la seconde demande de
 * balayer tous les dossiers plus la file d'adhésions. Composées écran par écran,
 * ces jointures deviennent un N+1 et divergent dès le deuxième écran qui les
 * refait. Elles sont donc DÉCLARÉES ICI et composées à un seul endroit
 * (`mocks/organization-workspace.ts` aujourd'hui, l'API demain).
 *
 * IL N'EXISTE PAS DE VUE « MES PROPOSITIONS » DANS LE MODÈLE — c'est l'écart
 * n° 8 relevé en dérivant les types, et il reste ouvert. `v_proposal_dashboard`
 * répond au COMITÉ (notes, rang, revues manquantes) et ne doit rien montrer de
 * tout cela au soumissionnaire.
 */

import type { EventEdition } from './event/edition'
import type { CallForProposals } from './event/call'
import type { Room } from './event/venue'
import type { Membership, Organization } from './org'
import type { Person } from './identity'
import type { TokenRejection } from './auth'
import type {
  Proposal,
  ProposalComment,
  ProposalHistoryEntry,
  ProposalTransition,
} from './programme/proposal'
import type { Session } from './programme/session'
import type {
  NotificationChannel,
  OffsetMinutes,
  ReminderStatus,
} from './engagement'
import type { Email, IsoDateTime, OrganizationId, ProposalId, Uuid } from './shared'

// ---------------------------------------------------------------------------
// Rappels — la lecture agrégée que l'écran demande
// ---------------------------------------------------------------------------

/**
 * UN RAPPEL DU CALENDRIER, tel que l'organisation le lit : un décalage, une
 * date, un état, un nombre de destinataires.
 *
 * CE N'EST PAS UNE LIGNE DE `scheduled_reminders`, et la nuance est le cœur du
 * sujet. La base matérialise une ligne PAR DESTINATAIRE et par décalage : une
 * séance à quarante inscrits en compte cent soixante. L'organisation n'a pas à
 * connaître qui reçoit quoi — ce sont des données personnelles d'inscrits, pas
 * les siennes —, elle a besoin de savoir que le rappel de J-2 est parti et à
 * combien de personnes. D'où cette agrégation par (séance, décalage, canal).
 *
 * Aucune fonction du modèle ne la rend aujourd'hui : obligation d'API inscrite
 * au prompt B6.
 */
export interface ReminderSlot {
  offset_before: OffsetMinutes
  channel: NotificationChannel
  /** Instant d'envoi — `starts_at − offset_before`, dans le fuseau de l'édition. */
  scheduled_for: IsoDateTime
  /** État consolidé des lignes du groupe : `sent` dès qu'elles sont parties. */
  status: ReminderStatus
  /** Destinataires du groupe. Un nombre, jamais une liste nominative. */
  recipient_count: number
  sent_at: IsoDateTime | null
}

// ---------------------------------------------------------------------------
// Les sessions d'un dossier retenu
// ---------------------------------------------------------------------------

/**
 * Une séance programmée, vue par l'organisation qui l'anime : son créneau, sa
 * salle, le nombre d'inscrits et le calendrier de ses rappels.
 *
 * LE NOMBRE, PAS LA LISTE. `registrations.forSession()` existe déjà pour le
 * back-office et exige un périmètre d'administration ; une organisation n'en a
 * pas et n'a pas à voir l'identité des inscrits.
 */
export interface TrackedSession {
  session: Session
  room: Room | null
  /** Inscriptions confirmées — ni annulées, ni en liste d'attente. */
  registered_count: number
  waitlisted_count: number
  capacity: number | null
  /** Les quatre rappels de la règle applicable, envoyés ou à venir. */
  reminders: ReminderSlot[]
}

// ---------------------------------------------------------------------------
// Le suivi d'un dossier
// ---------------------------------------------------------------------------

/**
 * UN DOSSIER ET SON AVANCEMENT — la brique de la liste comme de la fiche.
 *
 * `transitions` est le JOURNAL RÉEL (`programme.proposal_transitions`), et c'est
 * de lui que la frise se compose. Le graphe des états vit dans la base
 * (`proposal_transitions_allowed`) : le réimplémenter dans un composant, c'est
 * se garantir de diverger au premier changement de règle.
 */
export interface ProposalTracking {
  proposal: Proposal
  /** L'édition visée. Une organisation fidèle en a plusieurs, et la liste les
   *  groupe : un dossier de la COP30 ne se lit pas comme un dossier en cours. */
  edition: EventEdition
  transitions: ProposalTransition[]
  /** Demandes de correction encore ouvertes — c'est LE nombre que l'écran crie. */
  open_change_requests: number
  /** Séances programmées. Vide tant que le dossier n'est pas retenu. */
  sessions: TrackedSession[]
}

/** Le détail d'un dossier : son suivi, ses échanges, son historique. */
export interface ProposalFile {
  tracking: ProposalTracking
  /** Fil partagé avec le soumissionnaire. Les notes du comité n'y sont jamais. */
  comments: ProposalComment[]
  /** Auteurs des messages, pour ne pas résoudre les noms un par un. */
  participants: Person[]
  /** `programme.proposal_history()` — champ par champ, auteur et date. */
  history: ProposalHistoryEntry[]
}

// ---------------------------------------------------------------------------
// Ce qui attend une action
// ---------------------------------------------------------------------------

/**
 * NATURE D'UNE ACTION EN ATTENTE. Un code, pas une phrase : le libellé est un
 * texte d'interface (i18n), le code voyage.
 *
 * Chacune est une chose que l'organisation seule peut débloquer — c'est le
 * critère d'entrée dans le bloc. Ce que le comité doit faire n'y figure pas :
 * une liste où figure ce qu'on ne peut pas traiter cesse d'être lue.
 */
export type WorkspaceActionKind =
  | 'changes_requested'
  | 'draft_before_deadline'
  | 'coorganization_to_confirm'
  | 'membership_request'
  | 'session_report_missing'

/** Une ligne du bloc « ce qui attend une action de ma part ». */
export interface WorkspaceAction {
  kind: WorkspaceActionKind
  /** Dossier concerné, quand il y en a un (les adhésions n'en ont pas). */
  proposal_id: ProposalId | null
  /** Numéro de dossier, repris tel quel : c'est par lui qu'on désigne l'affaire. */
  reference_code: string | null
  /** Ce dont il s'agit : titre du dossier, nom de la personne, titre de la séance. */
  subject: string
  /** Points à traiter — corrections ouvertes, séances sans compte rendu. */
  count: number
  /** Échéance quand il y en a une (clôture de l'appel, date de la séance). */
  due_at: IsoDateTime | null
  /** Destination du clic, relative et non localisée : l'écran la localise. */
  target: string
}

// ---------------------------------------------------------------------------
// Les membres
// ---------------------------------------------------------------------------

/**
 * Un membre, son adhésion et la personne derrière.
 *
 * Une adhésion `pending` porte sa DIRECTION (`invited_at`) : invitation émise
 * par l'organisation, ou demande reçue d'une personne. Les deux se traitent à
 * l'opposé — l'une se relance, l'autre s'accepte — et le modèle les distingue
 * depuis le 17/08.
 */
export interface MemberEntry {
  membership: Membership
  person: Person
  /** L'organisation a invité cette personne et attend sa réponse. */
  is_invitation: boolean
}

/** Invitation d'un membre par son adresse. */
export interface InviteMemberPayload {
  organization_id: OrganizationId
  email: Email
  role: Membership['role']
  job_title: string | null
}

/**
 * DÉCISION D'UN RÉFÉRENT SUR UNE DEMANDE D'ADHÉSION.
 *
 * Elle ne vise QUE les demandes — `invited_at IS NULL`. Une invitation émise par
 * l'organisation n'a pas à être « acceptée » par elle : elle attend la personne,
 * et l'approuver reviendrait à faire entrer quelqu'un qui n'a rien répondu.
 * C'est précisément ce que la colonne `invited_at` permet d'empêcher.
 */
export interface DecideMembershipPayload {
  membership_id: Uuid
  approved: boolean
}

/**
 * Réponse à une invitation. Trois issues, et l'écran les rend différemment :
 * `invited` (jeton envoyé), `already_member` (rien à faire), `already_invited`
 * (invitation déjà en vol — on propose de la relancer, pas d'en émettre une
 * seconde).
 */
export type InviteMemberResult =
  | { status: 'invited'; entry: MemberEntry }
  | { status: 'already_member'; entry: MemberEntry }
  | { status: 'already_invited'; entry: MemberEntry }

/**
 * L'AUTRE BOUT DE L'INVITATION : ce que rend le lien reçu par courriel, une fois
 * suivi. `POST /organizations/invitations/accept`, corps `{ token }`.
 *
 * AUCUNE SESSION N'EST EXIGÉE, et c'est ce qui distingue cet appel de tous les
 * autres de ce fichier : le jeton EST la preuve d'adresse. La personne visée par
 * une invitation n'a le plus souvent pas encore de compte — le lui demander
 * avant d'accepter serait exiger d'elle ce que l'invitation est censée
 * déclencher.
 *
 * LES TROIS REFUS SONT CEUX DES JETONS DU NOYAU, et l'écran les rend
 * différemment : un lien périmé se redemande à l'organisation, un lien déjà
 * utilisé annonce que le travail est fait, un lien invalide ne suppose rien.
 *
 * L'ADHÉSION RENDUE EST LA LIGNE APPROUVÉE, pas celle d'avant : l'API l'active
 * dans la même transaction, et l'écran nomme l'organisation rejointe à partir de
 * la fiche jointe.
 */
export type AcceptInvitationResult =
  | { status: 'accepted'; membership: Membership; organization: Organization }
  | { status: 'rejected'; reason: TokenRejection }

// ---------------------------------------------------------------------------
// Le fil de discussion
// ---------------------------------------------------------------------------

/** Réponse à un message du comité. */
export interface ReplyToCommentPayload {
  proposal_id: ProposalId
  /** Message auquel on répond — la racine du fil, jamais une réponse. */
  parent_id: Uuid
  body: string
}

/**
 * Marquage « résolu » d'une demande de correction.
 *
 * QUI PEUT LE POSER n'est écrit nulle part dans le modèle : `resolved_by` est
 * une simple clé étrangère vers une personne. L'écran l'ouvre au soumissionnaire
 * — c'est lui qui sait qu'il a corrigé —, et le comité garde la main pour le
 * retirer. Obligation d'API : c'est une règle d'autorisation, elle appartient à
 * `identity.has_permission()`, pas à un formulaire.
 */
export interface ResolveCommentPayload {
  comment_id: Uuid
  resolved: boolean
}

// ---------------------------------------------------------------------------
// La vue d'ensemble
// ---------------------------------------------------------------------------

/**
 * TOUT CE QUE LA PAGE D'ACCUEIL DE L'ESPACE AFFICHE, en une réponse.
 *
 * `open_call` et `call_edition` ne sont pas un ornement : l'état vide — aucune
 * proposition — met l'appel en cours en avant, et c'est la seule chose utile à
 * montrer à qui n'a rien déposé.
 */
export interface WorkspaceOverview {
  organization: Organization
  /** L'adhésion de la personne connectée : c'est elle qui ouvre les actions de
   *  référent (inviter, accepter une demande). */
  membership: Membership
  proposals: ProposalTracking[]
  members: MemberEntry[]
  actions: WorkspaceAction[]
  open_call: CallForProposals | null
  call_edition: EventEdition | null
}
