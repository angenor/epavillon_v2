import { test } from 'node:test'
import assert from 'node:assert/strict'
import { creerLAttente, type EtatDAttente } from '../../app/utils/guide-nego/pdf/attente.ts'
import { fausseHorloge } from './fausse-horloge.ts'

function attente() {
  const horloge = fausseHorloge()
  const changements: EtatDAttente[] = []
  const a = creerLAttente({ minuterie: horloge.minuterie, surChangement: (e) => changements.push(e) })
  a.demarrer()
  return { horloge, changements, a }
}

test('au départ, on attend, sans sorties', () => {
  const { a, changements } = attente()
  assert.deepEqual(a.etat(), { mode: 'attente', sorties: false, resteSurLeTexte: false, pagePrete: false })
  assert.deepEqual(changements, [])
})

test('au bout de 3 s sans page, les sorties paraissent', () => {
  const { horloge, a, changements } = attente()
  horloge.avancer(2999)
  assert.equal(a.etat().sorties, false)
  horloge.avancer(1)
  assert.equal(a.etat().sorties, true)
  assert.deepEqual(changements.at(-1), a.etat())
})

test('la page prête avant 3 s : pas de sorties, les pages s’affichent', () => {
  const { horloge, a } = attente()
  horloge.avancer(2000)
  a.pagePrete()
  horloge.avancer(10_000)
  assert.deepEqual(a.etat(), { mode: 'pages', sorties: false, resteSurLeTexte: false, pagePrete: true })
  assert.equal(horloge.enCours(), 0)
})

test('le texte en attendant, puis la page prête : la page prend sa place', () => {
  const { horloge, a } = attente()
  horloge.avancer(3000)
  a.lireLeTexte()
  assert.equal(a.etat().mode, 'texte')
  a.pagePrete()
  assert.deepEqual(a.etat(), { mode: 'pages', sorties: false, resteSurLeTexte: false, pagePrete: true })
})

test('le texte, « Rester sur le texte », puis la page prête : on reste sur le texte', () => {
  const { horloge, a } = attente()
  horloge.avancer(3000)
  a.lireLeTexte()
  a.resterSurLeTexte()
  a.pagePrete()
  assert.deepEqual(a.etat(), { mode: 'texte', sorties: false, resteSurLeTexte: true, pagePrete: true })
})

test('les pages choisies pendant l’attente : les pages, sans bascule ensuite', () => {
  const { horloge, a } = attente()
  horloge.avancer(1000)
  a.choisirLeMode('pages')
  assert.equal(a.etat().mode, 'pages')
  assert.equal(a.etat().pagePrete, false)
  assert.equal(a.etat().resteSurLeTexte, true)
  a.pagePrete()
  assert.equal(a.etat().mode, 'pages')
})

test('le texte choisi à la main : la page prête ne le remplace pas', () => {
  const { a } = attente()
  a.choisirLeMode('texte')
  a.pagePrete()
  assert.equal(a.etat().mode, 'texte')
})

test('arrêter annule le minuteur des sorties', () => {
  const { horloge, a, changements } = attente()
  a.arreter()
  horloge.avancer(10_000)
  assert.equal(a.etat().sorties, false)
  assert.deepEqual(changements, [])
})
