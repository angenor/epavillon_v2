/**
 * DÉPÔT D'UNE PROPOSITION — ce que fera l'API au prompt B4, et que la base sait
 * déjà faire.
 *
 * Ce fichier ne contient AUCUNE donnée nouvelle : il rejoue en TypeScript ce que
 * `070_programme_proposals.sql` impose au moment où une organisation enregistre
 * puis dépose son dossier. Il est aux propositions ce que
 * `organization-search.ts` est aux organisations.
 *
 * TROIS RÈGLES DE LA BASE SONT REJOUÉES ICI, ET AUCUNE N'EST INVENTÉE :
 *
 *  1. LE NUMÉRO DE DOSSIER EST ATTRIBUÉ À L'INSERTION, pas à l'envoi.
 *     `tg_assign_reference_code` est un trigger `BEFORE INSERT` : il compose
 *     `<sigle de l'édition>-<numéro sur cinq chiffres>` depuis
 *     `programme.proposal_reference_seq`. Un brouillon porte donc déjà son
 *     numéro, et c'est le MÊME que celui de la confirmation d'envoi. La séquence
 *     ne recule jamais : elle repart ici après les quarante dossiers du jeu.
 *  2. LA RECEVABILITÉ SE VÉRIFIE AU DÉPÔT — `tg_check_submission_eligibility()` :
 *     l'appel doit être `open` ET dans sa fenêtre (prolongation comprise), le
 *     plafond `max_proposals_per_organization` ne doit pas être atteint, et
 *     l'organisation doit être vérifiée si l'appel l'exige. Les dossiers en
 *     `draft`, `withdrawn` et `rejected` ne comptent PAS dans le plafond : on ne
 *     pénalise pas une organisation pour un brouillon abandonné.
 *  3. LA TRANSITION `draft → submitted` POSE `submitted_at`, faute de quoi
 *     `ck_proposals_submitted_at` refuserait la ligne.
 *
 * RIEN N'EST ÉCRIT DANS LES DONNÉES SIMULÉES, qui sont en lecture seule : un
 * journal de session retient le dossier ouvert pendant la démonstration, pour
 * que l'enregistrement automatique et la reprise d'un brouillon se comportent
 * comme ils le feront. Il disparaît au rechargement de la page, ce qui est le
 * comportement d'un mock et non celui d'une base.
 */

import type { CallForProposals } from '~/types/event/call'
import type {
  PersonLookup,
  ProposalFormContext,
  SaveDraftPayload,
  SaveDraftResult,
  SubmitProposalPayload,
  SubmitProposalResult,
} from '~/types/proposal-form'
import type { Proposal } from '~/types/programme/proposal'
import { PROPOSAL_DRAFTED } from './ids'
import { callsForProposals } from './calls'
import { events } from './event'
import { organizations } from './org'
import { people } from './people'
import { memberships } from './memberships'
import { accounts } from './auth'
import { allProposals } from './proposals'

// ---------------------------------------------------------------------------
// La séquence de numérotation
// ---------------------------------------------------------------------------

/**
 * `programme.proposal_reference_seq`. Elle est GLOBALE et non par édition — un
 * numéro n'est jamais réutilisé, même d'une COP à l'autre. Elle repart au-delà
 * du plus grand numéro déjà semé : les quarante dossiers du jeu occupent
 * `COP31-00001` à `COP31-00040`.
 */
let referenceSequence = 40

function nextReferenceCode(eventId: string): string {
  const edition = events.find((e) => e.id === eventId)
  // Le repli de la fonction SQL : sigle de l'édition, à défaut les huit premiers
  // caractères du slug, à défaut `EPAV`.
  const prefix =
    edition?.acronym?.toUpperCase() ?? edition?.slug.slice(0, 8).toUpperCase() ?? 'EPAV'
  referenceSequence += 1
  return `${prefix}-${String(referenceSequence).padStart(5, '0')}`
}

// ---------------------------------------------------------------------------
// Le journal de session
// ---------------------------------------------------------------------------

interface DraftRecord {
  person_id: string
  proposal_id: string
  reference_code: string
  call_id: string
  event_id: string
  organization_id: string | null
  status: Proposal['status']
  saved_at: string
  submitted_at: string | null
  payload: SaveDraftPayload['draft']
}

/** Un seul dossier en cours par personne : celui que le formulaire tient. */
const sessionDrafts: DraftRecord[] = []

function recordOf(personId: string): DraftRecord | null {
  return sessionDrafts.find((entry) => entry.person_id === personId) ?? null
}

// ---------------------------------------------------------------------------
// Le contexte de l'écran
// ---------------------------------------------------------------------------

/**
 * Le tableau des appels, vu par son TYPE et non par ses valeurs littérales :
 * sans cette annotation, TypeScript déduit de l'unique appel ouvert que sa
 * prolongation n'est jamais nulle, et le repli `extended_until ?? closes_at` —
 * qui est la définition même de `event.effective_deadline()` — devient du code
 * mort à ses yeux.
 */
const calls: CallForProposals[] = callsForProposals

/**
 * L'ÉDITION QUI REÇOIT LES DOSSIERS AUJOURD'HUI.
 *
 * Le critère est celui du modèle — `event.is_call_open()` : statut `open` et
 * instant compris entre l'ouverture et l'échéance effective. On ne cherche pas
 * « la prochaine COP » : une édition annoncée dont l'appel n'est pas ouvert
 * n'accepte rien, et une édition en cours dont l'appel est clos non plus.
 */
export function openCallForProposals(at: number = Date.now()) {
  return (
    calls.find((call) => {
      if (call.status !== 'open') return false
      const opens = Date.parse(call.opens_at)
      const deadline = Date.parse(call.extended_until ?? call.closes_at)
      return at >= opens && at <= deadline
    }) ?? null
  )
}

/**
 * Ce que l'écran doit savoir avant d'afficher la première étape : où l'on
 * dépose, et combien de dossiers de cette organisation comptent déjà dans le
 * plafond de l'appel.
 */
export function proposalFormContext(
  personId: string,
  organizationIds: string[],
): ProposalFormContext {
  const call = openCallForProposals()
  if (!call) return { call_id: null, event_id: null, counted_proposals: 0 }

  const draft = recordOf(personId)
  const counted = allProposals.filter(
    (proposal) =>
      proposal.call_id === call.id &&
      organizationIds.includes(proposal.organization_id) &&
      proposal.deleted_at === null &&
      // Les trois statuts que le trigger écarte du décompte.
      !['draft', 'withdrawn', 'rejected'].includes(proposal.status),
  ).length

  const sessionCounted = sessionDrafts.filter(
    (entry) =>
      entry.call_id === call.id &&
      entry.status === 'submitted' &&
      entry.organization_id !== null &&
      organizationIds.includes(entry.organization_id) &&
      entry.proposal_id !== draft?.proposal_id,
  ).length

  return {
    call_id: call.id,
    event_id: call.event_id,
    counted_proposals: counted + sessionCounted,
  }
}

/** Le brouillon en cours de cette personne, pour reprendre où elle s'était arrêtée. */
export function draftProposalOf(personId: string): (SaveDraftResult & {
  draft: SaveDraftPayload['draft']
}) | null {
  const record = recordOf(personId)
  if (!record || record.status !== 'draft') return null
  return {
    proposal_id: record.proposal_id,
    reference_code: record.reference_code,
    saved_at: record.saved_at,
    status: record.status,
    draft: record.payload,
  }
}

// ---------------------------------------------------------------------------
// Les deux écritures
// ---------------------------------------------------------------------------

/**
 * Enregistrement automatique du brouillon.
 *
 * Le PREMIER appel crée la ligne : c'est lui qui attribue le numéro de dossier,
 * exactement comme l'insertion en base. Les suivants ne font que dater —
 * `tg_set_updated_at`.
 */
export function saveProposalDraft(personId: string, payload: SaveDraftPayload): SaveDraftResult {
  const now = new Date().toISOString()
  const existing = recordOf(personId)

  if (existing) {
    existing.payload = payload.draft
    existing.organization_id = payload.draft.organization_id
    existing.saved_at = now
    return {
      proposal_id: existing.proposal_id,
      reference_code: existing.reference_code,
      saved_at: now,
      status: existing.status,
    }
  }

  const record: DraftRecord = {
    person_id: personId,
    proposal_id: PROPOSAL_DRAFTED,
    reference_code: nextReferenceCode(payload.event_id),
    call_id: payload.call_id,
    event_id: payload.event_id,
    organization_id: payload.draft.organization_id,
    status: 'draft',
    saved_at: now,
    submitted_at: null,
    payload: payload.draft,
  }
  sessionDrafts.push(record)

  return {
    proposal_id: record.proposal_id,
    reference_code: record.reference_code,
    saved_at: now,
    status: 'draft',
  }
}

/**
 * Dépôt du dossier — la transition `draft → submitted`.
 *
 * LES DEUX REFUS SONT CEUX DU TRIGGER, et l'écran doit savoir les rendre : entre
 * l'ouverture du formulaire et le clic d'envoi, une échéance peut tomber ou une
 * consœur de la même organisation peut avoir déposé le quatrième dossier.
 */
export function submitProposal(
  personId: string,
  payload: SubmitProposalPayload,
): SubmitProposalResult {
  const call = calls.find((c) => c.id === payload.call_id)
  if (!call) throw new Error(`Appel ${payload.call_id} introuvable.`)

  const deadline = call.extended_until ?? call.closes_at
  const now = Date.now()
  if (call.status !== 'open' || now < Date.parse(call.opens_at) || now > Date.parse(deadline)) {
    return { status: 'call_closed', deadline }
  }

  const organizationId = payload.draft.organization_id
  if (call.max_proposals_per_organization !== null && organizationId) {
    const { counted_proposals } = proposalFormContext(personId, [organizationId])
    if (counted_proposals >= call.max_proposals_per_organization) {
      return { status: 'quota_reached', max: call.max_proposals_per_organization }
    }
  }

  if (call.requires_verified_organization && organizationId) {
    const organization = organizations.find((o) => o.id === organizationId)
    if (!organization || organization.verified_at === null) {
      // Aucun appel simulé ne l'exige ; la branche existe parce que la colonne
      // existe, et qu'un appel réservé aux organisations vérifiées est prévu.
      return { status: 'quota_reached', max: 0 }
    }
  }

  const record = recordOf(personId) ?? {
    person_id: personId,
    proposal_id: PROPOSAL_DRAFTED,
    reference_code: nextReferenceCode(payload.event_id),
    call_id: payload.call_id,
    event_id: payload.event_id,
    organization_id: organizationId,
    status: 'draft' as Proposal['status'],
    saved_at: new Date().toISOString(),
    submitted_at: null,
    payload: payload.draft,
  }
  if (!recordOf(personId)) sessionDrafts.push(record)

  const submittedAt = new Date().toISOString()
  record.payload = payload.draft
  record.status = 'submitted'
  record.submitted_at = submittedAt
  record.saved_at = submittedAt

  return {
    status: 'submitted',
    proposal_id: record.proposal_id,
    reference_code: record.reference_code,
    submitted_at: submittedAt,
    required_reviews: call.required_reviews,
    results_expected_at: call.results_expected_at,
  }
}

// ---------------------------------------------------------------------------
// Aide à la saisie des intervenants
// ---------------------------------------------------------------------------

/**
 * LA PERSONNE QUI PORTE CETTE ADRESSE, si la plateforme la connaît déjà.
 *
 * POURQUOI CETTE RECHERCHE EXISTE. `programme.proposal_speakers.person_id` est
 * NOT NULL et l'en-tête du fichier 070 est formel : « l'intervenant EST une
 * personne, créée à la volée si elle est inconnue ». Retrouver la personne au
 * lieu d'en créer une seconde, c'est le défaut n° 1 de la v1 traité une seconde
 * fois — sur les personnes cette fois, où il est bien moins visible : le même
 * expert existait en autant d'exemplaires que de participations.
 *
 * ON CHERCHE PAR ADRESSE, ET PAR ELLE SEULE. `people.primary_email` est la clé
 * de rapprochement du modèle (`citext`, donc insensible à la casse), et
 * `person_emails` porte les adresses secondaires — une inscription arrivant sur
 * l'une d'elles retrouve la bonne personne. Chercher par NOM rapprocherait deux
 * homonymes, ce qui est pire qu'un doublon : on attribuerait à quelqu'un les
 * interventions d'un autre.
 *
 * ON NE REND JAMAIS L'ANNUAIRE ENTIER : il comptera des milliers de personnes,
 * et une plateforme ne diffuse pas sa liste de contacts pour remplir un
 * formulaire. Les personnes invisibles de l'annuaire
 * (`is_directory_visible = false`) restent trouvables par leur adresse exacte —
 * qui la connaît la connaît déjà —, mais rien ne permet de les énumérer.
 */
export function lookupSpeakerByEmail(email: string): PersonLookup | null {
  const needle = email.trim().toLowerCase()
  if (needle.length === 0) return null

  const person = people.find(
    (candidate) =>
      candidate.primary_email.toLowerCase() === needle && candidate.status !== 'anonymized',
  )
  if (!person) return null

  const membership = memberships.find(
    (entry) => entry.person_id === person.id && entry.status === 'active',
  )
  const organization = membership
    ? (organizations.find((entry) => entry.id === membership.organization_id) ?? null)
    : null

  return {
    person_id: person.id,
    civility: person.civility,
    first_name: person.first_name,
    last_name: person.last_name,
    email: person.primary_email,
    // La fonction déclarée sur l'ADHÉSION prime celle du profil : c'est celle
    // que la personne exerce dans l'organisation qu'elle représenterait ici.
    job_title: membership?.job_title ?? person.job_title,
    organization_name: organization?.legal_name ?? null,
    organization_id: organization?.id ?? null,
    bio: person.biography ? (person.biography.fr ?? null) : null,
    // Un COMPTE, pas une simple fiche : c'est lui qui rend l'identité
    // intouchable par un tiers.
    has_account: accounts.some((account) => account.person_id === person.id),
  }
}
