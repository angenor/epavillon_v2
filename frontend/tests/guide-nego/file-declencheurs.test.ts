import { test } from 'node:test'
import assert from 'node:assert/strict'
import { creerFile, magasinEnMemoire, type Intention } from '../../app/utils/guide-nego/file.ts'

const intention: Intention = {
  cle: 'mes-thematiques',
  corps: { codes: ['adaptation'] },
  empreinte: null,
  personne: 'awa',
  prise_a: '2026-11-12T10:00:00.000Z',
}

/** Une file dont l'envoi attend qu'on le libère : c'est ce qui rend la simultanéité visible. */
function fileLente() {
  const magasin = magasinEnMemoire()
  let envois = 0
  let liberer: () => void = () => {}
  const file = creerFile({
    magasin,
    expediteurs: new Map([
      [
        'mes-thematiques',
        {
          envoyer: () =>
            new Promise((resolve) => {
              envois++
              liberer = () => resolve({ statut: 'succes' as const })
            }),
        },
      ],
    ]),
    personne: () => 'awa',
  })
  return { magasin, file, envois: () => envois, liberer: () => liberer() }
}

test('chacun des trois déclencheurs fait partir la file', async () => {
  const { magasin, file, envois, liberer } = fileLente()

  // Ouverture de l'application, événement `online`, retour au premier plan : les trois
  // appellent la même chose, et chacun doit suffire.
  for (const declencheur of ['ouverture', 'online', 'premier plan']) {
    await magasin.poser(intention)
    const depart = file.partir()
    await new Promise((r) => setTimeout(r, 0))
    liberer()
    assert.equal((await depart)[0]?.sort, 'envoyee', declencheur)
  }
  assert.equal(envois(), 3)
})

test('deux déclencheurs simultanés n’envoient qu’une fois', async () => {
  const { magasin, file, envois, liberer } = fileLente()
  await magasin.poser(intention)

  const premier = file.partir()
  const second = file.partir()
  await new Promise((r) => setTimeout(r, 0))
  liberer()

  const [a, b] = await Promise.all([premier, second])
  assert.deepEqual(a, b, 'le second déclencheur reçoit le même départ')
  assert.equal(envois(), 1)
  assert.deepEqual(await magasin.lire(), [])
})

test('un déclencheur après la fin du départ précédent repart bien', async () => {
  const { magasin, file, envois, liberer } = fileLente()
  await magasin.poser(intention)

  const premier = file.partir()
  await new Promise((r) => setTimeout(r, 0))
  liberer()
  await premier

  await magasin.poser(intention)
  const second = file.partir()
  await new Promise((r) => setTimeout(r, 0))
  liberer()
  await second

  assert.equal(envois(), 2)
})
