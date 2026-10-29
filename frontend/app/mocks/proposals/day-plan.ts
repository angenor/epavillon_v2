/**
 * Données simulées du jour J d'une activité retenue (`070` § 4 bis).
 *
 * Mêmes règles que l'API, pour que la fiche se teste hors ligne : déposant
 * proposé tant que rien n'est enregistré, liste jamais vide, une publication
 * par adresse, date limite dix jours avant la première séance non annulée.
 * Les gardes d'accès ne sont pas simulées (pas de session hors ligne).
 *
 * Amorce : le dossier « adaptation côtière » (retenu, programmé) a son déposant
 * par défaut, aucun besoin déclaré et une publication LinkedIn.
 */

import type { ProposalDocument } from '~/types/programme/proposal'
import type { AttachDocumentPayload } from '~/types/admin-review'
import type {
  AddSocialPostPayload,
  FocalPoint,
  ProposalDayPlan,
  SaveFocalPointsPayload,
  SaveTechnicalNeedsPayload,
  SocialPlatform,
  SocialPost,
  TechnicalNeeds,
} from '~/types/programme/day-plan'
import type { ApiErrorCode } from '~/types/api-error'
import type { Uuid } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'
import { PROPOSAL, PROPOSAL_SOCIAL_POST } from '../ids'
import { memberships } from '../memberships'
import { people } from '../people'
import { allSessions } from '../sessions'
import { allProposals } from './index'
import { proposalDocuments } from './documents'
import { proposalSpeakers } from './speakers'

interface DayPlanState {
  focal_points: FocalPoint[] | null
  technical_needs: TechnicalNeeds | null
  social_posts: SocialPost[]
}

const states = new Map<Uuid, DayPlanState>([
  [
    PROPOSAL.adaptationCotiere,
    {
      focal_points: null,
      technical_needs: null,
      social_posts: [
        {
          id: PROPOSAL_SOCIAL_POST(1),
          url: 'https://www.linkedin.com/posts/roac-afrique_cop31-adaptation-cotiere-activity-1',
          platform: 'linkedin',
          created_at: '2026-10-20T09:30:00Z',
        },
      ],
    },
  ],
])

function stateOf(proposalId: Uuid): DayPlanState {
  let state = states.get(proposalId)
  if (!state) {
    state = { focal_points: null, technical_needs: null, social_posts: [] }
    states.set(proposalId, state)
  }
  return state
}

function refuse(status: number, code: ApiErrorCode, message: string, field?: string): never {
  throw new ApiRequestError({ code, message, field }, status)
}

const PLATFORMS: [string, SocialPlatform][] = [
  ['linkedin.com', 'linkedin'],
  ['lnkd.in', 'linkedin'],
  ['facebook.com', 'facebook'],
  ['fb.com', 'facebook'],
  ['fb.watch', 'facebook'],
  ['fb.me', 'facebook'],
  ['x.com', 'x'],
  ['twitter.com', 'x'],
  ['t.co', 'x'],
  ['instagram.com', 'instagram'],
  ['instagr.am', 'instagram'],
  ['youtube.com', 'youtube'],
  ['youtu.be', 'youtube'],
  ['tiktok.com', 'tiktok'],
]

function hostOf(url: string): string | null {
  try {
    const parsed = new URL(url)
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return null
    return parsed.hostname.includes('.') ? parsed.hostname.toLowerCase() : null
  } catch {
    return null
  }
}

function platformOf(url: string): SocialPlatform {
  const host = hostOf(url)
  if (!host) return 'other'
  return PLATFORMS.find(([domain]) => host === domain || host.endsWith(`.${domain}`))?.[1] ?? 'other'
}

function submitterOf(proposalId: Uuid): FocalPoint | null {
  const proposal = allProposals.find((p) => p.id === proposalId)
  const person = proposal ? people.find((p) => p.id === proposal.submitted_by) : undefined
  if (!proposal || !person) return null
  const speaker = proposalSpeakers.find((s) => s.proposal_id === proposalId && s.person_id === person.id)
  return {
    id: null,
    speaker_id: speaker?.id ?? null,
    person_id: person.id,
    full_name: person.display_name,
    phone: person.phone ?? '',
    whatsapp: '',
    is_default: true,
  }
}

function deadlineOf(proposalId: Uuid): string | null {
  const starts = allSessions
    .filter((s) => s.proposal_id === proposalId && s.status !== 'cancelled')
    .map((s) => Date.parse(s.starts_at))
  if (starts.length === 0) return null
  return new Date(Math.min(...starts) - 10 * 24 * 3600 * 1000).toISOString()
}

export function proposalDayPlan(proposalId: Uuid): ProposalDayPlan {
  const state = stateOf(proposalId)
  const submitter = submitterOf(proposalId)
  return {
    focal_points: state.focal_points ?? (submitter ? [submitter] : []),
    technical_needs: state.technical_needs ?? {
      video: false,
      remote: false,
      remote_details: '',
      accessibility: false,
      accessibility_details: '',
      other: '',
      updated_at: null,
    },
    social_posts: [...state.social_posts],
    needs_deadline: deadlineOf(proposalId),
  }
}

export function saveProposalFocalPoints(proposalId: Uuid, payload: SaveFocalPointsPayload): ProposalDayPlan {
  if (payload.focal_points.length === 0) {
    refuse(422, 'VALIDATION_FAILED', "Gardez au moins une personne à joindre sur place le jour de l'activité.", 'focal_points')
  }
  const proposal = allProposals.find((p) => p.id === proposalId)
  const resolved = payload.focal_points.map((input, index): FocalPoint => {
    const speaker = input.speaker_id ? proposalSpeakers.find((s) => s.id === input.speaker_id && s.proposal_id === proposalId) : undefined
    if (input.speaker_id && !speaker) {
      refuse(422, 'PROPOSAL_UNKNOWN_REFERENCE', 'Cet intervenant ne figure pas dans ce dossier.', `focal_points[${index}].speaker_id`)
    }
    const personId = speaker ? speaker.person_id : input.person_id
    const person = personId ? people.find((p) => p.id === personId) : undefined
    const known =
      !input.person_id ||
      speaker !== undefined ||
      input.person_id === proposal?.submitted_by ||
      input.person_id === proposal?.contact_person_id ||
      memberships.some((m) => m.person_id === input.person_id && m.organization_id === proposal?.organization_id && m.status === 'active')
    if (!known) {
      refuse(
        422,
        'PROPOSAL_UNKNOWN_REFERENCE',
        "Cette personne n'est ni le déposant, ni le contact, ni un membre de l'organisation.",
        `focal_points[${index}].person_id`,
      )
    }
    const speakerName = speaker && !person ? `${speaker.first_name ?? ''} ${speaker.last_name ?? ''}`.trim() : ''
    const fullName = input.full_name.trim() || person?.display_name || speakerName
    if (!fullName) {
      refuse(422, 'VALIDATION_FAILED', 'Indiquez le nom de la personne à joindre sur place.', `focal_points[${index}].full_name`)
    }
    return {
      id: crypto.randomUUID(),
      speaker_id: speaker?.id ?? null,
      person_id: personId ?? null,
      full_name: fullName,
      phone: input.phone.trim() || person?.phone || '',
      whatsapp: input.whatsapp.trim(),
      is_default: false,
    }
  })
  resolved.forEach((point, index) => {
    const twice = resolved
      .slice(0, index)
      .some((before) => (point.speaker_id && before.speaker_id === point.speaker_id) || (point.person_id && before.person_id === point.person_id))
    if (twice) {
      refuse(422, 'VALIDATION_FAILED', `${point.full_name} figure deux fois parmi les personnes à joindre.`, `focal_points[${index}]`)
    }
  })
  stateOf(proposalId).focal_points = resolved
  return proposalDayPlan(proposalId)
}

export function saveProposalTechnicalNeeds(proposalId: Uuid, payload: SaveTechnicalNeedsPayload): ProposalDayPlan {
  stateOf(proposalId).technical_needs = {
    video: payload.video,
    remote: payload.remote,
    remote_details: payload.remote ? payload.remote_details.trim() : '',
    accessibility: payload.accessibility,
    accessibility_details: payload.accessibility ? payload.accessibility_details.trim() : '',
    other: payload.other.trim(),
    updated_at: new Date().toISOString(),
  }
  return proposalDayPlan(proposalId)
}

export function addProposalSocialPost(proposalId: Uuid, payload: AddSocialPostPayload): ProposalDayPlan {
  const url = payload.url.trim()
  if (!hostOf(url)) {
    refuse(422, 'VALIDATION_FAILED', "Collez l'adresse complète de la publication, commençant par https://.", 'url')
  }
  const state = stateOf(proposalId)
  if (state.social_posts.some((post) => post.url === url)) {
    refuse(409, 'CONFLICT', "Ce lien figure déjà parmi les publications de l'activité.", 'url')
  }
  state.social_posts.push({ id: crypto.randomUUID(), url, platform: platformOf(url), created_at: new Date().toISOString() })
  return proposalDayPlan(proposalId)
}

export function removeProposalSocialPost(proposalId: Uuid, postId: Uuid): ProposalDayPlan {
  const state = stateOf(proposalId)
  const before = state.social_posts.length
  state.social_posts = state.social_posts.filter((post) => post.id !== postId)
  if (state.social_posts.length === before) refuse(404, 'NOT_FOUND', 'La ressource demandée est introuvable.')
  return proposalDayPlan(proposalId)
}

// ---------------------------------------------------------------------------
// Les pièces du dossier : rattacher, détacher (l'objet stocké demeure)
// ---------------------------------------------------------------------------

export function attachProposalDocument(proposalId: Uuid, payload: AttachDocumentPayload, actorId: Uuid | null): ProposalDocument {
  if (payload.presented_by_speaker_id && !proposalSpeakers.some((s) => s.id === payload.presented_by_speaker_id && s.proposal_id === proposalId)) {
    refuse(422, 'PROPOSAL_UNKNOWN_REFERENCE', 'Cet intervenant ne figure pas dans ce dossier.', 'presented_by_speaker_id')
  }
  const document: ProposalDocument = {
    id: crypto.randomUUID(),
    proposal_id: proposalId,
    asset_id: payload.asset_id,
    title: { fr: payload.title.trim() || 'Pièce jointe' },
    document_type_code: payload.document_type_code,
    is_public: payload.is_public,
    uploaded_by: actorId,
    uploaded_at: new Date().toISOString(),
    sort_order: payload.sort_order,
    presented_by_speaker_id: payload.presented_by_speaker_id ?? null,
  }
  proposalDocuments.push(document)
  return document
}

export function removeProposalDocument(proposalId: Uuid, documentId: Uuid): void {
  const index = proposalDocuments.findIndex((d) => d.id === documentId && d.proposal_id === proposalId)
  if (index < 0) refuse(404, 'NOT_FOUND', 'La ressource demandée est introuvable.')
  proposalDocuments.splice(index, 1)
}
