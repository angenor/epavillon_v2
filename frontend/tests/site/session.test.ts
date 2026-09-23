import { test } from 'node:test'
import assert from 'node:assert/strict'
import { authGuardOutcome } from '../../app/utils/auth-guard.ts'
import { internalRedirect } from '../../app/utils/redirect.ts'

test('une session connue passe', () => {
  assert.equal(authGuardOutcome({ isAuthenticated: true, isResolved: true }), 'allow')
})

test('une session non tranchée passe : le navigateur tournera le jeton', () => {
  // Rendu serveur avec un témoin, ou API injoignable.
  assert.equal(authGuardOutcome({ isAuthenticated: false, isResolved: false }), 'allow')
})

test('seule une session finie mène à la connexion', () => {
  assert.equal(authGuardOutcome({ isAuthenticated: false, isResolved: true }), 'sign-in')
})

test('« Continuer » mène à la page demandée', () => {
  assert.equal(internalRedirect('/admin/negociations/codes', '/'), '/admin/negociations/codes')
  assert.equal(internalRedirect(['/en/admin'], '/'), '/en/admin')
})

test('« Continuer » mène à l’accueil sans destination, ou vers une adresse extérieure', () => {
  assert.equal(internalRedirect(undefined, '/'), '/')
  assert.equal(internalRedirect('https://ailleurs.example', '/'), '/')
  assert.equal(internalRedirect('//ailleurs.example', '/'), '/')
})
