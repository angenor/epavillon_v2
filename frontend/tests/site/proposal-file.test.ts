import { test } from 'node:test'
import assert from 'node:assert/strict'
import { proposalFileChecklist, proposalFileGaps } from '../../app/utils/proposal-file.ts'
import type { DraftSpeaker, ProposalDraft } from '../../app/types/proposal-form.ts'
import type { AttachedImage } from '../../app/types/media.ts'

const photo = { asset_id: 'a', url: 'x' } as unknown as AttachedImage

const speaker = (first_name: string, bio: string, withPhoto: boolean) =>
  ({ key: first_name, first_name, last_name: 'Diop', bio, photo_image: withPhoto ? photo : null }) as unknown as DraftSpeaker

const draft = (speakers: DraftSpeaker[]) =>
  ({
    title: 'Mangroves',
    objectives: '<p>Partager</p>',
    detailed_presentation: '<p>Trois expériences</p>',
    organization_id: 'org',
    format: 'in_person',
    duration_minutes: 90,
    speakers,
  }) as unknown as ProposalDraft

test('les manques nomment les intervenants sans biographie et sans photo', () => {
  const gaps = proposalFileGaps({
    draft: draft([speaker('Aïcha', '', false), speaker('Kofi', '<p>Chef de communauté</p>', false), speaker('Mariam', '<p>Directrice</p>', true)]),
    hasCover: true,
    hasLogo: false,
  })
  assert.deepEqual(gaps, [
    { kind: 'bio', names: ['Aïcha Diop'] },
    { kind: 'photo', names: ['Aïcha Diop', 'Kofi Diop'] },
  ])
})

test("une biographie faite de balises vides compte comme absente, et la couverture manquante s'ajoute", () => {
  const gaps = proposalFileGaps({ draft: draft([speaker('Aïcha', '<p></p>', true)]), hasCover: false, hasLogo: true })
  assert.deepEqual(gaps, [{ kind: 'bio', names: ['Aïcha Diop'] }, { kind: 'cover' }])
})

test('la liste du dossier marque le logo en rouge et compte ce qui est fait', () => {
  const items = proposalFileChecklist({ draft: draft([speaker('Mariam', '<p>Directrice</p>', true)]), hasCover: true, hasLogo: false })
  const logo = items.find((item) => item.key === 'logo')
  assert.equal(logo?.done, false)
  assert.equal(logo?.danger, true)
  assert.equal(items.filter((item) => item.done).length, items.length - 1)
})
