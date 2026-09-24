import {
  FORMAT_DE_COPIE,
  magasinEnMemoire,
  type CacheDeDocuments,
  type Copie,
  type DemandeDeTelechargement,
  type Depots,
  type LesCaches,
} from '../../app/utils/guide-nego/copies.ts'

/** Cache Storage en mémoire : des noms, des clés, des réponses. `plein` fait échouer le n-ième `put`. */
export function fauxCaches(options: { refuserAuPut?: number } = {}) {
  const noms = new Map<string, Map<string, Response>>()
  let puts = 0
  const ouvrir = async (nom: string): Promise<CacheDeDocuments> => {
    if (!noms.has(nom)) noms.set(nom, new Map())
    const entrees = noms.get(nom)!
    return {
      lire: async (cle) => entrees.get(cle)?.clone() ?? null,
      contient: async (cle) => entrees.has(cle),
      poser: async (cle, reponse) => {
        puts += 1
        if (options.refuserAuPut === puts) throw new Error('QuotaExceededError')
        entrees.set(cle, reponse)
      },
      retirer: async (cle) => void entrees.delete(cle),
      cles: async () => [...entrees.keys()],
    }
  }
  const caches: LesCaches = { ouvrir, supprimer: async (nom) => void noms.delete(nom) }
  return { caches, noms }
}

export function fauxDepots(options: { refuserAuPut?: number } = {}) {
  const { caches, noms } = fauxCaches(options)
  const depots: Depots = {
    caches,
    copies: magasinEnMemoire<Copie>(),
    aTelecharger: magasinEnMemoire<DemandeDeTelechargement>(),
  }
  const clesDe = (nom: string) => [...(noms.get(nom)?.keys() ?? [])].sort()
  return { depots, noms, clesDe }
}

export const reponse = (corps = '{}') => new Response(corps, { headers: { 'Content-Type': 'application/json' } })

/** Une copie de format 2 et ses entrées : la forme lisible, puis le PDF. */
export function copieDe(id: string, options: { reserve?: boolean; etag?: string; version?: string } = {}) {
  const lecture = `https://api.test/negotiation/documents/${id}/reading`
  const pdf = `https://api.test/negotiation/documents/${id}/file`
  const copie: Copie = {
    id,
    format: FORMAT_DE_COPIE,
    version: options.version ?? '2025',
    reading_etag: options.etag ?? `"${id}-1"`,
    reserve: options.reserve ?? false,
    gardee_a: '2026-11-12T08:00:00.000Z',
    octets: 1000,
    cles: [lecture, pdf],
  }
  const entrees = [
    { cle: lecture, reponse: reponse() },
    { cle: pdf, reponse: new Response(new Uint8Array(998), { headers: { 'Content-Type': 'application/pdf' } }) },
  ]
  return { copie, entrees }
}

/** Un `localStorage` en mémoire, au format de `appareil-lecture.ts`. */
export function fauxStockage(depart: Record<string, string> = {}) {
  const valeurs = new Map(Object.entries(depart))
  return {
    lire: (cle: string) => valeurs.get(cle) ?? null,
    poser: (cle: string, valeur: string) => void valeurs.set(cle, valeur),
    valeurs,
  }
}
