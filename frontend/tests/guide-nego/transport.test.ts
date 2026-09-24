import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  brancherLeTransport,
  ErreurDeReseau,
  lireLaTaille,
  type TransportDePdfjs,
} from '../../app/utils/guide-nego/pdf/transport.ts'

const ADRESSE = 'https://exemple.org/v2/api/negotiation/documents/d1/file'

/** Un `fetch` dont on tient chaque réponse : on voit ce qui est en route. */
function fauxReseau() {
  const appels: { adresse: string; init: RequestInit; repondre: (r: Response) => void; couper: () => void }[] = []
  const recuperer = (adresse: string, init: RequestInit = {}) =>
    new Promise<Response>((repondre, rejeter) => {
      appels.push({ adresse, init, repondre, couper: () => rejeter(new TypeError('Failed to fetch')) })
    })
  return { appels, recuperer }
}

function fauxTransport() {
  const recus: { debut: number; taille: number }[] = []
  const transport: TransportDePdfjs = {
    onDataRange: (debut, morceau) => void recus.push({ debut, taille: morceau?.byteLength ?? 0 }),
    requestDataRange: () => assert.fail('pdf.js ne doit pas garder sa lecture'),
    abort: () => undefined,
  }
  return { transport, recus }
}

const plage = (taille: number, statut = 206) => new Response(new Uint8Array(taille), { status: statut })
const attendre = () => new Promise((fin) => setTimeout(fin, 0))

test('demandé, en route, reçu, progression', async () => {
  const { appels, recuperer } = fauxReseau()
  const { transport, recus } = fauxTransport()
  const suivi = brancherLeTransport(transport, ADRESSE, { recuperer })
  assert.equal(suivi.progression, 0, 'rien de demandé : 0')
  assert.equal(suivi.enAttenteDuReseau, false)

  transport.requestDataRange(0, 262144)
  transport.requestDataRange(262144, 400000)
  assert.equal(suivi.demande, 400000)
  assert.equal(suivi.enRoute, 2)
  assert.equal(suivi.enAttenteDuReseau, true)

  const entetes = new Headers(appels[0]!.init.headers)
  assert.equal(entetes.get('Range'), 'bytes=0-262143', 'la fin de pdf.js est exclue, celle de HTTP incluse')
  assert.equal(appels[0]!.init.credentials, 'include', 'la session accompagne chaque plage')
  assert.equal(new Headers(appels[1]!.init.headers).get('Range'), 'bytes=262144-399999')

  appels[0]!.repondre(plage(262144))
  await attendre()
  assert.equal(suivi.recu, 262144)
  assert.equal(suivi.enRoute, 1)
  assert.equal(suivi.progression, 262144 / 400000)
  assert.deepEqual(recus, [{ debut: 0, taille: 262144 }])

  appels[1]!.repondre(plage(137856))
  await attendre()
  assert.equal(suivi.progression, 1)
  assert.equal(suivi.enAttenteDuReseau, false)
  assert.equal(suivi.erreur, null)
})

test('un 200 à une plage rend une erreur de réseau, et rien n’est donné à pdf.js', async () => {
  const { appels, recuperer } = fauxReseau()
  const { transport, recus } = fauxTransport()
  const suivi = brancherLeTransport(transport, ADRESSE, { recuperer })
  transport.requestDataRange(0, 1000)
  appels[0]!.repondre(plage(5000, 200))
  const erreur = await suivi.echec
  assert.ok(erreur instanceof ErreurDeReseau)
  assert.equal(erreur.statut, 200)
  assert.equal(suivi.erreur, erreur)
  assert.deepEqual(recus, [])
  assert.equal(suivi.enRoute, 0)
})

test('une coupure rend une erreur de réseau, et plus rien ne part ensuite', async () => {
  const { appels, recuperer } = fauxReseau()
  const { transport } = fauxTransport()
  const suivi = brancherLeTransport(transport, ADRESSE, { recuperer })
  transport.requestDataRange(0, 1000)
  appels[0]!.couper()
  const erreur = await suivi.echec
  assert.ok(erreur instanceof ErreurDeReseau)
  assert.equal(erreur.statut, null)
  transport.requestDataRange(1000, 2000)
  assert.equal(appels.length, 1)
})

test('une plage tronquée est une erreur de réseau', async () => {
  const { appels, recuperer } = fauxReseau()
  const { transport } = fauxTransport()
  const suivi = brancherLeTransport(transport, ADRESSE, { recuperer })
  transport.requestDataRange(0, 1000)
  appels[0]!.repondre(plage(600))
  assert.ok((await suivi.echec) instanceof ErreurDeReseau)
})

test('l’abandon par pdf.js coupe ce qui est en route, sans compter d’erreur', async () => {
  const { appels, recuperer } = fauxReseau()
  const { transport } = fauxTransport()
  const suivi = brancherLeTransport(transport, ADRESSE, { recuperer })
  transport.requestDataRange(0, 1000)
  transport.abort()
  assert.equal((appels[0]!.init.signal as AbortSignal).aborted, true)
  appels[0]!.couper()
  await attendre()
  assert.equal(suivi.erreur, null)
})

test('chaque changement est signalé : la demande, le reçu, la fin', async () => {
  const { appels, recuperer } = fauxReseau()
  const { transport } = fauxTransport()
  const vus: { enRoute: number; recu: number }[] = []
  brancherLeTransport(transport, ADRESSE, { recuperer, surChangement: (s) => vus.push({ enRoute: s.enRoute, recu: s.recu }) })
  transport.requestDataRange(0, 10)
  appels[0]!.repondre(plage(10))
  await attendre()
  await attendre()
  assert.deepEqual(vus[0], { enRoute: 1, recu: 0 })
  assert.deepEqual(vus.at(-1), { enRoute: 0, recu: 10 })
  assert.ok(vus.every((v, i) => i === 0 || v.recu >= vus[i - 1]!.recu), 'le reçu ne recule jamais')
})

test('le reçu avance au fil du flux, avant la fin du morceau', async () => {
  const { appels, recuperer } = fauxReseau()
  const { transport, recus } = fauxTransport()
  const suivi = brancherLeTransport(transport, ADRESSE, { recuperer })
  let pousser!: (morceau: Uint8Array | null) => void
  const flux = new ReadableStream<Uint8Array>({
    start(controle) {
      pousser = (morceau) => (morceau ? controle.enqueue(morceau) : controle.close())
    },
  })
  transport.requestDataRange(0, 10)
  appels[0]!.repondre(new Response(flux, { status: 206 }))
  pousser(new Uint8Array(4))
  await attendre()
  await attendre()
  assert.equal(suivi.recu, 4, 'quatre octets arrivés sur dix')
  assert.equal(suivi.enAttenteDuReseau, true)
  assert.equal(recus.length, 0, 'pdf.js ne reçoit le morceau qu’entier')
  pousser(new Uint8Array(6))
  pousser(null)
  for (let i = 0; i < 5; i++) await attendre()
  assert.equal(suivi.recu, 10)
  assert.equal(suivi.enAttenteDuReseau, false)
  assert.equal(recus.length, 1)
})

test('la taille se lit par HEAD, avec la session', async () => {
  let vu: RequestInit | undefined
  const taille = await lireLaTaille(ADRESSE, async (_adresse, init) => {
    vu = init
    return new Response(null, { status: 200, headers: { 'Content-Length': '2884088' } })
  })
  assert.equal(taille, 2884088)
  assert.equal(vu?.method, 'HEAD')
  assert.equal(vu?.credentials, 'include')
})

test('une taille illisible est une erreur de réseau', async () => {
  const refus = (statut: number, longueur?: string) => async () =>
    new Response(null, { status: statut, headers: longueur ? { 'Content-Length': longueur } : {} })
  await assert.rejects(lireLaTaille(ADRESSE, refus(403, '10')), (e) => e instanceof ErreurDeReseau && e.statut === 403)
  await assert.rejects(lireLaTaille(ADRESSE, refus(200)), ErreurDeReseau)
  await assert.rejects(
    lireLaTaille(ADRESSE, async () => Promise.reject(new TypeError('Failed to fetch'))),
    ErreurDeReseau,
  )
})
