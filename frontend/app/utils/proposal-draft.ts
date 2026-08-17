/**
 * LE DOSSIER, HORS DE TOUT COMPOSANT — brouillon vierge, validation, et
 * conversion vers ce que l'API attend.
 *
 * POURQUOI DES FONCTIONS PURES ET NON UN COMPOSABLE. Ces règles sont consultées
 * de trois endroits qui ne se connaissent pas : la barre d'étapes (qui doit
 * marquer en défaut une étape qu'on a quittée), le récapitulatif de l'étape 7,
 * et le bouton d'envoi. Trois lectures d'une même vérité ; écrite trois fois,
 * elle diverge, et c'est l'étape verte au-dessus d'un envoi refusé.
 *
 * CE QUE CES RÈGLES NE SONT PAS : un contrôle. La recevabilité se tranche en
 * base — `tg_check_submission_eligibility()` pour la fenêtre de l'appel et le
 * plafond par organisation, les `CHECK` de `programme.proposals` pour les
 * bornes. On les rejoue ici pour ne pas laisser remplir sept étapes avant de
 * refuser, jamais pour s'y substituer.
 *
 * D'OÙ VIENT CHAQUE OBLIGATION, ET POURQUOI CELA COMPTE :
 *
 *   · `NOT NULL` de la table  → erreur. La base refuserait la ligne.
 *   · borne de l'appel        → erreur. `min_speakers`, `max_speakers`,
 *                               `allowed_formats` sont des données de l'appel ;
 *                               aucune ne doit être écrite en dur ici.
 *   · absence regrettable     → avertissement. Un dossier sans résumé se dépose
 *                               et s'évalue mal ; le peindre en rouge
 *                               apprendrait à ignorer le rouge.
 */

import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type {
  DraftDocument,
  DraftIssue,
  DraftSpeaker,
  ProposalDraft,
} from '~/types/proposal-form'
import {
  DOCUMENT_MAX_BYTES,
  DOCUMENT_MIME_PREFIXES,
  TEXT_LIMITS,
} from '~/types/proposal-form'

/** Brouillon vierge — les valeurs par défaut sont celles des colonnes. */
export function emptyProposalDraft(defaults: {
  organizationId?: string | null
  /** `calls_for_proposals.default_duration_minutes`. */
  durationMinutes?: number | null
  /** Langue de l'interface au moment où le dossier s'ouvre. */
  locale?: string
}): ProposalDraft {
  return {
    organization_id: defaults.organizationId ?? null,
    co_organizations: [],
    title: '',
    summary: '',
    objectives: '',
    detailed_presentation: '',
    expected_outcomes: '',
    target_audiences: [],
    theme_codes: [],
    activity_type_code: null,
    format: null,
    // `proposals.language_codes` a pour défaut `{fr}` : on part de la langue de
    // travail de la personne, qu'elle reste libre de retirer.
    language_codes: [defaults.locale === 'en' ? 'en' : 'fr'],
    country_id: null,
    speakers: [],
    preferred_start_at: null,
    duration_minutes: defaults.durationMinutes ?? null,
    requested_sessions: 1,
    scheduling_constraints: '',
    documents: [],
  }
}

/** Clé de liste stable, le temps de la saisie. Aucune existence en base. */
export function draftKey(prefix: string, index: number): string {
  return `${prefix}-${index}-${Math.random().toString(36).slice(2, 8)}`
}

/** Intervenant vierge. `speaker` est le rôle par défaut de la colonne. */
export function emptyDraftSpeaker(index: number): DraftSpeaker {
  return {
    key: draftKey('speaker', index),
    person_id: null,
    has_account: false,
    civility: null,
    first_name: '',
    last_name: '',
    email: '',
    job_title: '',
    organization_name: '',
    organization_id: null,
    role: 'speaker',
    bio: '',
    photo: null,
  }
}

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/

/**
 * Le TEXTE d'un fragment de texte riche, balisage retiré.
 *
 * Sert à deux choses, et à rien d'autre : compter des caractères, et savoir si
 * un champ est vide — un document vierge de ProseMirror vaut `<p></p>`, ce qui
 * n'est pas une chaîne vide et passerait pour un contenu rédigé.
 */
export function plainTextOf(html: string): string {
  return html
    .replace(/<[^>]*>/g, ' ')
    .replace(/&nbsp;/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

/**
 * Tous les défauts du dossier, étape par étape.
 *
 * L'ORDRE EST CELUI DU PARCOURS, et il est structurant : le résumé des erreurs
 * affiché en tête de page les énumère dans cet ordre, si bien qu'on corrige en
 * avançant plutôt qu'en sautant d'une étape à l'autre.
 */
export function validateProposalDraft(
  draft: ProposalDraft,
  call: CallForProposals,
  edition: EventEdition,
): DraftIssue[] {
  return [
    ...validateOrganizations(draft),
    ...validatePresentation(draft),
    ...validateClassification(draft, call),
    ...validateSpeakers(draft, call),
    ...validateSchedule(draft, call, edition),
    ...validateDocuments(draft),
  ]
}

function validateOrganizations(draft: ProposalDraft): DraftIssue[] {
  const issues: DraftIssue[] = []

  if (!draft.organization_id) {
    issues.push({
      step: 'organizations',
      field: 'organization_id',
      severity: 'error',
      messageKey: 'proposal.form.step-organizations.errors.leadRequired',
    })
  }

  return issues
}

function validatePresentation(draft: ProposalDraft): DraftIssue[] {
  const issues: DraftIssue[] = []

  // Les trois colonnes `NOT NULL` du dossier : titre, objectifs, présentation.
  const required = [
    ['title', draft.title],
    ['objectives', draft.objectives],
    ['detailed_presentation', plainTextOf(draft.detailed_presentation)],
  ] as const

  for (const [field, value] of required) {
    if (value.trim().length === 0) {
      issues.push({
        step: 'presentation',
        field,
        severity: 'error',
        messageKey: `proposal.form.step-presentation.errors.${field}Required`,
      })
    }
  }

  // Les trois colonnes facultatives. Elles ne bloquent rien, mais un dossier qui
  // ne dit ni ce qu'il produit ni à qui il s'adresse s'évalue mal : le comité
  // note sur six critères, dont l'impact et l'inclusion.
  const recommended = [
    ['summary', draft.summary],
    ['expected_outcomes', draft.expected_outcomes],
  ] as const

  for (const [field, value] of recommended) {
    if (value.trim().length === 0) {
      issues.push({
        step: 'presentation',
        field,
        severity: 'warning',
        messageKey: `proposal.form.step-presentation.warnings.${field}Missing`,
      })
    }
  }

  // Le public visé est une LISTE : c'est son absence complète qui se signale,
  // pas une chaîne vide.
  if (draft.target_audiences.length === 0) {
    issues.push({
      step: 'presentation',
      field: 'target_audiences',
      severity: 'warning',
      messageKey: 'proposal.form.step-presentation.warnings.target_audienceMissing',
    })
  }

  // Dépassements de longueur : la saisie n'a pas été coupée, l'envoi refuse.
  const limited = [
    ['title', draft.title, TEXT_LIMITS.title],
    ['summary', draft.summary, TEXT_LIMITS.summary],
    ['objectives', draft.objectives, TEXT_LIMITS.objectives],
    // La présentation détaillée est du HTML : on compte le TEXTE. Compter le
    // balisage ferait grossir le décompte à chaque mise en gras.
    ['detailed_presentation', plainTextOf(draft.detailed_presentation), TEXT_LIMITS.detailed_presentation],
    ['expected_outcomes', draft.expected_outcomes, TEXT_LIMITS.expected_outcomes],
  ] as const

  for (const [field, value, max] of limited) {
    if (value.length > max) {
      issues.push({
        step: 'presentation',
        field,
        severity: 'error',
        messageKey: 'validation.maxLength',
        params: { max, count: max },
      })
    }
  }

  return issues
}

function validateClassification(draft: ProposalDraft, call: CallForProposals): DraftIssue[] {
  const issues: DraftIssue[] = []

  if (!draft.format) {
    issues.push({
      step: 'classification',
      field: 'format',
      severity: 'error',
      messageKey: 'proposal.form.step-classification.errors.formatRequired',
    })
  } else if (!call.allowed_formats.includes(draft.format)) {
    // Le cas se produit sur un brouillon repris après que l'IFDD a restreint les
    // formats de l'appel. Silencieux sans ce contrôle, refusé à l'envoi.
    issues.push({
      step: 'classification',
      field: 'format',
      severity: 'error',
      messageKey: 'proposal.form.step-classification.errors.formatNotAllowed',
    })
  }

  if (draft.language_codes.length === 0) {
    issues.push({
      step: 'classification',
      field: 'language_codes',
      severity: 'error',
      messageKey: 'proposal.form.step-classification.errors.languagesRequired',
    })
  }

  // Ni les thématiques ni la catégorie ne sont obligatoires en base — les
  // premières vivent dans `reference.entity_terms`, la seconde est nullable.
  // Elles commandent pourtant les filtres de la programmation publique : un
  // dossier sans thématique devient introuvable une fois retenu.
  if (draft.theme_codes.length === 0) {
    issues.push({
      step: 'classification',
      field: 'theme_codes',
      severity: 'warning',
      messageKey: 'proposal.form.step-classification.warnings.themesMissing',
    })
  }

  if (!draft.activity_type_code) {
    issues.push({
      step: 'classification',
      field: 'activity_type_code',
      severity: 'warning',
      messageKey: 'proposal.form.step-classification.warnings.categoryMissing',
    })
  }

  return issues
}

function validateSpeakers(draft: ProposalDraft, call: CallForProposals): DraftIssue[] {
  const issues: DraftIssue[] = []

  // Les deux bornes viennent de l'appel, pas d'une constante : la COP29 en
  // demandait deux à six, un autre appel en demandera d'autres.
  if (draft.speakers.length < call.min_speakers) {
    issues.push({
      step: 'speakers',
      field: 'speakers',
      severity: 'error',
      messageKey: 'proposal.form.step-speakers.errors.tooFew',
      params: { min: call.min_speakers, count: call.min_speakers },
    })
  }

  if (draft.speakers.length > call.max_speakers) {
    issues.push({
      step: 'speakers',
      field: 'speakers',
      severity: 'error',
      messageKey: 'proposal.form.step-speakers.errors.tooMany',
      params: { max: call.max_speakers, count: call.max_speakers },
    })
  }

  draft.speakers.forEach((speaker, index) => {
    const position = index + 1
    const name = `${speaker.first_name} ${speaker.last_name}`.trim()
    const label = name.length > 0 ? name : String(position)

    if (speaker.first_name.trim().length === 0 || speaker.last_name.trim().length === 0) {
      issues.push({
        step: 'speakers',
        field: `speakers.${speaker.key}.name`,
        severity: 'error',
        messageKey: 'proposal.form.step-speakers.errors.nameRequired',
        params: { position },
      })
    }

    // `identity.people.primary_email` est NOT NULL : c'est par elle que l'API
    // rapproche l'intervenant d'une personne déjà connue, ou en crée une.
    if (!EMAIL_PATTERN.test(speaker.email.trim())) {
      issues.push({
        step: 'speakers',
        field: `speakers.${speaker.key}.email`,
        severity: 'error',
        messageKey: 'proposal.form.step-speakers.errors.emailInvalid',
        params: { speaker: label },
      })
    }

    if (speaker.bio.length > TEXT_LIMITS.speaker_bio) {
      issues.push({
        step: 'speakers',
        field: `speakers.${speaker.key}.bio`,
        severity: 'error',
        messageKey: 'validation.maxLength',
        params: { max: TEXT_LIMITS.speaker_bio, count: TEXT_LIMITS.speaker_bio },
      })
    }

    // CIVILITÉ, FONCTION ET ORGANISATION SONT OBLIGATOIRES — arbitrage du
    // commanditaire du 17/08. Aucune des trois n'est `NOT NULL` en base : ce
    // sont des exigences de dossier, pas de données. Elles se défendent : le
    // programme imprimé annonce « Mme Awa Sow Fall, directrice exécutive,
    // ROAC », et une ligne amputée s'y voit immédiatement.
    if (!speaker.civility) {
      issues.push({
        step: 'speakers',
        field: `speakers.${speaker.key}.civility`,
        severity: 'error',
        messageKey: 'proposal.form.step-speakers.errors.civilityRequired',
        params: { speaker: label },
      })
    }

    if (speaker.job_title.trim().length === 0) {
      issues.push({
        step: 'speakers',
        field: `speakers.${speaker.key}.job_title`,
        severity: 'error',
        messageKey: 'proposal.form.step-speakers.errors.jobTitleRequired',
        params: { speaker: label },
      })
    }

    if (speaker.organization_name.trim().length === 0) {
      issues.push({
        step: 'speakers',
        field: `speakers.${speaker.key}.organization`,
        severity: 'error',
        messageKey: 'proposal.form.step-speakers.errors.organizationRequired',
        params: { speaker: label },
      })
    }
  })

  // Une même adresse deux fois, c'est une personne dédoublée dans l'annuaire —
  // et `ux_proposal_speakers` refuserait le second rôle identique.
  const seen = new Set<string>()
  for (const speaker of draft.speakers) {
    const email = speaker.email.trim().toLowerCase()
    if (email.length === 0) continue
    if (seen.has(email)) {
      issues.push({
        step: 'speakers',
        field: `speakers.${speaker.key}.email`,
        severity: 'error',
        messageKey: 'proposal.form.step-speakers.errors.emailDuplicate',
        params: { email },
      })
    }
    seen.add(email)
  }

  return issues
}

function validateSchedule(
  draft: ProposalDraft,
  call: CallForProposals,
  edition: EventEdition,
): DraftIssue[] {
  const issues: DraftIssue[] = []

  // LES BORNES VIENNENT DE L'APPEL, pas d'une constante : `min_duration_minutes`
  // et `max_duration_minutes` sont des colonnes depuis le 17/08. Le `CHECK` de
  // `proposals.duration_minutes` (15 à 600) reste en base comme garde-fou de
  // données ; ce n'est pas lui qui dit ce qu'un pavillon accepte.
  if (draft.duration_minutes === null) {
    issues.push({
      step: 'schedule',
      field: 'duration_minutes',
      severity: 'error',
      messageKey: 'proposal.form.step-schedule.errors.durationRequired',
    })
  } else if (
    draft.duration_minutes < call.min_duration_minutes ||
    draft.duration_minutes > call.max_duration_minutes
  ) {
    issues.push({
      step: 'schedule',
      field: 'duration_minutes',
      severity: 'error',
      messageKey: 'proposal.form.step-schedule.errors.durationRange',
      params: { min: call.min_duration_minutes, max: call.max_duration_minutes },
    })
  }

  // Le `CHECK` de la colonne `requested_sessions` : 1 à 50 occurrences.
  if (
    !Number.isInteger(draft.requested_sessions) ||
    draft.requested_sessions < 1 ||
    draft.requested_sessions > 50
  ) {
    issues.push({
      step: 'schedule',
      field: 'requested_sessions',
      severity: 'error',
      messageKey: 'proposal.form.step-schedule.errors.sessionsRange',
      params: { min: 1, max: 50 },
    })
  }

  if (!draft.preferred_start_at) {
    issues.push({
      step: 'schedule',
      field: 'preferred_start_at',
      severity: 'error',
      messageKey: 'proposal.form.step-schedule.errors.slotRequired',
    })
  } else {
    // La préférence doit tomber pendant l'édition, et dans la plage d'accueil du
    // pavillon. AUCUN chevauchement n'est vérifié, et c'est la règle métier n° 2 :
    // les organisations proposent librement, l'IFDD arbitre. Une plage horaire
    // n'est pas un chevauchement — c'est l'amplitude d'ouverture du stand, un
    // fait matériel que rien ne peut contourner.
    const start = instantFromWallClock(draft.preferred_start_at, edition.timezone)
    const startMs = start ? Date.parse(start) : Number.NaN
    const from = Date.parse(edition.starts_at)
    const to = Date.parse(edition.ends_at)

    if (!Number.isFinite(startMs)) {
      issues.push({
        step: 'schedule',
        field: 'preferred_start_at',
        severity: 'error',
        messageKey: 'validation.date',
      })
    } else if (startMs < from || startMs > to) {
      issues.push({
        step: 'schedule',
        field: 'preferred_start_at',
        severity: 'error',
        messageKey: 'proposal.form.step-schedule.errors.outsideEdition',
      })
    } else {
      const window = dailyWindowIssue(draft, call)
      if (window) issues.push(window)
    }
  }

  if (draft.scheduling_constraints.length > TEXT_LIMITS.scheduling_constraints) {
    issues.push({
      step: 'schedule',
      field: 'scheduling_constraints',
      severity: 'error',
      messageKey: 'validation.maxLength',
      params: {
        max: TEXT_LIMITS.scheduling_constraints,
        count: TEXT_LIMITS.scheduling_constraints,
      },
    })
  }

  return issues
}

function validateDocuments(draft: ProposalDraft): DraftIssue[] {
  const issues: DraftIssue[] = []

  for (const document of draft.documents) {
    if (document.title.trim().length === 0) {
      issues.push({
        step: 'documents',
        field: `documents.${document.key}.title`,
        severity: 'error',
        messageKey: 'proposal.form.step-documents.errors.titleRequired',
        params: { file: document.upload.file_name },
      })
    }

    const rejection = rejectDocument(document.upload)
    if (rejection) {
      issues.push({
        step: 'documents',
        field: `documents.${document.key}.file`,
        severity: 'error',
        messageKey: rejection,
        params: { file: document.upload.file_name },
      })
    }
  }

  return issues
}

/**
 * L'ACTIVITÉ TIENT-ELLE DANS LA PLAGE D'ACCUEIL DU PAVILLON ?
 *
 * `calls_for_proposals.daily_start_time` et `daily_end_time` sont des colonnes
 * depuis le 17/08, en heure LOCALE de l'événement. La comparaison se fait donc
 * sur l'heure MURALE saisie — jamais sur l'instant : convertir en UTC pour
 * comparer à « 09:00 » n'aurait aucun sens, ces deux bornes n'étant pas datées.
 *
 * C'est la FIN qui doit tenir avant la fermeture, début plus durée : une séance
 * de deux heures commencée à 16 h déborde d'une heure sur un stand fermé.
 */
function dailyWindowIssue(draft: ProposalDraft, call: CallForProposals): DraftIssue | null {
  const time = /T(\d{2}):(\d{2})/.exec(draft.preferred_start_at ?? '')
  if (!time) return null

  const startMinutes = Number(time[1]) * 60 + Number(time[2])
  const endMinutes = startMinutes + (draft.duration_minutes ?? 0)
  const open = timeToMinutes(call.daily_start_time)
  const close = timeToMinutes(call.daily_end_time)

  if (startMinutes < open || endMinutes > close) {
    return {
      step: 'schedule',
      field: 'preferred_start_at',
      severity: 'error',
      messageKey: 'proposal.form.step-schedule.errors.outsideDailyWindow',
      params: {
        open: call.daily_start_time.slice(0, 5),
        close: call.daily_end_time.slice(0, 5),
      },
    }
  }
  return null
}

/** « 09:00:00 » ou « 09:00 » → minutes depuis minuit. */
export function timeToMinutes(value: string): number {
  const parts = /^(\d{2}):(\d{2})/.exec(value)
  return parts ? Number(parts[1]) * 60 + Number(parts[2]) : 0
}

/**
 * Le fichier est-il recevable ? Rend la clé du refus, ou `null`.
 *
 * Mêmes bornes que `media.attachable_roles` pour le rôle `document` : format et
 * taille. Le refus se dit AVANT le téléversement — laisser partir vingt-cinq
 * mégaoctets pour les rejeter à l'arrivée est une minute perdue sur une
 * connexion lente, et une raison d'abandonner.
 */
export function rejectDocument(upload: { mime_type: string; byte_size: number }): string | null {
  const accepted = DOCUMENT_MIME_PREFIXES.some((prefix) => upload.mime_type.startsWith(prefix))
  if (!accepted) return 'proposal.form.step-documents.errors.fileType'
  if (upload.byte_size > DOCUMENT_MAX_BYTES) return 'proposal.form.step-documents.errors.fileTooLarge'
  return null
}

/** « 2,4 Mo » — taille lisible, séparateur de la locale active. */
export function formatByteSize(bytes: number, locale: string): string {
  const units = ['o', 'ko', 'Mo', 'Go']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  const formatted = new Intl.NumberFormat(locale, {
    maximumFractionDigits: unit === 0 ? 0 : 1,
  }).format(value)
  return `${formatted} ${units[unit]}`
}

/** Les défauts d'une étape donnée, gravité comprise. */
export function issuesOfStep(issues: DraftIssue[], step: string): DraftIssue[] {
  return issues.filter((issue) => issue.step === step)
}

/** Le dossier est-il envoyable ? Seules les erreurs comptent. */
export function hasBlockingIssue(issues: DraftIssue[]): boolean {
  return issues.some((issue) => issue.severity === 'error')
}
