import { test } from 'node:test'
import assert from 'node:assert/strict'
import { editionComplete, editionDuGuide } from '../../app/utils/guide-nego/edition.ts'

const edition = (libelle: string, series_kind: string, temporal_state: string, starts_at: string) => ({
  edition_label: libelle,
  acronym: null,
  series_kind,
  temporal_state,
  starts_at,
  slug: libelle.toLowerCase(),
  timezone: 'America/Belem',
  city: 'Belém',
}) as never

test('la COP climat qui se tient passe avant la prochaine', () => {
  assert.deepEqual(
    editionDuGuide([
      edition('COP32', 'cop_climate', 'upcoming', '2027-11-08T08:00:00Z'),
      edition('COP31', 'cop_climate', 'ongoing', '2026-11-09T08:00:00Z'),
    ]),
    { libelle: 'COP31', enCours: true, slug: 'cop31', timezone: 'America/Belem', city: 'Belém' },
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
    { libelle: 'COP31', enCours: false, slug: 'cop31', timezone: 'America/Belem', city: 'Belém' },
  )
})

test('aucune COP climat à venir : rien, et l’écran se tait', () => {
  assert.equal(editionDuGuide([edition('COP30', 'cop_climate', 'past', '2025-11-10T08:00:00Z')]), null)
})

test('une garde d’avant 3a, sans slug ni fuseau, ne désigne aucune édition', () => {
  assert.equal(editionComplete({ libelle: 'COP31', enCours: true }), false)
  assert.equal(editionComplete({ libelle: 'COP31', enCours: true, slug: 'cop31', timezone: 'America/Belem', city: null }), true)
})
