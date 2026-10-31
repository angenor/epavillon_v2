/**
 * Le tableau de bord d'un dossier (01/11), hors ligne : la part `dashboard` de
 * `GET /proposals/{id}/review-desk`, et les deux rattachements (porteuse,
 * déposant), gardés dans un tampon de session comme les autres écritures.
 */

import type {
  ChangeLeadOrganizationPayload,
  ChangeSubmitterPayload,
  OrganizationOtherProposal,
  ProposalContactPerson,
  ProposalDashboard,
  ProposalSessionSlot,
} from '~/types/admin-review'
import type { Proposal } from '~/types/programme/proposal'
import type { PersonId, Uuid } from '~/types/shared'
import { events } from './event'
import { organizations } from './org'
import { people } from './people'
import { allProposals } from './proposals'
import { registrations } from './registrations'
import { rooms } from './rooms'
import { allSessions, sessionTracks } from './sessions'
import { programmeTracks } from './tracks'

const reassigned = new Map<Uuid, Partial<Pick<Proposal, 'organization_id' | 'submitted_by'>>>()

/** Le dossier tel que la session de démonstration l'a rattaché. */
export function withReassignment(proposal: Proposal): Proposal {
  const patch = reassigned.get(proposal.id)
  return patch ? { ...proposal, ...patch } : proposal
}

function contactOf(personId: PersonId | null): ProposalContactPerson | null {
  const person = personId ? people.find((entry) => entry.id === personId) : undefined
  if (!person) return null
  const organization = organizations.find((entry) => entry.id === person.primary_organization_id)
  return {
    person_id: person.id,
    display_name: person.display_name,
    job_title: person.job_title,
    organization_name: organization?.legal_name ?? null,
    email: person.primary_email,
    photo: null,
  }
}

function slotsOf(proposalId: Uuid): ProposalSessionSlot[] {
  return allSessions
    .filter((session) => session.proposal_id === proposalId)
    .sort((a, b) => a.sequence_number - b.sequence_number)
    .map((session) => {
      const room = rooms.find((entry) => entry.id === session.room_id)
      const own = registrations.filter((entry) => entry.session_id === session.id)
      return {
        id: session.id,
        sequence_number: session.sequence_number,
        starts_at: session.starts_at,
        ends_at: session.ends_at,
        timezone: session.timezone,
        status: session.status,
        published_at: session.published_at,
        room_id: session.room_id,
        room_name: room?.name ?? null,
        is_streamed: session.is_streamed,
        tracks: sessionTracks
          .filter((link) => link.session_id === session.id)
          .flatMap((link) => {
            const track = programmeTracks.find((entry) => entry.id === link.track_id)
            return track ? [{ id: track.id, title: track.title, color_hex: track.color_hex }] : []
          }),
        registration_count: own.filter((entry) => ['registered', 'attended', 'no_show'].includes(entry.status)).length,
        waitlist_count: own.filter((entry) => entry.status === 'waitlisted').length,
        meeting: null,
      }
    })
}

/** `eventIds` nul : périmètre global. */
export function proposalDeskDashboard(
  proposal: Proposal,
  speakerCount: number,
  documentCount: number,
  eventIds: Uuid[] | null,
): ProposalDashboard {
  const sessions = slotsOf(proposal.id)
  const lead = organizations.find((entry) => entry.id === proposal.organization_id)

  const others: OrganizationOtherProposal[] = allProposals
    .map(withReassignment)
    .filter(
      (entry) =>
        entry.organization_id === proposal.organization_id &&
        entry.id !== proposal.id &&
        entry.deleted_at === null &&
        (eventIds === null || eventIds.includes(entry.event_id)),
    )
    .sort((a, b) => b.created_at.localeCompare(a.created_at))
    .map((entry) => {
      const edition = events.find((event) => event.id === entry.event_id)
      const placed = slotsOf(entry.id).filter((slot) => slot.room_id !== null && slot.status !== 'cancelled')
      return {
        id: entry.id,
        reference_code: entry.reference_code,
        title: entry.title,
        status: entry.status,
        created_at: entry.created_at,
        event_id: entry.event_id,
        event_title: edition?.title ?? { fr: '' },
        event_acronym: edition?.acronym ?? null,
        timezone: edition?.timezone ?? 'UTC',
        city: edition?.city ?? null,
        retained_starts_at: placed[0]?.starts_at ?? null,
        submitter: contactOf(entry.submitted_by),
      }
    })

  return {
    sessions,
    submitter: contactOf(proposal.submitted_by),
    contact: proposal.contact_person_id && proposal.contact_person_id !== proposal.submitted_by
      ? contactOf(proposal.contact_person_id)
      : null,
    lead_organization_id: proposal.organization_id,
    lead_logo: null,
    lead_type_label: null,
    lead_country_name: null,
    other_proposals: others,
    counts: {
      registrations: sessions.reduce((sum, slot) => sum + slot.registration_count, 0),
      speakers: speakerCount,
      documents: documentCount,
    },
  }
}

function reassign(
  proposalId: Uuid,
  patch: Partial<Pick<Proposal, 'organization_id' | 'submitted_by'>>,
): Proposal {
  const base = allProposals.find((entry) => entry.id === proposalId)
  if (!base) throw new Error(`Dossier ${proposalId} introuvable.`)
  reassigned.set(proposalId, { ...reassigned.get(proposalId), ...patch })
  return withReassignment(base)
}

export function changeLeadOrganization(proposalId: Uuid, payload: ChangeLeadOrganizationPayload): Proposal {
  return reassign(proposalId, { organization_id: payload.organization_id })
}

export function changeSubmitter(proposalId: Uuid, payload: ChangeSubmitterPayload): Proposal {
  return reassign(proposalId, { submitted_by: payload.person_id })
}
