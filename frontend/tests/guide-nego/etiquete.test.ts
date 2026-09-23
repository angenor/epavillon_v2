import { test } from 'node:test'
import assert from 'node:assert/strict'
import { creerAppelsEtiquetes, estInchange, type TransportEtiquete } from '../../app/composables/api/etiquete.ts'

/** Un transport qui répond comme le client HTTP : un `304` ne lève pas et n'a pas de corps. */
function transport(reponse: { statut: number; etag: string | null; corps?: unknown }) {
  const options: Record<string, unknown>[] = []
  const http: TransportEtiquete = {
    isConfigured: { value: true },
    request: async <T>(_path: string, opts: Record<string, unknown> = {}) => {
      options.push(opts)
      const surReponse = opts.onResponse as (ctx: { response: Pick<Response, 'status' | 'headers'> }) => void
      surReponse({ response: { status: reponse.statut, headers: new Headers(reponse.etag ? { etag: reponse.etag } : {}) } })
      return (reponse.statut === 304 ? undefined : reponse.corps) as T
    },
  }
  return { http, options }
}

const sansExemples = async () => {
  throw new Error('les exemples ne servent pas quand l’API est branchée')
}

test('sans empreinte connue, la lecture est celle de 0c : ni en-tête, ni « inchangé »', async () => {
  const { http, options } = transport({ statut: 200, etag: '"v1"', corps: { documents: [] } })
  const lu = await creerAppelsEtiquetes(http, sansExemples, 0).lireEtiquete('/negotiation/documents', () => ({ valeur: {}, empreinte: null }))
  assert.deepEqual(lu, { valeur: { documents: [] }, empreinte: '"v1"' })
  assert.equal((options[0]?.headers as Record<string, string> | undefined)?.['If-None-Match'], undefined)
})

test('avec une empreinte, elle part en If-None-Match, et un 304 rend « inchangé »', async () => {
  const { http, options } = transport({ statut: 304, etag: '"v1"' })
  const lu = await creerAppelsEtiquetes(http, sansExemples, 0).lireEtiquete('/negotiation/documents', () => ({ valeur: {}, empreinte: null }), '"v1"')
  assert.deepEqual((options[0]?.headers as Record<string, string>)['If-None-Match'], '"v1"')
  assert.ok(estInchange(lu))
  assert.equal(lu.empreinte, '"v1"')
})

test('une liste changée revient entière, avec sa nouvelle empreinte', async () => {
  const { http } = transport({ statut: 200, etag: '"v2"', corps: { documents: [1] } })
  const lu = await creerAppelsEtiquetes(http, sansExemples, 0).lireEtiquete('/negotiation/documents', () => ({ valeur: {}, empreinte: null }), '"v1"')
  assert.ok(!estInchange(lu))
  assert.deepEqual(lu, { valeur: { documents: [1] }, empreinte: '"v2"' })
})

test('hors ligne, le jeu d’exemple dit « inchangé » quand son empreinte est la même', async () => {
  const http: TransportEtiquete = { isConfigured: { value: false }, request: async () => assert.fail('pas de réseau') }
  const exemples = async <T>(f: (m: never) => T | Promise<T>) => f(undefined as never)
  const appels = creerAppelsEtiquetes(http, exemples, 0)
  const fixe = () => ({ valeur: { documents: [] }, empreinte: '"x"' })
  assert.ok(estInchange(await appels.lireEtiquete('/negotiation/documents', fixe, '"x"')))
  assert.ok(!estInchange(await appels.lireEtiquete('/negotiation/documents', fixe, '"y"')))
})
