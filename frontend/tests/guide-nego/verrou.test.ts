import { test } from 'node:test'
import assert from 'node:assert/strict'
import { ACCES_VISITEUSE, etatDAcces } from '../../app/utils/guide-nego/acces.ts'
import {
  adresseDeLaSortie,
  moduleOuvert,
  sortieDuVerrou,
} from '../../app/utils/guide-nego/verrou.ts'
import type { AccessStateView } from '../../app/types/negotiation.ts'

const ADMISE: AccessStateView = {
  admission_mode: 'code',
  state: 'granted',
  granted: {
    scope: {
      type: 'negotiation_space',
      id: '01930000-0000-7000-8000-00000000000a',
      name: 'COP31 — Climat',
    },
    granted_at: '2026-11-08T10:12:00Z',
    source_code_label: 'Réseau des négociatrices — COP31',
  },
  networks: [],
  request: null,
}

/**
 * **Le cœur de FR-032.** Le verrou suit l'état dérivé du RBAC par l'API, et
 * rien d'autre. Un client qui **prétend** avoir l'accès n'ouvre rien : ni un
 * `granted` bricolé dans la garde locale, ni un mode d'admission recopié, ni un
 * réseau rejoint. Un seul champ décide, et il vient de la plateforme.
 */
test("un client qui prétend avoir l'accès n'ouvre aucun module", () => {
  const menteuse: AccessStateView = {
    ...ACCES_VISITEUSE,
    // Tout y est SAUF l'état : la garde locale a été réécrite à la main.
    granted: ADMISE.granted,
    networks: [{ code: 'women_negotiators', label: 'Réseau des négociatrices francophones' }],
  }

  assert.equal(moduleOuvert(menteuse), false, "un `granted` sans `state` n'ouvre rien")
  assert.equal(moduleOuvert({ ...menteuse, state: 'pending' }), false)
  assert.equal(moduleOuvert({ ...menteuse, state: 'rejected' }), false)
  assert.equal(
    moduleOuvert({ ...menteuse, state: 'revoked' }),
    false,
    'un accès retiré referme les modules dès la lecture suivante (FR-041)',
  )
  assert.equal(moduleOuvert(null), false, 'rien de lu : rien ne s’ouvre')

  assert.equal(moduleOuvert(ADMISE), true, "et l'état rendu par l'API, lui, ouvre")
})

/**
 * `etatDAcces` ne recopie que les champs de la réponse : un objet enrichi à la
 * main dans IndexedDB perd ce qu'il avait en trop, et `state` reste ce que
 * l'API avait dit.
 */
test("l'état gardé ne retient rien que l'API n'ait dit", () => {
  const bricole = {
    ...ACCES_VISITEUSE,
    granted: ADMISE.granted,
    // Un champ que l'écran pourrait croire décisif, et qui n'existe pas.
    ouvert: true,
  } as unknown as AccessStateView

  const garde = etatDAcces(bricole)
  assert.equal(garde.state, 'visitor')
  assert.equal(moduleOuvert(garde), false)
  assert.equal('ouvert' in garde, false, "le champ inventé ne survit pas à la lecture")
})

/**
 * FR-031 : le compte d'abord. Saisir un code ou demander l'accès sans session
 * n'ouvrirait rien, et le refus viendrait de l'API sans que l'écran ait rien
 * expliqué.
 */
test('sans compte, le verrou mène au compte quel que soit le mode', () => {
  for (const mode of ['code', 'approval', 'code_and_approval'] as const) {
    assert.equal(sortieDuVerrou({ ...ACCES_VISITEUSE, admission_mode: mode }, false), 'compte')
  }
  assert.equal(sortieDuVerrou(null, false), 'compte')
  assert.equal(adresseDeLaSortie('compte'), '/guide-nego/compte')
})

/**
 * FR-022 : en « approbation seule », le verrou mène à la demande — proposer la
 * saisie d'un code enverrait chercher un code qui n'ouvre rien.
 */
test('le mode décide de la suite du verrou', () => {
  const visiteuse = (mode: AccessStateView['admission_mode']): AccessStateView => ({
    ...ACCES_VISITEUSE,
    admission_mode: mode,
  })

  assert.equal(sortieDuVerrou(visiteuse('code'), true), 'code')
  assert.equal(
    sortieDuVerrou(visiteuse('code_and_approval'), true),
    'code',
    'en mode « les deux », le code se saisit : il ouvre une demande qui le porte',
  )
  assert.equal(sortieDuVerrou(visiteuse('approval'), true), 'demande')

  assert.equal(adresseDeLaSortie('code'), '/guide-nego/code')
  assert.equal(adresseDeLaSortie('demande'), '/guide-nego/demande')
})

/**
 * Une personne qui attend déjà une réponse n'a rien à ressaisir : on la ramène
 * à sa demande, qui lui dit où elle en est.
 */
test('une demande en attente ramène à la demande, jamais au code', () => {
  const enAttente: AccessStateView = {
    ...ACCES_VISITEUSE,
    admission_mode: 'code_and_approval',
    state: 'pending',
    request: {
      id: '01930000-0000-7000-8000-00000000000b',
      status: 'pending',
      submitted_at: '2026-11-12T09:20:00Z',
      decided_at: null,
      decision_reason: null,
    },
  }

  assert.equal(sortieDuVerrou(enAttente, true), 'attente')
  assert.equal(
    adresseDeLaSortie('attente'),
    '/guide-nego/demande',
    "« attente » et « demande » mènent au même écran, qui sait lequel des deux afficher",
  )
})

/**
 * Un refus n'est pas définitif, et le verrou ne s'y arrête pas : on repart par
 * le chemin que le mode offre.
 */
test('après un refus, le verrou repropose ce que le mode offre', () => {
  const refusee: AccessStateView = {
    ...ACCES_VISITEUSE,
    admission_mode: 'approval',
    state: 'rejected',
    request: {
      id: '01930000-0000-7000-8000-00000000000c',
      status: 'rejected',
      submitted_at: '2026-11-10T09:20:00Z',
      decided_at: '2026-11-11T09:20:00Z',
      decision_reason: 'Compte non vérifié',
    },
  }

  assert.equal(sortieDuVerrou(refusee, true), 'demande')
  assert.equal(sortieDuVerrou({ ...refusee, admission_mode: 'code' }, true), 'code')
})
