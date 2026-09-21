import { test, beforeEach } from 'node:test'
import assert from 'node:assert/strict'
import {
  CLE_APPAREIL,
  identifiantDAppareil,
  libelleDAppareil,
  plateformeDe,
} from '../../app/utils/guide-nego/appareil.ts'

/**
 * `localStorage` n'existe pas sous Node : on le simule, et on l'éteint pour
 * éprouver le cas « stockage refusé » — la navigation privée, où l'accès LÈVE.
 */
function poserUnStockage(): Map<string, string> {
  const boite = new Map<string, string>()
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (cle: string) => boite.get(cle) ?? null,
      setItem: (cle: string, valeur: string) => void boite.set(cle, valeur),
    },
  })
  return boite
}

function poserUnStockageQuiRefuse(): void {
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: () => {
        throw new Error('stockage refusé')
      },
      setItem: () => {
        throw new Error('stockage refusé')
      },
    },
  })
}

beforeEach(() => {
  poserUnStockage()
})

test("l'identifiant d'appareil est engendré une fois, puis relu", () => {
  const boite = poserUnStockage()

  const premier = identifiantDAppareil()
  assert.ok(premier.length >= 8, 'un identifiant vide ne nommerait aucune session')
  assert.equal(boite.get(CLE_APPAREIL), premier)
  assert.equal(identifiantDAppareil(), premier, 'la seconde demande rend le même')
})

test('la clé porte le préfixe de Guide Négo, et jamais celui du site', () => {
  assert.ok(CLE_APPAREIL.startsWith('gn.'))
  assert.ok(!CLE_APPAREIL.includes('epavillon'))
})

/**
 * Un stockage refusé ne doit pas empêcher d'ouvrir une session : l'identifiant
 * n'accorde rien, et une application qui ne s'ouvre pas en navigation privée
 * serait un défaut bien plus grave qu'un identifiant qui change.
 */
test('un stockage refusé rend quand même un identifiant', () => {
  poserUnStockageQuiRefuse()
  const identifiant = identifiantDAppareil()
  assert.ok(identifiant.length >= 8)
})

test('la plateforme se déduit de ce que le téléphone annonce', () => {
  assert.equal(
    plateformeDe('Mozilla/5.0 (Linux; Android 14; SM-A546B) AppleWebKit/537.36 Chrome/120 Mobile Safari/537.36'),
    'android',
  )
  assert.equal(
    plateformeDe('Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 Version/17.1 Mobile Safari/604.1'),
    'ios',
  )
  assert.equal(plateformeDe('Mozilla/5.0 (X11; Linux x86_64) Chrome/120'), 'other')
  assert.equal(plateformeDe(''), 'other')
})

/**
 * L'ordre des tests de navigateur n'est pas décoratif : Chrome annonce
 * « Safari », Edge et Opera annoncent « Chrome ». Tester Safari en premier
 * nommerait « Safari » le Chrome de tous les Android.
 */
test("le libellé nomme le bon navigateur malgré les annonces qui se recouvrent", () => {
  assert.equal(
    libelleDAppareil('Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Chrome/120 Mobile Safari/537.36'),
    'Android · Chrome',
  )
  assert.equal(
    libelleDAppareil('Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Chrome/120 Safari/537.36 Edg/120'),
    'Android · Edge',
  )
  assert.equal(
    libelleDAppareil('Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) Version/17.1 Mobile Safari/604.1'),
    'iPhone · Safari',
  )
  assert.equal(
    libelleDAppareil('Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) CriOS/120 Mobile Safari/604.1'),
    'iPhone · Chrome',
  )
  assert.equal(libelleDAppareil(''), 'Ordinateur · Navigateur')
})
