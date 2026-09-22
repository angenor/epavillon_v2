import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  ACCES_VISITEUSE,
  accesLisibleHorsConnexion,
  accesOuvert,
  codeOffertParLeMode,
  demandePossible,
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

/**
 * FR-022 : le champ de code DISPARAÎT en « approbation seule ».
 *
 * **À ne pas confondre avec `saisiePossible`**, qui dit ce qui est faisable
 * maintenant. Celle-ci dit ce que le MODE offre, sans regarder le réseau : les
 * confondre ferait disparaître le champ à chaque tunnel, au lieu de le
 * désactiver en disant ce qui manque.
 */
test('le mode « approbation seule » retire le code du parcours', () => {
  assert.equal(codeOffertParLeMode({ ...ACCES_VISITEUSE, admission_mode: 'approval' }), false)
  assert.equal(codeOffertParLeMode({ ...ACCES_VISITEUSE, admission_mode: 'code' }), true)
  assert.equal(
    codeOffertParLeMode({ ...ACCES_VISITEUSE, admission_mode: 'code_and_approval' }),
    true,
    'en mode « les deux », le code se saisit — il ouvre une demande qui le porte',
  )
  assert.equal(codeOffertParLeMode(null), true, 'rien de lu : le parcours ordinaire')
})

/**
 * FR-024 : on ne demande pas ce qu'on a déjà, ni ce qu'on a déjà demandé.
 *
 * Et en mode « code seul », la demande ne se propose pas du tout : personne ne
 * la traiterait, et la réponse promise ne viendrait jamais.
 */
test("la demande d'accès ne se propose que lorsqu'elle a un sens", () => {
  const visiteuse = { ...ACCES_VISITEUSE, admission_mode: 'approval' as const }

  assert.equal(demandePossible(visiteuse, true), true)
  assert.equal(demandePossible(visiteuse, false), false, 'sans réseau, rien ne part')
  assert.equal(
    demandePossible({ ...ACCES_VISITEUSE, admission_mode: 'code' }, true),
    false,
    'en mode « code seul », la demande n\'a pas de destinataire',
  )
  assert.equal(
    demandePossible({ ...ADMISE, admission_mode: 'approval' }, true),
    false,
    "on ne demande pas un accès qu'on détient",
  )
  assert.equal(
    demandePossible({ ...visiteuse, state: 'pending' }, true),
    false,
    'une demande en attente ne se redemande pas',
  )
  assert.equal(
    demandePossible({ ...visiteuse, state: 'rejected' }, true),
    true,
    'un refus n\'est pas définitif : une nouvelle demande est une nouvelle ligne',
  )
  assert.equal(demandePossible(null, true), false, 'rien de lu : on ne propose rien')
})
