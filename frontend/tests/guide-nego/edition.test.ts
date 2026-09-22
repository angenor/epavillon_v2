import { test } from 'node:test'
import assert from 'node:assert/strict'
import { editionDuGuide } from '../../app/utils/guide-nego/edition.ts'

const edition = (libelle: string, series_kind: string, temporal_state: string, starts_at: string) => ({
  edition_label: libelle,
  acronym: null,
  series_kind,
  temporal_state,
  starts_at,
}) as never

test('la COP climat qui se tient passe avant la prochaine', () => {
  assert.deepEqual(
    editionDuGuide([
      edition('COP32', 'cop_climate', 'upcoming', '2027-11-08T08:00:00Z'),
      edition('COP31', 'cop_climate', 'ongoing', '2026-11-09T08:00:00Z'),
    ]),
    { libelle: 'COP31', enCours: true },
  )
})

test('sinon la plus proche des prochaines, et jamais une autre série', () => {
  assert.deepEqual(
    editionDuGuide([
      edition('Webinaire', 'webinar_series', 'ongoing', '2026-09-01T08:00:00Z'),
      edition('COP16 biodiversité', 'cop_biodiversity', 'upcoming', '2026-10-01T08:00:00Z'),
      edition('COP32', 'cop_climate', 'upcoming', '2027-11-08T08:00:00Z'),
      edition('COP31', 'cop_climate', 'upcoming', '2026-11-09T08:00:00Z'),
      edition('COP30', 'cop_climate', 'past', '2025-11-10T08:00:00Z'),
    ]),
    { libelle: 'COP31', enCours: false },
  )
})

test('aucune COP climat à venir : rien, et l’écran se tait', () => {
  assert.equal(editionDuGuide([edition('COP30', 'cop_climate', 'past', '2025-11-10T08:00:00Z')]), null)
})
