/**
 * Les termes proposés, sans API : une réserve en mémoire que le téléphone et la file
 * des experts partagent. Une proposition du même terme normalisé s'ajoute à celle qui
 * attend ; le mock n'a qu'une personne, donc un seul auteur par proposition.
 */
import type { AdminProposal, AdminProposalNearby } from '~/types/admin-negotiation-queue'
import type { ProposalInput, ProposalReceipt } from '~/types/negotiation-savoir'
import { ApiRequestError } from '~/utils/api-error'
import { normaliserTerme, resoudreLeTerme } from '~/utils/guide-nego/lexique'
import { LEXIQUE } from './negotiation-savoir'

interface Proposition extends AdminProposal {
  refs: string[]
}

let propositions: Proposition[] = []

const maintenant = () => new Date().toISOString()

export function proposerUnTerme(entree: ProposalInput): ProposalReceipt {
  const rejouee = propositions.find((p) => p.refs.includes(entree.client_ref))
  if (rejouee) return { id: rejouee.id, client_ref: entree.client_ref, term: rejouee.term, created_at: rejouee.created_at }
  const term = entree.term.trim()
  if (!term) throw new ApiRequestError({ code: 'VALIDATION_FAILED', message: 'Écrivez le terme.', field: 'term' }, 422)
  const contexte = entree.context?.trim() || null
  if (contexte && [...contexte].length > 600) {
    throw new ApiRequestError({ code: 'NEGOTIATION_TEXT_TOO_LONG', message: 'Ce texte dépasse 600 caractères.', field: 'context' }, 422)
  }
  if (resoudreLeTerme(term, LEXIQUE)) {
    throw new ApiRequestError({ code: 'NEGOTIATION_GLOSSARY_TERM_EXISTS', message: 'Ce terme est déjà dans le lexique.' }, 409)
  }
  const norme = normaliserTerme(term)
  const created_at = maintenant()
  const attente = propositions.find((p) => p.status === 'pending' && normaliserTerme(p.term) === norme)
  if (attente) {
    attente.refs.push(entree.client_ref)
    return { id: attente.id, client_ref: entree.client_ref, term: attente.term, created_at }
  }
  const p: Proposition = {
    id: crypto.randomUUID(),
    term,
    status: 'pending',
    authors_count: 1,
    contexts: [{ context: contexte, created_at }],
    nearby: prochesDe(norme),
    glossary_entry_id: null,
    glossary_entry_slug: null,
    rejection_reason: null,
    created_at,
    handled_at: null,
    refs: [entree.client_ref],
  }
  propositions = [...propositions, p]
  return { id: p.id, client_ref: entree.client_ref, term, created_at }
}

/** Un mot en commun suffit ici ; l'API compare par trigrammes. */
function prochesDe(norme: string): AdminProposalNearby[] {
  const mots = new Set(norme.split(' ').filter((m) => m.length > 3))
  return LEXIQUE.filter((e) => normaliserTerme(e.term).split(' ').some((m) => mots.has(m)))
    .slice(0, 5)
    .map((e) => ({ id: e.id, slug: e.slug, term: e.term, status: e.status, similarity: 0.4 }))
}

const versLaFile = ({ refs: _r, ...p }: Proposition): AdminProposal => structuredClone(p)

export function propositionsDeLaFile(): AdminProposal[] {
  return propositions.filter((p) => p.status === 'pending').map(versLaFile)
}

export const propositionsEnAttente = (): number => propositions.filter((p) => p.status === 'pending').length

function propositionOuRefus(id: string): Proposition {
  const p = propositions.find((x) => x.id === id)
  if (!p) throw new ApiRequestError({ code: 'NOT_FOUND', message: "Cette proposition n'existe pas." }, 404)
  return p
}

export const proposition = (id: string): AdminProposal => versLaFile(propositionOuRefus(id))

function enAttenteOuRefus(id: string): Proposition {
  const p = propositionOuRefus(id)
  if (p.status !== 'pending') {
    throw new ApiRequestError({ code: 'NEGOTIATION_QUEUE_ITEM_CLOSED', message: 'Cet élément de la file a déjà été traité.' }, 409)
  }
  return p
}

/** Le terme à reprendre dans le brouillon ; le brouillon naît dans le lexique d'exemple. */
export const aAccepter = (id: string): string => enAttenteOuRefus(id).term

export function marquerAcceptee(id: string, entree: { id: string; slug: string }): void {
  Object.assign(propositionOuRefus(id), {
    status: 'accepted',
    glossary_entry_id: entree.id,
    glossary_entry_slug: entree.slug,
    handled_at: maintenant(),
  })
}

export function rejeterUneProposition(id: string, reason: string): AdminProposal {
  const p = enAttenteOuRefus(id)
  if (!reason.trim()) throw new ApiRequestError({ code: 'VALIDATION_FAILED', message: 'Indiquez le motif du refus.', field: 'reason' }, 422)
  Object.assign(p, { status: 'rejected', rejection_reason: reason.trim(), handled_at: maintenant() })
  return versLaFile(p)
}
