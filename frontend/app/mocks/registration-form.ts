/**
 * Données simulées de `programme.registration_forms` et
 * `programme.registration_form_fields`.
 *
 * LES QUESTIONS SONT DES DONNÉES, LES RÉPONSES UN DOCUMENT JSON. La v1 a vu sa
 * table d'inscriptions grossir au fil des besoins — six colonnes `guest_*`, un
 * `referral_source`, un `fallback_payload`, plus une table annexe de données
 * démographiques : chaque nouvelle question coûtait une migration et un
 * déploiement. Ici, ajouter une question est une insertion.
 *
 * Deux formulaires : celui de la plateforme, repris de
 * `075_programme_sessions.sql`, et celui de l'édition COP31, qui y ajoute ce que
 * le pavillon a besoin de savoir — accréditation en zone bleue et besoins
 * d'accessibilité.
 *
 * `is_sensitive` marque une donnée personnelle sensible : elle impose un
 * consentement et sort des exports non anonymisés. Les besoins d'accessibilité
 * en relèvent ; la fonction et l'organisation, non.
 *
 * LES LIBELLÉS D'OPTIONS D'UNE LISTE ADOSSÉE À UNE TAXONOMIE NE SONT PAS ÉCRITS
 * ICI : le champ `referral_source` renvoie à `reference.taxonomy_terms`, et ses
 * libellés viennent de `mocks/reference.ts`. Les recopier serait exactement
 * l'erreur que le projet interdit.
 */

import type { RegistrationForm, RegistrationFormField } from '~/types/programme/registration'
import { EVENT, FORM_FIELD, REGISTRATION_FORM } from './ids'

export const registrationForms = [
  {
    id: REGISTRATION_FORM.default,
    code: 'default',
    name: { fr: 'Inscription standard', en: 'Standard registration' },
    description: {
      fr: "Formulaire par défaut appliqué aux sessions sans formulaire dédié",
      en: 'Default form',
    },
    event_id: null,
    is_default: true,
    allows_anonymous: true,
    created_at: '2026-01-12T09:00:00Z',
    updated_at: '2026-01-12T09:00:00Z',
  },
  {
    id: REGISTRATION_FORM.cop31,
    code: 'cop31',
    name: { fr: 'Inscription aux activités de la COP31', en: 'COP31 activity registration' },
    description: {
      fr: "Formulaire appliqué aux sessions du pavillon de la Francophonie à Belém.",
      en: 'Form applied to the Francophonie Pavilion sessions in Belém.',
    },
    event_id: EVENT.cop31,
    is_default: false,
    // Une personne sans compte peut s'inscrire : elle est créée dans
    // `identity.people` et sera rattachée à son compte le jour où elle en ouvre
    // un. Le « cas invité » de la v1 devient un réglage.
    allows_anonymous: true,
    created_at: '2026-07-20T10:00:00Z',
    updated_at: '2026-08-03T14:00:00Z',
  },
] satisfies RegistrationForm[]

export const registrationFormFields = [
  // --- Formulaire par défaut de la plateforme ------------------------------
  {
    id: FORM_FIELD(1),
    form_id: REGISTRATION_FORM.default,
    code: 'job_title',
    label: { fr: 'Fonction', en: 'Job title' },
    help_text: null,
    field_type: 'text',
    is_required: false,
    options: {},
    validation: { maxLength: 120 },
    is_sensitive: false,
    sort_order: 10,
    is_active: true,
  },
  {
    id: FORM_FIELD(2),
    form_id: REGISTRATION_FORM.default,
    code: 'organization',
    label: { fr: 'Organisation', en: 'Organization' },
    help_text: null,
    field_type: 'text',
    is_required: false,
    options: {},
    validation: { maxLength: 160 },
    is_sensitive: false,
    sort_order: 20,
    is_active: true,
  },
  {
    id: FORM_FIELD(3),
    form_id: REGISTRATION_FORM.default,
    code: 'country',
    label: { fr: 'Pays', en: 'Country' },
    help_text: null,
    field_type: 'country',
    is_required: true,
    options: {},
    validation: {},
    is_sensitive: false,
    sort_order: 30,
    is_active: true,
  },
  {
    id: FORM_FIELD(4),
    form_id: REGISTRATION_FORM.default,
    code: 'referral_source',
    label: {
      fr: 'Comment avez-vous connu cette activité ?',
      en: 'How did you hear about this activity?',
    },
    help_text: null,
    field_type: 'single_choice',
    is_required: false,
    // Les libellés viennent de la taxonomie, jamais des fichiers i18n.
    options: { taxonomy: 'referral_source' },
    validation: {},
    is_sensitive: false,
    sort_order: 40,
    is_active: true,
  },

  // --- Formulaire de l'édition COP31 ---------------------------------------
  {
    id: FORM_FIELD(10),
    form_id: REGISTRATION_FORM.cop31,
    code: 'job_title',
    label: { fr: 'Fonction', en: 'Job title' },
    help_text: null,
    field_type: 'text',
    is_required: true,
    options: {},
    validation: { maxLength: 120 },
    is_sensitive: false,
    sort_order: 10,
    is_active: true,
  },
  {
    id: FORM_FIELD(11),
    form_id: REGISTRATION_FORM.cop31,
    code: 'organization',
    label: { fr: 'Organisation', en: 'Organization' },
    help_text: {
      fr: "Si votre organisation est déjà inscrite sur la plateforme, choisissez-la plutôt que de la saisir à nouveau.",
    },
    field_type: 'text',
    is_required: true,
    options: {},
    validation: { maxLength: 160 },
    is_sensitive: false,
    sort_order: 20,
    is_active: true,
  },
  {
    id: FORM_FIELD(12),
    form_id: REGISTRATION_FORM.cop31,
    code: 'country',
    label: { fr: 'Pays', en: 'Country' },
    help_text: null,
    field_type: 'country',
    is_required: true,
    options: {},
    validation: {},
    is_sensitive: false,
    sort_order: 30,
    is_active: true,
  },
  {
    id: FORM_FIELD(13),
    form_id: REGISTRATION_FORM.cop31,
    code: 'badge_unfccc',
    label: {
      fr: 'Disposez-vous d’une accréditation CCNUCC (zone bleue) ?',
      en: 'Do you hold a UNFCCC (blue zone) accreditation?',
    },
    help_text: {
      fr: "L'accès physique au pavillon exige un badge CCNUCC. Sans accréditation, l'inscription porte sur la retransmission en ligne.",
      en: 'Physical access requires a UNFCCC badge. Without one, registration covers the online broadcast.',
    },
    field_type: 'boolean',
    is_required: true,
    options: {},
    validation: {},
    is_sensitive: false,
    sort_order: 40,
    is_active: true,
  },
  {
    id: FORM_FIELD(14),
    form_id: REGISTRATION_FORM.cop31,
    code: 'referral_source',
    label: {
      fr: 'Comment avez-vous connu cette activité ?',
      en: 'How did you hear about this activity?',
    },
    help_text: null,
    field_type: 'single_choice',
    is_required: false,
    options: { taxonomy: 'referral_source' },
    validation: {},
    is_sensitive: false,
    sort_order: 50,
    is_active: true,
  },
  {
    // DONNÉE SENSIBLE : consentement exigé, exclue des exports non anonymisés.
    id: FORM_FIELD(15),
    form_id: REGISTRATION_FORM.cop31,
    code: 'access_needs',
    label: {
      fr: "Besoins d'accessibilité ou d'interprétation",
      en: 'Accessibility or interpretation needs',
    },
    help_text: {
      fr: "Ces informations ne sont transmises qu'à l'équipe logistique du pavillon.",
      en: 'Shared only with the pavilion logistics team.',
    },
    field_type: 'long_text',
    is_required: false,
    options: {},
    validation: { maxLength: 500 },
    is_sensitive: true,
    sort_order: 60,
    is_active: true,
  },
  {
    // Champ DÉSACTIVÉ : conservé pour les réponses déjà collectées, retiré du
    // formulaire courant. Le supprimer rendrait illisibles les inscriptions
    // existantes.
    id: FORM_FIELD(16),
    form_id: REGISTRATION_FORM.cop31,
    code: 'dietary',
    label: { fr: 'Régime alimentaire', en: 'Dietary requirements' },
    help_text: null,
    field_type: 'text',
    is_required: false,
    options: {},
    validation: { maxLength: 120 },
    is_sensitive: true,
    sort_order: 70,
    is_active: false,
  },
] satisfies RegistrationFormField[]
