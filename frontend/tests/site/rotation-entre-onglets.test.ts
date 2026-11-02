import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  ATTENTE_MAX_MS,
  ROTATION_FRAICHE_MS,
  tournerEntreOnglets,
  type Marque,
  type Onglets,
} from '../../app/utils/rotation-entre-onglets.ts'
import type { IssueDeRotation } from '../../app/utils/rotation.ts'

interface R {
  issue: IssueDeRotation
}

/** Un navigateur simulé : un verrou en file, une marque partagée, une horloge. */
function navigateur(avecVerrou = true) {
  let marque: Marque | null = null
  let horloge = 1_000_000
  let file: Promise<unknown> = Promise.resolve()
  const onglet = (id: string): Onglets => ({
    onglet: id,
    verrou: avecVerrou
      ? <T>(_nom: string, _signal: AbortSignal, travail: () => Promise<T>) => {
          const tour = file.then(travail)
          file = tour.catch(() => undefined)
          return tour
        }
      : undefined,
    lireMarque: () => marque,
    poserMarque: (m) => (marque = m),
    maintenant: () => horloge,
  })
  return { onglet, avancer: (ms: number) => (horloge += ms) }
}

function compteur(issue: IssueDeRotation = 'renouvelee') {
  const appel = { n: 0 }
  const tourner = async (): Promise<R> => {
    appel.n += 1
    await new Promise((resolve) => setTimeout(resolve, 5))
    return { issue }
  }
  return { appel, tourner }
}

const DEJA: R = { issue: 'renouvelee' }

test('six onglets qui rafraîchissent ensemble : une seule rotation', async () => {
  const nav = navigateur()
  const { appel, tourner } = compteur()
  const issues = await Promise.all(
    ['a', 'b', 'c', 'd', 'e', 'f'].map((id) => tournerEntreOnglets(tourner, DEJA, nav.onglet(id))),
  )
  assert.equal(appel.n, 1)
  assert.ok(issues.every((r) => r.issue === 'renouvelee'), 'chacun rejoue sa requête')
})

test('la marque d’une rotation ancienne ne dispense pas de tourner', async () => {
  const nav = navigateur()
  const { appel, tourner } = compteur()
  await tournerEntreOnglets(tourner, DEJA, nav.onglet('a'))
  nav.avancer(ROTATION_FRAICHE_MS)
  await tournerEntreOnglets(tourner, DEJA, nav.onglet('b'))
  assert.equal(appel.n, 2)
})

test('la marque de son propre onglet ne dispense pas de tourner', async () => {
  const nav = navigateur()
  const { appel, tourner } = compteur()
  const a = nav.onglet('a')
  await tournerEntreOnglets(tourner, DEJA, a)
  await tournerEntreOnglets(tourner, DEJA, a)
  assert.equal(appel.n, 2)
})

test('une session finie ne pose pas de marque : l’onglet suivant demande à l’API', async () => {
  const nav = navigateur()
  const { appel, tourner } = compteur('finie')
  const [a, b] = await Promise.all([
    tournerEntreOnglets(tourner, DEJA, nav.onglet('a')),
    tournerEntreOnglets(tourner, DEJA, nav.onglet('b')),
  ])
  assert.equal(appel.n, 2)
  assert.equal(a.issue, 'finie')
  assert.equal(b.issue, 'finie')
})

test('sans verrou, la marque évite la seconde rotation', async () => {
  const nav = navigateur(false)
  const { appel, tourner } = compteur()
  await tournerEntreOnglets(tourner, DEJA, nav.onglet('a'))
  const b = await tournerEntreOnglets(tourner, DEJA, nav.onglet('b'))
  assert.equal(appel.n, 1)
  assert.equal(b.issue, 'renouvelee')
})

test('un verrou tenu trop longtemps n’empêche pas de tourner', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] })
  const appel = { n: 0 }
  const tourner = async (): Promise<R> => {
    appel.n += 1
    return { issue: 'renouvelee' }
  }
  const fige: Onglets = {
    onglet: 'b',
    verrou: (_nom, signal) =>
      new Promise((_resolve, reject) => signal.addEventListener('abort', () => reject(new DOMException('', 'AbortError')))),
    lireMarque: () => null,
    poserMarque: () => undefined,
    maintenant: () => 0,
  }
  const issue = tournerEntreOnglets(tourner, DEJA, fige)
  t.mock.timers.tick(ATTENTE_MAX_MS)
  assert.equal((await issue).issue, 'renouvelee')
  assert.equal(appel.n, 1)
})
