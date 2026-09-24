/**
 * Ce que le build legacy de pdf.js appelle sans l'émuler (ADR-022) : posé dans la
 * page ET dans le travailleur, avant pdf.js. `Promise.withResolvers` arrive avec
 * iOS 17.4 ; le remplacer ramène le plancher à iOS 16.4.
 *
 * Pur : l'objet global est reçu, jamais deviné. Une fonction présente n'est jamais remplacée.
 */

export type Remplacement = 'Promise.withResolvers' | 'ReadableStream[Symbol.asyncIterator]'

interface LecteurDeFlux {
  read(): Promise<{ done: boolean; value?: unknown }>
  cancel(raison?: unknown): Promise<void>
  releaseLock(): void
}

interface FluxLisible {
  getReader(): LecteurDeFlux
}

export interface PorteeGlobale {
  Promise: PromiseConstructor
  ReadableStream?: { prototype: object }
}

async function* itererLeFlux(this: FluxLisible): AsyncGenerator<unknown, void, undefined> {
  const lecteur = this.getReader()
  let fini = false
  try {
    for (;;) {
      const { done, value } = await lecteur.read()
      if (done) {
        fini = true
        return
      }
      yield value
    }
  } finally {
    // Une sortie anticipée annule le flux, comme l'itérateur natif.
    if (!fini) await lecteur.cancel().catch(() => undefined)
    lecteur.releaseLock()
  }
}

/** Rend ce qui a été posé, pour le dire en recette. */
export function installerLesRemplacements(portee: PorteeGlobale): Remplacement[] {
  const poses: Remplacement[] = []
  const Promesse = portee.Promise

  if (typeof Promesse.withResolvers !== 'function') {
    Object.defineProperty(Promesse, 'withResolvers', {
      configurable: true,
      writable: true,
      value: function withResolvers<T>(): PromiseWithResolvers<T> {
        let resolve!: (valeur: T | PromiseLike<T>) => void
        let reject!: (raison?: unknown) => void
        const promise = new Promesse<T>((ok, ko) => {
          resolve = ok
          reject = ko
        })
        return { promise, resolve, reject }
      },
    })
    poses.push('Promise.withResolvers')
  }

  const prototype = portee.ReadableStream?.prototype
  if (prototype && typeof (prototype as { [Symbol.asyncIterator]?: unknown })[Symbol.asyncIterator] !== 'function') {
    Object.defineProperty(prototype, Symbol.asyncIterator, {
      configurable: true,
      writable: true,
      value: itererLeFlux,
    })
    poses.push('ReadableStream[Symbol.asyncIterator]')
  }

  return poses
}
