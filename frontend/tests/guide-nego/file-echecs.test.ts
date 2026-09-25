import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  creerFile,
  decider,
  magasinEnMemoire,
  type Intention,
  type Reponse,
  type Suite,
} from '../../app/utils/guide-nego/file.ts'

const intention: Intention = {
  cle: 'mes-thematiques',
  corps: { codes: ['adaptation'] },
  empreinte: null,
  personne: 'awa',
  prise_a: '2026-11-12T10:00:00.000Z',
}

function fileQuiRepond(reponses: Reponse[], avis: Suite[] = []) {
  const magasin = magasinEnMemoire()
  let envois = 0
  const file = creerFile({
    magasin,
    expediteurs: new Map([
      ['mes-thematiques', { envoyer: async () => reponses[envois++] ?? { statut: 'panne' } }],
    ]),
    personne: () => 'awa',
    signaler: (suite) => avis.push(suite),
  })
  return { magasin, file, envois: () => envois }
}

test('un 5xx garde l’entrée, et elle repart au déclenchement suivant', async () => {
  const { magasin, file, envois } = fileQuiRepond([
    { statut: 'refus', code: 503, message: null },
    { statut: 'succes' },
  ])
  await magasin.poser(intention)

  assert.equal((await file.partir())[0]?.sort, 'reportee')
  assert.equal((await magasin.lire()).length, 1, 'c’est exactement ce pour quoi la file existe')

  assert.equal((await file.partir())[0]?.sort, 'envoyee')
  assert.deepEqual(await magasin.lire(), [])
  assert.equal(envois(), 2)
})

test('une panne réseau garde l’entrée aussi — envoi qui lève compris', async () => {
  const magasin = magasinEnMemoire()
  await magasin.poser(intention)
  const file = creerFile({
    magasin,
    expediteurs: new Map([
      [
        'mes-thematiques',
        {
          envoyer: async () => {
            throw new TypeError('Failed to fetch')
          },
        },
      ],
    ]),
    personne: () => 'awa',
  })
  assert.equal((await file.partir())[0]?.sort, 'reportee')
  assert.equal((await magasin.lire()).length, 1)
})

test('un refus définitif retire l’entrée et le dit', async () => {
  for (const code of [400, 401, 403]) {
    const avis: Suite[] = []
    const { magasin, file } = fileQuiRepond([{ statut: 'refus', code, message: `refus ${code}` }], avis)
    await magasin.poser(intention)

    assert.equal((await file.partir())[0]?.sort, 'refusee', `${code}`)
    assert.deepEqual(await magasin.lire(), [], `${code} : une intention qui ne peut pas aboutir n’a rien à faire en file`)
    assert.deepEqual(avis, [{ cle: 'mes-thematiques', sort: 'refusee', message: `refus ${code}` }])
  }
})

test('un refus relit l’état vrai : ce qui s’affichait déjà se défait', async () => {
  const magasin = magasinEnMemoire()
  await magasin.poser({ ...intention, cle: 'agenda-s1', corps: { garder: true, remind: false } })
  let relectures = 0
  const file = creerFile({
    magasin,
    expediteurs: new Map([
      [
        'agenda-s1',
        {
          envoyer: async () => ({ statut: 'refus' as const, code: 409, message: 'Cette session est annulée.' }),
          relire: () => void relectures++,
        },
      ],
    ]),
    personne: () => 'awa',
  })
  assert.equal((await file.partir())[0]?.sort, 'refusee')
  assert.deepEqual(await magasin.lire(), [], 'le 409 abandonne l’intention')
  assert.equal(relectures, 1)
})

test('la décision, code par code', () => {
  assert.equal(decider({ statut: 'succes' }), 'envoyee')
  assert.equal(decider({ statut: 'panne' }), 'reportee')
  assert.equal(decider({ statut: 'refus', code: 412, message: null }), 'perimee')
  assert.equal(decider({ statut: 'refus', code: 500, message: null }), 'reportee')
  assert.equal(decider({ statut: 'refus', code: 429, message: null }), 'reportee', 'trop d’essais : on réessaie plus tard')
  assert.equal(decider({ statut: 'refus', code: 422, message: null }), 'refusee')
})
