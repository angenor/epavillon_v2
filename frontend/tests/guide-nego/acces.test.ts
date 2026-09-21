import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  ACCES_VISITEUSE,
  accesLisibleHorsConnexion,
  accesOuvert,
  etatDAcces,
  saisiePossible,
} from '../../app/utils/guide-nego/acces.ts'

const ADMISE = {
  admission_mode: 'code' as const,
  state: 'granted' as const,
  granted: {
    scope: { type: 'negotiation_space' as const, id: '01930000-0000-7000-8000-00000000000a', name: 'COP31 — Climat' },
    granted_at: '2026-11-08T10:12:00Z',
    source_code_label: 'Réseau des négociatrices — COP31',
  },
  networks: [{ code: 'women_negotiators', label: 'Réseau des négociatrices francophones' }],
  request: null,
}

test("l'état d'accès ne garde que ce que l'écran affiche", () => {
  const etat = etatDAcces(ADMISE)
  assert.equal(etat.state, 'granted')
  assert.equal(etat.granted?.scope.name, 'COP31 — Climat')
  assert.equal(etat.networks.length, 1)
  assert.equal(etat.request, null)
})

test('rien de lu : visiteuse, et rien ne lève', () => {
  assert.deepEqual(etatDAcces(null), ACCES_VISITEUSE)
})

/**
 * **Un seul état ouvre les modules réservés.** Un écran qui testerait « pas
 * visiteuse » les ouvrirait à une personne dont l'accès vient d'être retiré, ou
 * dont la demande est encore en attente.
 */
test('seul « admise » ouvre les modules réservés', () => {
  assert.equal(accesOuvert(ADMISE), true)

  for (const etat of ['visitor', 'pending', 'rejected', 'revoked'] as const) {
    assert.equal(
      accesOuvert({ ...ADMISE, state: etat }),
      false,
      `« ${etat} » ne doit rien ouvrir`,
    )
  }
  assert.equal(accesOuvert(null), false)
})

/**
 * **Le cœur de FR-019.** La saisie d'un code exige le réseau : rien n'est mis en
 * file, parce qu'un accès ne se promet pas avant d'avoir été obtenu. Le geste
 * réflexe — accepter la saisie et l'envoyer plus tard — annoncerait un accès que
 * l'API pourrait refuser.
 */
test('sans réseau, la saisie d\'un code est impossible', () => {
  assert.equal(saisiePossible(ADMISE, false), false)
  assert.equal(saisiePossible(ACCES_VISITEUSE, false), false)
  assert.equal(saisiePossible(null, false), false, 'rien de lu et pas de réseau : toujours non')
})

/** FR-022 : en mode « approbation seule », le code ne se propose pas du tout. */
test('en mode « approbation seule », le code ne se saisit pas', () => {
  assert.equal(saisiePossible({ ...ACCES_VISITEUSE, admission_mode: 'approval' }, true), false)
  assert.equal(saisiePossible({ ...ACCES_VISITEUSE, admission_mode: 'code' }, true), true)
  assert.equal(
    saisiePossible({ ...ACCES_VISITEUSE, admission_mode: 'code_and_approval' }, true),
    true,
    'le code reconnu ouvre une demande : il se saisit',
  )
})

/**
 * FR-034 : hors connexion, on affiche ce qui a été lu avec l'heure de sa
 * lecture. **Jamais un écran vide, et jamais un accès inventé.**
 */
test("hors connexion, l'état lu s'affiche avec son heure", () => {
  const lu = accesLisibleHorsConnexion(ADMISE, '2026-11-12T08:30:00Z')
  assert.equal(lu.etat.state, 'granted')
  assert.equal(lu.luA, '2026-11-12T08:30:00Z')

  const jamais = accesLisibleHorsConnexion(null, null)
  assert.deepEqual(jamais.etat, ACCES_VISITEUSE, 'jamais un écran vide')
  assert.equal(jamais.luA, null)
  assert.equal(accesOuvert(jamais.etat), false, 'et jamais un accès inventé')
})
