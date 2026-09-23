import { test } from 'node:test'
import assert from 'node:assert/strict'
import { creerFileATelecharger, type IssueDeTelechargement } from '../../app/utils/guide-nego/a-telecharger.ts'
import { magasinEnMemoire, type DemandeDeTelechargement } from '../../app/utils/guide-nego/copies.ts'

const demande = (id: string): DemandeDeTelechargement => ({ id, reserve: false, octets: null, demande_a: '2026-11-12T09:00:00Z' })

test('deux déclencheurs à la fois ne téléchargent qu’une fois', async () => {
  const magasin = magasinEnMemoire<DemandeDeTelechargement>()
  let appels = 0
  let liberer: () => void = () => {}
  const file = creerFileATelecharger({
    magasin,
    telecharger: () =>
      new Promise<IssueDeTelechargement>((resolve) => {
        appels += 1
        liberer = () => resolve('reussi')
      }),
  })
  await file.demander(demande('guide'))
  const online = file.partir()
  const premierPlan = file.partir()
  await new Promise((r) => setTimeout(r, 0))
  liberer()
  await Promise.all([online, premierPlan])
  assert.equal(appels, 1)
  assert.deepEqual(await magasin.lire(), [], 'réussie, la demande se retire')
})

test('la demande survit à une fermeture, et part au premier déclencheur suivant', async () => {
  const magasin = magasinEnMemoire<DemandeDeTelechargement>()
  const sansReseau = creerFileATelecharger({ magasin, telecharger: async () => 'panne' })
  await sansReseau.demander(demande('guide'))
  await sansReseau.partir()
  assert.equal((await magasin.lire()).length, 1, 'une panne la laisse')

  // Réouverture : une nouvelle file sur le même magasin.
  const partis: string[] = []
  const reouverte = creerFileATelecharger({
    magasin,
    telecharger: async (d) => {
      partis.push(d.id)
      return 'reussi'
    },
  })
  await reouverte.partir()
  await reouverte.partir()
  assert.deepEqual(partis, ['guide'], 'une fois, et pas à chaque déclencheur')
})

test('un refus ou une place manquante retire la demande : la rejouer n’y changerait rien', async () => {
  const magasin = magasinEnMemoire<DemandeDeTelechargement>()
  const issues: Record<string, IssueDeTelechargement> = { depublie: 'refus', trop_gros: 'place', coupe: 'panne' }
  const file = creerFileATelecharger({ magasin, telecharger: async (d) => issues[d.id]! })
  for (const id of Object.keys(issues)) await file.demander(demande(id))
  await file.partir()
  assert.deepEqual((await magasin.lire()).map((d) => d.id), ['coupe'])
})

test('une demande reposée ne fait qu’une entrée', async () => {
  const magasin = magasinEnMemoire<DemandeDeTelechargement>()
  const file = creerFileATelecharger({ magasin, telecharger: async () => 'reussi' })
  await file.demander(demande('guide'))
  await file.demander(demande('guide'))
  assert.equal((await magasin.lire()).length, 1)
})

test('une demande retirée pendant que la file part ne part plus : la déconnexion l’a effacée', async () => {
  const magasin = magasinEnMemoire<DemandeDeTelechargement>()
  const partis: string[] = []
  const file = creerFileATelecharger({
    magasin,
    telecharger: async (d) => {
      partis.push(d.id)
      // Pendant le premier téléchargement, la personne se déconnecte.
      if (d.id === 'public') await magasin.retirer('reserve')
      return 'reussi'
    },
  })
  await file.demander(demande('public'))
  await file.demander({ ...demande('reserve'), reserve: true })
  await file.partir()
  assert.deepEqual(partis, ['public'])
})

test('un magasin illisible ne fait rien partir, et n’efface rien', async () => {
  const magasin = { ...magasinEnMemoire<DemandeDeTelechargement>(), lire: async () => null }
  const file = creerFileATelecharger({ magasin, telecharger: async () => assert.fail('rien ne doit partir') })
  assert.deepEqual(await file.partir(), [])
})
