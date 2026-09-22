import { test } from 'node:test'
import assert from 'node:assert/strict'
import { creerFile, magasinEnMemoire, type Intention } from '../../app/utils/guide-nego/file.ts'

const intentionDAwa: Intention = {
  cle: 'mes-thematiques',
  corps: { codes: ['adaptation'] },
  empreinte: null,
  personne: 'awa',
  prise_a: '2026-11-12T10:00:00.000Z',
}

test('après la déconnexion, la file est vide et rien ne part ensuite', async () => {
  const magasin = magasinEnMemoire()
  await magasin.poser(intentionDAwa)
  let envois = 0
  let personne: string | null = 'awa'
  const file = creerFile({
    magasin,
    expediteurs: new Map([['mes-thematiques', { envoyer: async () => (envois++, { statut: 'succes' as const }) }]]),
    personne: () => personne,
  })

  // Se déconnecter : la file se vide d'abord.
  await file.vider()
  personne = null
  assert.deepEqual(await magasin.lire(), [])

  // Fatou se connecte sur le même téléphone.
  personne = 'fatou'
  assert.deepEqual(await file.partir(), [])
  assert.equal(envois, 0, 'le choix d’Awa ne part pas sous le compte de Fatou')
})

test('une intention prise par une personne, une autre connectée : rien ne part', async () => {
  const magasin = magasinEnMemoire()
  await magasin.poser(intentionDAwa)
  let envois = 0
  const file = creerFile({
    magasin,
    expediteurs: new Map([['mes-thematiques', { envoyer: async () => (envois++, { statut: 'succes' as const }) }]]),
    personne: () => 'fatou',
  })

  const suites = await file.partir()
  assert.equal(suites[0]?.sort, 'ignoree')
  assert.equal(envois, 0)
  assert.equal((await magasin.lire()).length, 1, 'l’entrée reste : ce n’est pas à Fatou de la retirer')
})

test('sans personne connectée, rien ne part non plus', async () => {
  const magasin = magasinEnMemoire()
  await magasin.poser(intentionDAwa)
  let envois = 0
  const file = creerFile({
    magasin,
    expediteurs: new Map([['mes-thematiques', { envoyer: async () => (envois++, { statut: 'succes' as const }) }]]),
    personne: () => null,
  })
  await file.partir()
  assert.equal(envois, 0)
})
