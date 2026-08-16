/**
 * Schéma `programme`, partie 4 — formulaires d'inscription configurables,
 * inscriptions, questions du public.
 * Dérivé de `docs/database/075_programme_sessions.sql` § 3, 4 et 5.
 *
 * DÉCISION DU MODÈLE : les QUESTIONS sont des données, les RÉPONSES un document
 * JSON validé en base. La v1 a vu `activity_registrations` grossir au fil des
 * besoins — six colonnes `guest_*`, `referral_source`, `fallback_payload`, plus
 * une table annexe de données démographiques : chaque nouvelle question coûtait
 * une migration et un déploiement.
 */

import type {
  AssetId,
  EventId,
  I18nText,
  IsoDateTime,
  OrganizationId,
  PersonId,
  RegistrationFormId,
  RegistrationId,
  SessionId,
  Uuid,
} from '../shared'

// ---------------------------------------------------------------------------
// Formulaires
// ---------------------------------------------------------------------------

/** Table `programme.registration_forms` — `075` § 3. */
export interface RegistrationForm {
  id: RegistrationFormId
  code: string
  name: I18nText
  description: I18nText | null
  /** Formulaire appliqué par défaut aux sessions d'une édition ; nul pour le
   *  formulaire par défaut de la plateforme. */
  event_id: EventId | null
  is_default: boolean
  /** Une personne non connectée peut-elle s'inscrire ? Le cas « invité » de la
   *  v1 devient un réglage : la personne est créée dans `identity.people` sans
   *  compte, et sera rattachée à son compte le jour où elle en ouvre un. */
  allows_anonymous: boolean
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/** ENUM `programme.form_field_type`. */
export type FormFieldType =
  | 'text'
  | 'long_text'
  | 'email'
  | 'phone'
  | 'number'
  | 'date'
  | 'single_choice'
  | 'multiple_choice'
  | 'boolean'
  | 'country'
  | 'taxonomy_term'

/**
 * Options d'un champ à choix : soit une liste explicite, soit le renvoi à une
 * taxonomie de `reference.taxonomy_terms` — auquel cas les libellés viennent de
 * la base, jamais des fichiers i18n.
 */
export type FormFieldOptions =
  | { taxonomy: string }
  | { values: Array<{ value: string; label: I18nText }> }
  | Record<string, never>

/** Table `programme.registration_form_fields` — `075` § 3. */
export interface RegistrationFormField {
  id: Uuid
  form_id: RegistrationFormId
  /** Clé de la réponse dans `Registration.answers`. */
  code: string
  label: I18nText
  help_text: I18nText | null
  field_type: FormFieldType
  is_required: boolean
  options: FormFieldOptions
  /** Contraintes de saisie : `min`, `max`, `pattern`… */
  validation: Record<string, unknown>
  /** Donnée personnelle sensible : impose un consentement et l'exclusion des
   *  exports non anonymisés. */
  is_sensitive: boolean
  sort_order: number
  is_active: boolean
}

// ---------------------------------------------------------------------------
// Inscriptions
// ---------------------------------------------------------------------------

/** ENUM `programme.registration_status`. */
export type RegistrationStatus = 'registered' | 'waitlisted' | 'cancelled' | 'attended' | 'no_show'

/** Contrainte `ck` sur `programme.registrations.source`. */
export type RegistrationSource = 'web' | 'import' | 'admin' | 'api' | 'partner'

/** Table `programme.registrations` — `075` § 4. */
export interface Registration {
  id: RegistrationId
  session_id: SessionId
  /** Une seule colonne quel que soit le profil : la personne existe toujours,
   *  avec ou sans compte. Fin de la dualité utilisateur/invité de la v1. */
  person_id: PersonId
  organization_id: OrganizationId | null
  status: RegistrationStatus
  /** Réponses au formulaire ; les clés sont les `code` des champs actifs. */
  answers: Record<string, unknown>
  /** Référence `reference.locales.code` — langue des courriels de rappel. */
  locale: string
  /** Renseigné si et seulement si `status` vaut `waitlisted`. */
  waitlist_position: number | null
  /** Premier clic sur « Rejoindre », écrit une seule fois : base du taux de
   *  participation réel. */
  joined_at: IsoDateTime | null
  attendance_minutes: number | null
  certificate_asset_id: AssetId | null
  source: RegistrationSource
  cancelled_at: IsoDateTime | null
  cancelled_reason: string | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Questions du public
// ---------------------------------------------------------------------------

/** Table `programme.session_questions` — `075` § 5. */
export interface SessionQuestion {
  id: Uuid
  session_id: SessionId
  /** Nul pour une question posée sans compte, ou après anonymisation. */
  person_id: PersonId | null
  body: string
  /** `identity.people.id` des intervenants visés. */
  target_speaker_ids: PersonId[]
  upvotes: number
  /** Une question masquée reste tracée : qui, quand, pourquoi. */
  is_visible: boolean
  moderated_by: PersonId | null
  moderated_at: IsoDateTime | null
  moderation_reason: string | null
  answered_at: IsoDateTime | null
  created_at: IsoDateTime
}

/** Table `programme.session_question_answers` — `075` § 5. */
export interface SessionQuestionAnswer {
  id: Uuid
  question_id: Uuid
  answered_by: PersonId | null
  body: string
  /** Réponse officielle de l'intervenant ou de l'équipe, par opposition à une
   *  contribution du public. */
  is_official: boolean
  created_at: IsoDateTime
}

/** Table `programme.session_question_votes` — `075` § 5. */
export interface SessionQuestionVote {
  question_id: Uuid
  person_id: PersonId
  created_at: IsoDateTime
}
