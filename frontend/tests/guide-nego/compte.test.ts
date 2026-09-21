import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  COMPTE_DECONNECTE,
  compteLisibleHorsConnexion,
  etatDuCompte,
  reconnexionAReclamer,
} from '../../app/utils/guide-nego/compte.ts'

const AWA = {
  id: '01930000-0000-7000-8000-000000000001',
  first_name: 'Awa',
  last_name: 'Diallo',
  primary_email: 'awa.diallo@example.org',
  email_verified_at: '2026-11-03T09:12:00Z',
  session: { client_kind: 'app' as const, device_label: 'Android · Chrome', issued_at: '2026-11-03T09:14:00Z' },
}

test("l'état du compte ne garde que ce que l'écran affiche", () => {
  const etat = etatDuCompte(AWA as never)
  assert.equal(etat.connectee, true)
  assert.equal(etat.prenom, 'Awa')
  assert.equal(etat.adresse, 'awa.diallo@example.org')
  assert.equal(etat.adresseConfirmee, true)
  assert.equal(etat.appareil, 'Android · Chrome')
  assert.equal(etat.ouverteLe, '2026-11-03T09:14:00Z')
})

test('une adresse non confirmée se distingue d\'une adresse confirmée', () => {
  const etat = etatDuCompte({ ...AWA, email_verified_at: null } as never)
  assert.equal(etat.connectee, true)
  assert.equal(etat.adresseConfirmee, false, "c'est la seule chose qui retient une connexion")
})

test('personne de connectée : tout est nul, et rien ne lève', () => {
  assert.deepEqual(etatDuCompte(null), COMPTE_DECONNECTE)
})

/**
 * **Le cœur de FR-006 ter.** Réclamer une reconnexion sans réseau est une
 * impasse : la personne ne peut rien faire, et l'écran qu'on vient de vider
 * était le seul contenu dont elle disposait.
 */
test('sans réseau, aucune reconnexion n\'est réclamée', () => {
  assert.equal(reconnexionAReclamer(COMPTE_DECONNECTE, false, true), false)
  assert.equal(reconnexionAReclamer(null, false, true), false)
})

test('au retour du réseau, la reconnexion est réclamée — et pas avant', () => {
  assert.equal(reconnexionAReclamer(COMPTE_DECONNECTE, true, true), true)
  assert.equal(
    reconnexionAReclamer(COMPTE_DECONNECTE, true, false),
    false,
    "tant qu'aucun verdict n'est possible, on ne réclame rien",
  )
})

test('une session vivante ne réclame jamais rien', () => {
  assert.equal(reconnexionAReclamer(etatDuCompte(AWA as never), true, true), false)
})

/**
 * Hors connexion, l'écran garde ce qui a été lu **avec l'heure de sa lecture**,
 * et ne se vide jamais.
 */
test('hors connexion, ce qui a été lu reste lisible avec son heure', () => {
  const garde = compteLisibleHorsConnexion(etatDuCompte(AWA as never), '2026-11-03T09:14:00Z')
  assert.equal(garde.etat.adresse, 'awa.diallo@example.org')
  assert.equal(garde.luA, '2026-11-03T09:14:00Z')

  const rien = compteLisibleHorsConnexion(null, null)
  assert.deepEqual(rien.etat, COMPTE_DECONNECTE, "jamais d'écran vide, même sans rien de gardé")
})
