import { test } from 'node:test'
import assert from 'node:assert/strict'
import { issueDeRotation, type IssueDeRotation } from '../../app/utils/rotation.ts'
import { COMPTE_DECONNECTE, CompteInjoignable, relireLeCompte } from '../../app/utils/guide-nego/compte.ts'

test('seules « expired » et 401 disent que la session est finie', () => {
  assert.equal(issueDeRotation({ status: 200, corps: { status: 'renewed' } }), 'renouvelee')
  assert.equal(issueDeRotation({ status: 200, corps: { status: 'expired' } }), 'finie')
  assert.equal(issueDeRotation({ status: 401, corps: null }), 'finie')
})

test('tout le reste dit seulement que l’API ne répond pas', () => {
  assert.equal(issueDeRotation(null), 'injoignable', 'erreur réseau ou délai')
  for (const status of [500, 502, 503, 504, 429, 403]) {
    assert.equal(issueDeRotation({ status, corps: null }), 'injoignable', String(status))
  }
  assert.equal(issueDeRotation({ status: 200, corps: '<html>' }), 'injoignable', 'réponse illisible')
  assert.equal(issueDeRotation({ status: 200, corps: null }), 'injoignable')
})

const AWA = {
  id: 'awa',
  first_name: 'Awa',
  last_name: 'Diallo',
  primary_email: 'awa@example.org',
  email_verified_at: '2026-11-03T09:14:00Z',
}

/** `/auth/me` répond « personne » au premier appel, puis `apres` s'il est rappelé. */
function lecture(issue: IssueDeRotation, gardeConnectee: boolean, temoin = false, apres: unknown = AWA) {
  let appels = 0
  let rotations = 0
  return {
    deps: {
      lire: async () => (appels++ === 0 ? null : apres) as never,
      tourner: async () => (rotations++, issue),
      gardeConnectee,
      temoin,
    },
    rotations: () => rotations,
  }
}

/**
 * Les trois issues, croisées avec une garde connectée ou non. La garde connectée
 * suffit à tenter la rotation : c'est le lendemain matin, jeton d'accès expiré.
 */
test('les trois issues de la rotation, garde connectée ou non', async () => {
  for (const gardeConnectee of [true, false]) {
    const temoin = !gardeConnectee

    const renouvelee = await relireLeCompte(lecture('renouvelee', gardeConnectee, temoin).deps)
    assert.equal(renouvelee.connectee, true, `renouvelée, garde ${gardeConnectee}`)

    const finie = await relireLeCompte(lecture('finie', gardeConnectee, temoin).deps)
    assert.deepEqual(finie, COMPTE_DECONNECTE, `finie, garde ${gardeConnectee}`)

    await assert.rejects(
      relireLeCompte(lecture('injoignable', gardeConnectee, temoin).deps),
      CompteInjoignable,
      `injoignable, garde ${gardeConnectee} : on lève, la garde reste`,
    )
  }
})

test('sans garde connectée ni témoin, personne n’est déconnecté par une rotation', async () => {
  const { deps, rotations } = lecture('injoignable', false, false)
  assert.deepEqual(await relireLeCompte(deps), COMPTE_DECONNECTE)
  assert.equal(rotations(), 0, 'rien à tourner : il n’y avait pas de session')
})

test('une personne lue du premier coup ne fait tourner aucun jeton', async () => {
  let rotations = 0
  const etat = await relireLeCompte({
    lire: async () => AWA as never,
    tourner: async () => (rotations++, 'finie'),
    gardeConnectee: true,
    temoin: true,
  })
  assert.equal(etat.connectee, true)
  assert.equal(rotations, 0)
})
