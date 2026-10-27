import { test } from 'node:test'
import assert from 'node:assert/strict'
import { emptyDraftSpeaker, emptyProposalDraft, validateProposalDraft } from '../../app/utils/proposal-draft.ts'
import type { CallForProposals } from '../../app/types/event/call.ts'
import type { EventEdition } from '../../app/types/event/edition.ts'
import type { DraftSpeaker } from '../../app/types/proposal-form.ts'

const call = {
  min_speakers: 1,
  max_speakers: 6,
  allowed_formats: ['panel'],
  min_duration_minutes: 30,
  max_duration_minutes: 120,
  daily_start_time: '09:00:00',
  daily_end_time: '18:00:00',
} as unknown as CallForProposals
const edition = { timezone: 'America/Belem', starts_at: '', ends_at: '' } as unknown as EventEdition

function speaker(email: string): DraftSpeaker {
  return {
    ...emptyDraftSpeaker(0),
    civility: 'mme',
    first_name: 'Aïcha',
    last_name: 'Traoré',
    email,
    job_title: 'Conseillère budgétaire',
    organization_name: 'Ministère des Finances',
  }
}

function speakerIssues(...speakers: DraftSpeaker[]) {
  const draft = { ...emptyProposalDraft({}), speakers }
  return validateProposalDraft(draft, call, edition).filter((issue) => issue.step === 'speakers')
}

test('un intervenant sans adresse est recevable (D16)', () => {
  assert.deepEqual(speakerIssues(speaker('')), [])
  assert.deepEqual(speakerIssues(speaker('   ')), [])
})

test('une adresse saisie doit être bien formée', () => {
  const issues = speakerIssues(speaker('aicha.traore@'))
  assert.equal(issues.length, 1)
  assert.equal(issues[0]?.messageKey, 'proposal.form.step-speakers.errors.emailInvalid')
  assert.deepEqual(speakerIssues(speaker('aicha.traore@finances.gouv.ml')), [])
})

test('deux intervenants sans adresse ne sont pas des doublons', () => {
  assert.deepEqual(speakerIssues(speaker(''), { ...speaker(''), key: 'autre' }), [])
})

test('sans adresse, prénom et nom restent exigés', () => {
  const issues = speakerIssues({ ...speaker(''), last_name: ' ' })
  assert.deepEqual(issues.map((issue) => issue.messageKey), ['proposal.form.step-speakers.errors.nameRequired'])
})
