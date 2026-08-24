/**
 * MODIFICATION D'UN DOSSIER EXISTANT — la lecture qui le recompose en
 * brouillon, et les écritures qui l'enregistrent.
 *
 * POURQUOI CE FICHIER EXISTE. `proposal-submission.ts` (prompt A4) ne connaît
 * qu'un seul dossier par personne, celui qu'elle est en train d'écrire :
 * `recordOf(personId)`. Or le commanditaire a tranché le 17/08 — « tant que
 * l'événement n'est pas terminé, il peut modifier » —, et il faut donc pouvoir
 * ouvrir un dossier DÉSIGNÉ, y compris l'un des quarante et un dossiers du jeu,
 * déposé il y a deux mois par quelqu'un d'autre de la même organisation.
 *
 * LA LECTURE EST UNE RECOMPOSITION, pas un simple `SELECT`. Le formulaire
 * travaille sur un `ProposalDraft` — une structure d'ÉCRAN, en français, avec
 * ses heures murales et ses clés de liste —, quand la base range la même chose
 * dans cinq tables (`proposals`, `proposal_organizations`, `proposal_speakers`,
 * `proposal_documents`, `reference.entity_terms`). C'est l'API qui portera
 * cette recomposition (prompt B4) ; ici on la rejoue, comme
 * `organization-search.ts` rejoue `find_similar_organizations()`.
 *
 * TROIS CONVERSIONS VALENT D'ÊTRE SIGNALÉES, chacune ayant sa raison :
 *   · `preferred_start_at` redevient une HEURE MURALE dans le fuseau de
 *     l'édition — sans quoi un créneau saisi à 14:30 à Belém se rouvrirait à
 *     11:30 pour qui modifie depuis Dakar, sans qu'aucune erreur ne soit levée ;
 *   · les textes multilingues sont ramenés à leur FRANÇAIS, la langue que le
 *     formulaire exige (`platform.is_i18n_text`) et la seule qu'il sait rendre ;
 *   · un intervenant retrouve son `has_account`, qui VERROUILLE son identité :
 *     ce n'est pas au déposant de réécrire la fiche de quelqu'un d'autre.
 *
 * LE JOURNAL D'ÉDITION est en mémoire du module, comme partout depuis A2 : les
 * quarante et un dossiers écrits à la main ne sont jamais modifiés, on empile
 * par-dessus. `editedProposal()` rend la surcouche, que l'espace organisation
 * applique pour que le titre corrigé s'affiche là où on vient de le corriger.
 */

import type { Proposal, ProposalStatus } from '~/types/programme/proposal'
import type {
  DraftOrganization,
  EditableProposal,
  ProposalDraft,
  ReopenedDraft,
  ReopenedSpeaker,
  SaveDraftPayload,
  SaveDraftResult,
  SubmitProposalResult,
} from '~/types/proposal-form'
import { accounts } from './auth'
import { events } from './event'
import { callsForProposals } from './calls'
import { organizations } from './org'
import { people } from './people'
import { entityTerms, taxonomyTerms } from './reference'
import { allProposals, proposalOrganizations, proposalSpeakers } from './proposals'

// ---------------------------------------------------------------------------
// Le journal des modifications de la session
// ---------------------------------------------------------------------------

interface EditRecord {
  proposal_id: string
  draft: ProposalDraft
  status: ProposalStatus
  saved_at: string
  submitted_at: string | null
}

const edits = new Map<string, EditRecord>()

/**
 * La surcouche à appliquer à un dossier du jeu : ce qui a été modifié pendant
 * la session. Rend `null` quand rien n'a bougé.
 *
 * Seuls les champs que le FORMULAIRE écrit sont repris — le reste de la ligne
 * (numéro, dates de décision, agrégats de revue) appartient à la base et au
 * comité, et une modification de l'organisation n'y touche pas.
 */
export function editedProposal(proposalId: string): Partial<Proposal> | null {
  const record = edits.get(proposalId)
  if (!record) return null

  return {
    title: { fr: record.draft.title },
    summary: record.draft.summary ? { fr: record.draft.summary } : null,
    objectives: { fr: record.draft.objectives },
    detailed_presentation: { fr: record.draft.detailed_presentation },
    expected_outcomes: record.draft.expected_outcomes ? { fr: record.draft.expected_outcomes } : null,
    target_audiences: record.draft.target_audiences.map((audience) => ({ fr: audience })),
    format: record.draft.format ?? undefined,
    activity_type_code: record.draft.activity_type_code,
    language_codes: record.draft.language_codes,
    country_id: record.draft.country_id,
    duration_minutes: record.draft.duration_minutes,
    requested_sessions: record.draft.requested_sessions,
    scheduling_constraints: record.draft.scheduling_constraints || null,
    status: record.status,
    submitted_at: record.submitted_at,
    updated_at: record.saved_at,
  }
}

// ---------------------------------------------------------------------------
// Lecture — recomposer le brouillon
// ---------------------------------------------------------------------------

/** Le français d'un texte multilingue : la langue du formulaire. */
function fr(value: { fr: string } | null | undefined): string {
  return value?.fr ?? ''
}

/**
 * CE QUE L'API REND D'UN BROUILLON qu'elle a reçu : ni clés de liste, ni photos,
 * ni pièces jointes. Le mock doit dire la même chose qu'elle, sinon la bascule
 * vers l'API réelle se ferait sans que rien ne signale l'écart.
 */
function reopened(draft: ProposalDraft): ReopenedDraft {
  const { documents: _documents, speakers, ...rest } = draft
  return {
    ...rest,
    speakers: speakers.map(({ key: _key, photo: _photo, ...speaker }) => speaker),
  }
}

function coOrganizationsOf(proposalId: string): DraftOrganization[] {
  return proposalOrganizations
    .filter((link) => link.proposal_id === proposalId && link.role !== 'lead')
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((link) => {
      const organization = organizations.find((o) => o.id === link.organization_id)
      return {
        organization_id: link.organization_id,
        role: link.role as DraftOrganization['role'],
        legal_name: organization?.legal_name ?? '',
        acronym: organization?.acronym ?? null,
        country_id: organization?.country_id ?? null,
      }
    })
}

/**
 * NI CLÉ DE LISTE NI PHOTO : l'API n'en rend pas. La clé est locale à l'écran,
 * qui la pose à la réception ; la photo appartient à la fiche de la personne.
 */
function speakersOf(proposalId: string): ReopenedSpeaker[] {
  return proposalSpeakers
    .filter((speaker) => speaker.proposal_id === proposalId)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((speaker) => {
      const person = people.find((p) => p.id === speaker.person_id)
      return {
        person_id: speaker.person_id,
        // L'identité d'une personne qui a un COMPTE lui appartient : le
        // formulaire la verrouille (écart n° 31).
        has_account: accounts.some((account) => account.person_id === speaker.person_id),
        civility: person?.civility ?? null,
        first_name: person?.first_name ?? '',
        last_name: person?.last_name ?? '',
        email: person?.primary_email ?? '',
        job_title: speaker.job_title_snapshot ?? '',
        organization_name: speaker.organization_snapshot ?? '',
        organization_id: speaker.organization_id,
        role: speaker.role,
        bio: fr(speaker.bio),
      }
    })
}

function themeCodesOf(proposalId: string): string[] {
  const termIds = new Set(
    entityTerms
      .filter((link) => link.entity_table === 'proposals' && link.entity_id === proposalId)
      .map((link) => link.term_id),
  )
  return taxonomyTerms
    .filter((term) => termIds.has(term.id) && term.taxonomy_code === 'activity_theme')
    .map((term) => term.code)
}

/**
 * Recompose le brouillon d'un dossier existant.
 *
 * Rend `null` si le dossier n'existe pas — l'écran affiche alors son état
 * « introuvable » plutôt qu'un formulaire vide qui écraserait quelque chose.
 */
export function editableProposal(proposalId: string): EditableProposal | null {
  const stored = allProposals.find((proposal) => proposal.id === proposalId)
  if (!stored) return null

  const record = edits.get(proposalId)
  if (record) {
    return {
      proposal_id: proposalId,
      reference_code: stored.reference_code,
      call_id: stored.call_id,
      event_id: stored.event_id,
      status: record.status,
      saved_at: record.saved_at,
      draft: reopened(record.draft),
    }
  }

  const edition = events.find((event) => event.id === stored.event_id)
  const timezone = edition?.timezone ?? 'UTC'

  const draft: ReopenedDraft = {
    organization_id: stored.organization_id,
    co_organizations: coOrganizationsOf(proposalId),
    title: fr(stored.title),
    summary: fr(stored.summary),
    objectives: fr(stored.objectives),
    detailed_presentation: fr(stored.detailed_presentation),
    expected_outcomes: fr(stored.expected_outcomes),
    target_audiences: stored.target_audiences.map((audience) => fr(audience)),
    theme_codes: themeCodesOf(proposalId),
    activity_type_code: stored.activity_type_code,
    format: stored.format,
    language_codes: stored.language_codes,
    country_id: stored.country_id,
    speakers: speakersOf(proposalId),
    // Heure MURALE dans le fuseau de l'ÉDITION : c'est ainsi que le créneau a
    // été saisi, et c'est ainsi qu'il doit se rouvrir, où que l'on soit.
    preferred_start_at: stored.preferred_start_at
      ? wallClockInZone(stored.preferred_start_at, timezone).slice(0, 16)
      : null,
    duration_minutes: stored.duration_minutes,
    requested_sessions: stored.requested_sessions,
    scheduling_constraints: stored.scheduling_constraints ?? '',
  }

  return {
    proposal_id: proposalId,
    reference_code: stored.reference_code,
    call_id: stored.call_id,
    event_id: stored.event_id,
    status: stored.status,
    saved_at: stored.updated_at,
    draft,
  }
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

/**
 * Enregistrement d'un dossier EXISTANT. L'état ne change pas : corriger n'est
 * pas déposer, et un dossier en évaluation ne repart pas au comité parce qu'on
 * a rectifié une faute de frappe.
 */
export function saveExistingProposal(payload: SaveDraftPayload): SaveDraftResult | null {
  const proposalId = payload.proposal_id
  if (!proposalId) return null
  const stored = allProposals.find((proposal) => proposal.id === proposalId)
  if (!stored) return null

  const now = new Date().toISOString()
  const previous = edits.get(proposalId)
  const record: EditRecord = {
    proposal_id: proposalId,
    draft: payload.draft,
    status: previous?.status ?? stored.status,
    saved_at: now,
    submitted_at: previous?.submitted_at ?? stored.submitted_at,
  }
  edits.set(proposalId, record)

  return {
    proposal_id: proposalId,
    reference_code: stored.reference_code,
    saved_at: now,
    status: record.status,
  }
}

/**
 * RENVOI AU COMITÉ d'un dossier corrigé — `changes_requested → submitted`.
 *
 * LA FENÊTRE DE L'APPEL NE S'Y APPLIQUE PAS, et c'est une correction du modèle
 * du 17/08 : le comité demande ses corrections APRÈS la clôture — c'est même le
 * cas normal, l'évaluation commençant quand l'appel se ferme. Le contrôle
 * indifférencié refusait le renvoi d'un dossier que le comité venait lui-même
 * de réclamer, et l'organisation se retrouvait bloquée devant un écran lui
 * réclamant des corrections impossibles à renvoyer.
 */
export function resubmitProposal(payload: SaveDraftPayload): SubmitProposalResult | null {
  const proposalId = payload.proposal_id
  if (!proposalId) return null
  const stored = allProposals.find((proposal) => proposal.id === proposalId)
  if (!stored) return null

  const call = callsForProposals.find((entry) => entry.id === stored.call_id) ?? null
  const now = new Date().toISOString()

  edits.set(proposalId, {
    proposal_id: proposalId,
    draft: payload.draft,
    status: 'submitted',
    saved_at: now,
    submitted_at: now,
  })

  return {
    status: 'submitted',
    proposal_id: proposalId,
    reference_code: stored.reference_code,
    submitted_at: now,
    required_reviews: call?.required_reviews ?? 0,
    results_expected_at: call?.results_expected_at ?? null,
  }
}
