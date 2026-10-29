/**
 * Données simulées de l'atelier de l'image de couverture (`070` § 4 ter).
 *
 * Mêmes règles que l'API : choix nuls tant que rien n'est enregistré, codes
 * de la forme `^[a-z][a-z0-9-]{1,40}$`, dossier rejeté, retiré ou annulé
 * refusé. Les images sont lues comme `media.attached_image()` les rend. Les
 * gardes d'accès ne sont pas simulées (pas de session hors ligne).
 */

import type {
  CoverDesign,
  CoverStudioContext,
  SaveCoverDesignPayload,
} from '~/types/programme/cover-design'
import type { ApiErrorCode } from '~/types/api-error'
import type { Uuid } from '~/types/shared'
import { ApiRequestError } from '~/utils/api-error'
import { attachedImage } from '../covers'
import { allProposals } from './index'

const designs = new Map<Uuid, CoverDesign>()

const CODE = /^[a-z][a-z0-9-]{1,40}$/

const CLOSED: Partial<Record<string, string>> = {
  rejected: "Ce dossier n'a pas été retenu : son image de couverture ne se modifie plus.",
  withdrawn: 'Ce dossier a été retiré : son image de couverture ne se modifie plus.',
  cancelled: 'Cette activité a été annulée : son image de couverture ne se modifie plus.',
}

function refuse(status: number, code: ApiErrorCode, message: string, field?: string): never {
  throw new ApiRequestError({ code, message, field }, status)
}

function proposalOf(proposalId: Uuid) {
  return allProposals.find((p) => p.id === proposalId) ?? refuse(404, 'NOT_FOUND', 'La ressource demandée est introuvable.')
}

export function proposalCoverStudio(proposalId: Uuid): CoverStudioContext {
  const proposal = proposalOf(proposalId)
  return {
    design: designs.get(proposalId) ?? null,
    source_photo: attachedImage('programme', 'proposals', proposal.id, 'cover_source'),
    edition_cover: attachedImage('event', 'events', proposal.event_id, 'cover'),
    edition_banner: attachedImage('event', 'events', proposal.event_id, 'banner'),
    edition_thumbnail: attachedImage('event', 'events', proposal.event_id, 'thumbnail'),
    organization_logo: attachedImage('org', 'organizations', proposal.organization_id, 'logo'),
  }
}

export function saveProposalCoverDesign(proposalId: Uuid, payload: SaveCoverDesignPayload): CoverStudioContext {
  const proposal = proposalOf(proposalId)
  const closed = CLOSED[proposal.status]
  if (closed) refuse(422, 'PROPOSAL_NOT_EDITABLE', closed)

  const templateCode = payload.template_code.trim()
  if (!CODE.test(templateCode)) {
    refuse(422, 'VALIDATION_FAILED', "Choisissez un modèle d'image dans la liste.", 'template_code')
  }
  const accent = payload.accent.trim()
  if (!CODE.test(accent)) {
    refuse(422, 'VALIDATION_FAILED', 'Choisissez une couleur dans la liste.', 'accent')
  }

  designs.set(proposalId, {
    template_code: templateCode,
    accent,
    options: {
      show_date: payload.options.show_date === true,
      show_speakers: payload.options.show_speakers === true,
      show_logos: payload.options.show_logos === true,
      show_event_banner: payload.options.show_event_banner === true,
      show_event_icon: payload.options.show_event_icon === true,
    },
    updated_at: new Date().toISOString(),
  })
  return proposalCoverStudio(proposalId)
}
