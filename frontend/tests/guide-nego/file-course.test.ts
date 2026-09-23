import { test } from 'node:test'
import assert from 'node:assert/strict'
import { creerFile, magasinEnMemoire, type Expediteur, type Intention } from '../../app/utils/guide-nego/file.ts'

const favori = (voulu: boolean): Intention => ({
  cle: 'favori-guide',
  corps: { favori: voulu },
  empreinte: null,
  personne: 'awa',
  prise_a: '2026-11-12T10:00:00.000Z',
})

/** Un envoi qui ne rend la main que quand le test le dit. */
function envoiRetenu() {
  const envoyes: unknown[] = []
  let liberer: () => void = () => undefined
  const expediteur: Expediteur = {
    envoyer: async (intention) => {
      envoyes.push(intention.corps)
      if (envoyes.length === 1) await new Promise<void>((r) => (liberer = r))
      return { statut: 'succes' }
    },
  }
  return { envoyes, expediteur, liberer: () => liberer() }
}

test('un choix remplacé pendant son envoi n’est pas perdu : il repart dans la foulée', async () => {
  const magasin = magasinEnMemoire()
  const envoi = envoiRetenu()
  const file = creerFile({ magasin, expediteurs: () => envoi.expediteur, personne: () => 'awa' })

  await file.poser(favori(true))
  const depart = file.partir()
  await new Promise((r) => setTimeout(r, 0))
  await file.poser(favori(false))
  envoi.liberer()
  await depart

  assert.deepEqual(envoi.envoyes, [{ favori: true }, { favori: false }])
  assert.deepEqual(await magasin.lire(), [])
})

test('un choix posé pendant un départ part avec lui, sans attendre le déclencheur suivant', async () => {
  const magasin = magasinEnMemoire()
  const envoi = envoiRetenu()
  const file = creerFile({ magasin, expediteurs: () => envoi.expediteur, personne: () => 'awa' })

  await file.poser({ ...favori(true), cle: 'favori-note' })
  const depart = file.partir()
  await new Promise((r) => setTimeout(r, 0))
  await file.poser(favori(true))
  envoi.liberer()
  await depart

  assert.equal(envoi.envoyes.length, 2)
  assert.deepEqual(await magasin.lire(), [])
})
