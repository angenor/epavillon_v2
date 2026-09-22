import { test } from 'node:test'
import assert from 'node:assert/strict'
import { partOccupee, placeDe, tailleLisible } from '../../app/utils/guide-nego/place.ts'
import { etatDuCompte } from '../../app/utils/guide-nego/compte.ts'

// `Intl` sépare le nombre de l'unité par une espace insécable, et c'est juste.
const lue = (octets: number, locale: string) => tailleLisible(octets, locale).replace(/\s/gu, ' ')

test('la place s’écrit comme sur le téléphone, en puissances de mille', () => {
  assert.equal(lue(7_000_000, 'fr'), '7 Mo')
  assert.equal(lue(2_140_000_000, 'fr'), '2,1 Go')
  assert.equal(lue(512, 'fr'), '512 o')
  assert.equal(lue(7_000_000, 'en'), '7 MB')
})

/** Un navigateur qui ne mesure pas ne donne jamais un zéro faux (R8). */
test('sans estimation, la place est inconnue — pas nulle', () => {
  assert.equal(placeDe(null), null)
  assert.equal(placeDe({}), null)
  assert.equal(placeDe({ usage: Number.NaN, quota: 10 }), null)
  assert.deepEqual(placeDe({ usage: 0, quota: 100 }), { utilise: 0, libre: 100 })
  assert.deepEqual(placeDe({ usage: 7 }), { utilise: 7, libre: null })
})

test('la part occupée se borne, et se tait quand la place libre est inconnue', () => {
  assert.equal(partOccupee({ utilise: 25, libre: 75 }), 0.25)
  assert.equal(partOccupee({ utilise: 0, libre: 0 }), 0)
  assert.equal(partOccupee({ utilise: 7, libre: null }), null)
  assert.deepEqual(placeDe({ usage: 120, quota: 100 }), { utilise: 120, libre: 0 }, 'jamais une place négative')
})

test('le compte gardé porte son pays, pour que le profil le nomme sans réseau', () => {
  const etat = etatDuCompte({
    id: 'awa',
    first_name: 'Awa',
    last_name: 'Diallo',
    primary_email: 'awa@example.org',
    email_verified_at: '2026-11-03T09:14:00Z',
    country_id: 'senegal',
  } as unknown as Parameters<typeof etatDuCompte>[0])
  assert.equal(etat.paysId, 'senegal')
})
