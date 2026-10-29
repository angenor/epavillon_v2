import type { DraftSpeaker, ProposalDraft } from '~/types/proposal-form'
import type { ProposalDayPlan } from '~/types/programme/day-plan'
import { plainTextOf } from './proposal-draft.ts'

/**
 * CE QUI MANQUE À UN DOSSIER, vu de l'organisation (28/10).
 *
 * La fiche le dit en tête, en mots simples, avec un lien vers la rubrique : les
 * personnes qui la remplissent ne sont pas à l'aise avec les formulaires, et un
 * champ vide au fond d'une carte repliée ne se voit pas. Le logo manquant est à
 * part, en rouge : il manque sur la page publique comme sur l'image.
 */
export interface ProposalFileFacts {
  draft: ProposalDraft
  hasCover: boolean
  hasLogo: boolean
  /** Activité retenue seulement : ce que le jour J demande en plus. */
  dayPlan?: ProposalDayPlan | null
}

export type ProposalFileGap =
  | { kind: 'bio'; names: string[] }
  | { kind: 'photo'; names: string[] }
  | { kind: 'cover' }
  | { kind: 'focalPhone'; names: string[] }
  | { kind: 'needs' }

export function speakerName(speaker: Pick<DraftSpeaker, 'first_name' | 'last_name'>): string {
  return `${speaker.first_name} ${speaker.last_name}`.trim()
}

const hasPhoto = (speaker: DraftSpeaker): boolean => Boolean(speaker.photo_image)

const hasBio = (speaker: DraftSpeaker): boolean => plainTextOf(speaker.bio).trim().length > 0

/** Les manques ordinaires, dans l'ordre où la fiche les présente ; le logo se lit à part. */
export function proposalFileGaps(facts: ProposalFileFacts): ProposalFileGap[] {
  const gaps: ProposalFileGap[] = []
  const withoutBio = facts.draft.speakers.filter((speaker) => !hasBio(speaker)).map(speakerName)
  if (withoutBio.length > 0) gaps.push({ kind: 'bio', names: withoutBio })
  const withoutPhoto = facts.draft.speakers.filter((speaker) => !hasPhoto(speaker)).map(speakerName)
  if (withoutPhoto.length > 0) gaps.push({ kind: 'photo', names: withoutPhoto })
  if (!facts.hasCover) gaps.push({ kind: 'cover' })
  const plan = facts.dayPlan
  if (plan) {
    const withoutPhone = plan.focal_points.filter((point) => !point.phone.trim()).map((point) => point.full_name)
    if (withoutPhone.length > 0) gaps.push({ kind: 'focalPhone', names: withoutPhone })
    if (plan.technical_needs.updated_at === null) gaps.push({ kind: 'needs' })
  }
  return gaps
}

export type ProposalFileChecklistKey =
  | 'presentation'
  | 'cover'
  | 'organizations'
  | 'speakers'
  | 'bios'
  | 'photos'
  | 'logo'
  | 'format'
  | 'focal'
  | 'needs'

export interface ProposalFileChecklistItem {
  key: ProposalFileChecklistKey
  done: boolean
  /** Rouge plutôt que jaune : le logo seulement. */
  danger: boolean
  /** Ancre de la rubrique qui le règle. */
  anchor: string
}

export function proposalFileChecklist(facts: ProposalFileFacts): ProposalFileChecklistItem[] {
  const { draft } = facts
  const item = (key: ProposalFileChecklistKey, done: boolean, anchor: string, danger = false) => ({
    key,
    done,
    danger,
    anchor,
  })
  const filled = (value: string) => plainTextOf(value).trim().length > 0
  return [
    item('presentation', filled(draft.title) && filled(draft.objectives) && filled(draft.detailed_presentation), 'presentation'),
    item('cover', facts.hasCover, 'couverture'),
    item('organizations', draft.organization_id !== null, 'organisations'),
    item('speakers', draft.speakers.length > 0, 'intervenants'),
    item('bios', draft.speakers.length > 0 && draft.speakers.every(hasBio), 'intervenants'),
    item('photos', draft.speakers.length > 0 && draft.speakers.every(hasPhoto), 'intervenants'),
    item('logo', facts.hasLogo, 'logo', true),
    item('format', draft.format !== null && draft.duration_minutes !== null, 'format'),
    ...(facts.dayPlan
      ? [
          item('focal', facts.dayPlan.focal_points.some((point) => point.phone.trim().length > 0), 'point-focal'),
          item('needs', facts.dayPlan.technical_needs.updated_at !== null, 'besoins-techniques'),
        ]
      : []),
  ]
}
