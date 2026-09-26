import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  choixValidable,
  codesVoulus,
  etatDesThematiques,
  libelleDe,
  propositionAFaire,
  recapitulatif,
  thematiquesGardees,
} from '../../app/utils/guide-nego/thematiques.ts'

const terme = (code: string, label: Record<string, string>, ordre: number, actif = true) => ({
  id: `01930000-0000-7000-8000-0000000000${ordre}`,
  taxonomy_code: 'negotiation_theme',
  code,
  label,
  description: null,
  parent_id: null,
  sort_order: ordre,
  is_active: actif,
  created_at: '2026-09-22T09:00:00Z',
})

const VOCABULAIRE = thematiquesGardees([
  terme('gender', { fr: 'Genre', en: 'Gender' }, 70),
  terme('adaptation', { fr: 'Adaptation', en: 'Adaptation' }, 10),
  terme('finance', { fr: 'Finance', en: 'Finance' }, 30),
  terme('retiree', { fr: 'Retirée', en: 'Withdrawn' }, 40, false),
])

test('le vocabulaire ne garde que les termes actifs, dans leur ordre', () => {
  assert.deepEqual(
    VOCABULAIRE.map((t) => t.code),
    ['adaptation', 'finance', 'gender'],
    'une thématique désactivée cesse d’être proposée (FR-008)',
  )
})

test('le libellé suit la langue, et retombe sur le français', () => {
  const adaptation = VOCABULAIRE[0]!
  assert.equal(libelleDe(adaptation, 'en'), 'Adaptation')
  assert.equal(libelleDe({ ...adaptation, label: { fr: 'Genre' } }, 'en'), 'Genre', 'repli FR-002')
  assert.equal(libelleDe({ ...adaptation, label: {} }, 'en'), 'adaptation', 'le code en dernier recours')
})

/** **Zéro coché n'est pas validable** (FR-005), à la première entrée comme après. */
test('la validation se refuse à zéro thématique', () => {
  assert.equal(choixValidable([]), false)
  assert.equal(choixValidable(['adaptation']), true)
})

test('les codes envoyés sont uniques, triés, sans blanc', () => {
  assert.deepEqual(codesVoulus([' gender ', 'adaptation', 'gender', '']), ['adaptation', 'gender'])
})

test('le récapitulatif compte juste, et nomme dans l’ordre du vocabulaire', () => {
  const resume = recapitulatif(['gender', 'adaptation'], VOCABULAIRE, 'fr')
  assert.equal(resume.nombre, 2)
  assert.deepEqual(resume.noms, ['Adaptation', 'Genre'])

  const vide = recapitulatif([], VOCABULAIRE, 'fr')
  assert.equal(vide.nombre, 0)
  assert.deepEqual(vide.noms, [])
})

test('un code sans libellé connu ne fait pas paraître de code brut', () => {
  const resume = recapitulatif(['inconnue'], VOCABULAIRE, 'fr')
  assert.equal(resume.nombre, 1, 'le nombre reste juste')
  assert.deepEqual(resume.noms, [], 'le profil dira « 1 thématique suivie »')
})

test('l’état lu garde les codes triés et l’empreinte', () => {
  const etat = etatDesThematiques(
    {
      themes: [
        { code: 'gender', followed_at: '2026-11-12T10:00:00Z' },
        { code: 'adaptation', followed_at: '2026-11-12T10:00:00Z' },
        { code: 'gender', followed_at: '2026-11-12T10:00:00Z' },
      ],
    },
    '"a1b2"',
  )
  assert.deepEqual(etat.codes, ['adaptation', 'gender'])
  assert.equal(etat.empreinte, '"a1b2"')
  assert.deepEqual(etatDesThematiques(null, null), { codes: [], notify: [], empreinte: null })
})

/**
 * **La garde de première entrée ne se déclenche qu'une fois**, et jamais avant
 * que l'état ne soit lu : proposer l'écran à qui suit déjà des thématiques, le
 * temps d'une lecture, serait la marche imposée à contretemps.
 */
test('l’écran de premier choix se propose une seule fois', () => {
  const neuf = { connectee: true, pret: true, nombreSuivi: 0, dejaProposee: false }
  assert.equal(propositionAFaire(neuf), true)
  assert.equal(propositionAFaire({ ...neuf, dejaProposee: true }), false, 'déjà proposé')
  assert.equal(propositionAFaire({ ...neuf, nombreSuivi: 2 }), false, 'elle suit déjà')
  assert.equal(propositionAFaire({ ...neuf, pret: false }), false, 'rien n’est encore lu')
  assert.equal(propositionAFaire({ ...neuf, connectee: false }), false, 'sans compte, jamais (FR-010)')
})
