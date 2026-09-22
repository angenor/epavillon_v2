import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  creerFile,
  magasinEnMemoire,
  type Intention,
  type Suite,
} from '../../app/utils/guide-nego/file.ts'

const intention: Intention = {
  cle: 'mes-thematiques',
  corps: { codes: ['adaptation'] },
  empreinte: '"a1"',
  personne: 'awa',
  prise_a: '2026-11-12T10:00:00.000Z',
}

test('un 412 retire l’entrée, déclenche la relecture et produit le message', async () => {
  const magasin = magasinEnMemoire()
  await magasin.poser(intention)
  let relectures = 0
  const avis: Suite[] = []

  const file = creerFile({
    magasin,
    expediteurs: new Map([
      [
        'mes-thematiques',
        {
          envoyer: async () => ({
            statut: 'refus' as const,
            code: 412,
            message: 'Vos thématiques ont changé sur un autre appareil.',
          }),
          relire: () => void relectures++,
        },
      ],
    ]),
    personne: () => 'awa',
    signaler: (suite) => avis.push(suite),
  })

  const suites = await file.partir()
  assert.deepEqual(suites, [
    { cle: 'mes-thematiques', sort: 'perimee', message: 'Vos thématiques ont changé sur un autre appareil.' },
  ])
  assert.deepEqual(await magasin.lire(), [], 'l’intention est abandonnée, jamais fusionnée')
  assert.equal(relectures, 1, 'l’état vrai est relu')
  assert.equal(avis.length, 1, 'la personne est prévenue')
  assert.equal(avis[0]?.sort, 'perimee')
})

test('un succès retire l’entrée, relit, et ne dit rien', async () => {
  const magasin = magasinEnMemoire()
  await magasin.poser(intention)
  let relectures = 0
  const avis: Suite[] = []

  const file = creerFile({
    magasin,
    expediteurs: new Map([
      ['mes-thematiques', { envoyer: async () => ({ statut: 'succes' as const }), relire: () => void relectures++ }],
    ]),
    personne: () => 'awa',
    signaler: (suite) => avis.push(suite),
  })

  assert.equal((await file.partir())[0]?.sort, 'envoyee')
  assert.deepEqual(await magasin.lire(), [])
  assert.equal(relectures, 1)
  assert.equal(avis.length, 0, 'rien à dire : le choix affiché est celui qui est parti')
})
