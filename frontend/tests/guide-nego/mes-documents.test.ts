import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { LibraryDocument } from '../../app/types/negotiation-documents.ts'
import type { Copie } from '../../app/utils/guide-nego/copies.ts'
import { favorisConnus, placeDesCopies, surLeTelephone } from '../../app/utils/guide-nego/mes-documents.ts'

const doc = (id: string) => ({ id, title: id }) as LibraryDocument

const copie = (id: string, gardee_a: string, octets: number): Copie => ({
  id,
  format: 2,
  version: '1.0',
  reading_etag: null,
  reserve: false,
  gardee_a,
  octets,
  cles: [`${id}/reading`, `${id}/file`],
})

test('sur le téléphone : le dernier gardé d’abord, et seulement ce que la bibliothèque nomme', () => {
  const copies = [copie('guide', '2026-11-10T18:05:00Z', 7e6), copie('resume', '2026-11-11T09:00:00Z', 2e6), copie('retire', '2026-11-12T09:00:00Z', 1e6)]
  const lignes = surLeTelephone(copies, [doc('guide'), doc('resume')])
  assert.deepEqual(
    lignes.map((l) => l.document.id),
    ['resume', 'guide'],
  )
  assert.equal(placeDesCopies(copies), 10e6, 'une copie que la liste ne nomme plus occupe encore sa place')
  assert.equal(placeDesCopies([]), 0)
})

test('les favoris dans l’ordre de la bibliothèque, sans les dépubliés', () => {
  const favoris = favorisConnus(new Set(['b', 'disparu', 'a']), [doc('a'), doc('b'), doc('c')])
  assert.deepEqual(
    favoris.map((d) => d.id),
    ['a', 'b'],
  )
})
