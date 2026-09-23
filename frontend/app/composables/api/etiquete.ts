/**
 * Les appels qui lisent et renvoient l'empreinte de l'état (`ETag`).
 *
 * Écrits à l'étape 0c pour les thématiques suivies, où l'empreinte part en
 * `If-Match` avec un choix pris hors connexion. L'étape 1 y ajoute la relecture
 * conditionnelle : une copie de la bibliothèque gardée sur le téléphone se relit
 * avec `If-None-Match`, et un `304` dit « inchangé » sans renvoyer la liste.
 *
 * **L'API doit exposer `ETag`** (`middleware/cors.rs`) : un en-tête non exposé
 * est caché au code par le navigateur, et `empreinte` serait nulle sans qu'une
 * seule erreur ne le dise.
 *
 * Hors de `http.ts` pour se tester sans Nuxt : le transport et les données
 * simulées sont injectés.
 */

export interface AvecEmpreinte<T> {
  valeur: T
  /** Nulle si la réponse n'en portait pas — l'écriture part alors sans garde. */
  empreinte: string | null
}

/** Le `304` d'une relecture conditionnelle : ce que le téléphone garde est à jour. */
export interface Inchange {
  inchange: true
  empreinte: string
}

export const estInchange = <T>(lu: AvecEmpreinte<T> | Inchange): lu is Inchange => 'inchange' in lu

type Mocks = typeof import('~/mocks')
type Exemples<T> = (m: Mocks) => AvecEmpreinte<T> | Promise<AvecEmpreinte<T>>

export interface TransportEtiquete {
  isConfigured: { value: boolean }
  request: <T>(path: string, options?: Record<string, unknown>) => Promise<T>
}

export type LireLesExemples = <T>(fromMocks: (m: Mocks) => T | Promise<T>, latencyMs: number) => Promise<T>

export interface AppelsEtiquetes {
  lireEtiquete<T>(path: string, fromMocks: Exemples<T>): Promise<AvecEmpreinte<T>>
  lireEtiquete<T>(path: string, fromMocks: Exemples<T>, siDifferent: string | null): Promise<AvecEmpreinte<T> | Inchange>
  ecrireEtiquete<T>(
    path: string,
    body: object,
    fromMocks: Exemples<T>,
    siCorrespond?: string | null,
  ): Promise<AvecEmpreinte<T>>
}

export function creerAppelsEtiquetes(
  http: TransportEtiquete,
  lireLesExemples: LireLesExemples,
  latenceSimuleeMs: number,
): AppelsEtiquetes {
  async function etiquete<T>(path: string, options: Record<string, unknown>): Promise<AvecEmpreinte<T> | Inchange> {
    let empreinte: string | null = null
    let statut = 0
    const valeur = await http.request<T>(path, {
      ...options,
      onResponse: ({ response }: { response: Pick<Response, 'status' | 'headers'> }) => {
        statut = response.status
        empreinte = response.headers.get('etag')
      },
    })
    // Un `304` ne lève pas et n'a pas de corps : c'est son statut qui le dit.
    const demandee = (options.headers as Record<string, string> | undefined)?.['If-None-Match']
    if (statut === 304 && demandee) return { inchange: true, empreinte: empreinte ?? demandee }
    return { valeur, empreinte }
  }

  async function lireEtiquete<T>(
    path: string,
    fromMocks: Exemples<T>,
    siDifferent?: string | null,
  ): Promise<AvecEmpreinte<T> | Inchange> {
    if (!http.isConfigured.value) {
      const lu = await lireLesExemples(fromMocks, latenceSimuleeMs)
      return siDifferent && lu.empreinte === siDifferent ? { inchange: true, empreinte: siDifferent } : lu
    }
    return etiquete<T>(path, siDifferent ? { headers: { 'If-None-Match': siDifferent } } : {})
  }

  return {
    lireEtiquete: lireEtiquete as AppelsEtiquetes['lireEtiquete'],

    /**
     * `retry: 0`, comme toute écriture. `If-Match` absent est accepté par
     * l'API : l'écran en ligne vient de lire, il n'a rien à opposer.
     */
    async ecrireEtiquete<T>(path: string, body: object, fromMocks: Exemples<T>, siCorrespond?: string | null) {
      if (!http.isConfigured.value) return lireLesExemples(fromMocks, latenceSimuleeMs * 3)
      const lu = await etiquete<T>(path, {
        method: 'PUT',
        body: body as Record<string, unknown>,
        retry: 0,
        headers: siCorrespond ? { 'If-Match': siCorrespond } : undefined,
      })
      // Une écriture ne demande jamais `If-None-Match` : `etiquete` ne rend « inchangé » qu'à une lecture.
      return lu as AvecEmpreinte<T>
    },
  }
}
