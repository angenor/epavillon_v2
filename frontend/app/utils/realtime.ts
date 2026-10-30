/**
 * La part de `useRealtime()` qui se teste sans navigateur ni Nuxt : grammaire
 * des sujets, découpage du flux SSE, attente croissante, lecture du pouls.
 */

/** Même grammaire que `platform.signal()` et `kernel::realtime::Topic`. */
const TOPIC = /^[a-z][a-z_]*(:[a-z0-9_-]+)+$/
const PUBLIC_TOPIC = /^edition:[a-z0-9_-]+:public(:|$)/

export const isValidTopic = (topic: string): boolean => topic.length <= 200 && TOPIC.test(topic)

/** Ouvert à tous, sans session : suivi par le pouls, relu avec un délai au hasard. */
export const isPublicTopic = (topic: string): boolean => PUBLIC_TOPIC.test(topic)

export interface SseEvent {
  name: string
  data: string
}

/** Découpe ce qui est arrivé en événements complets ; `rest` attend la suite. */
export function splitSse(buffer: string): { events: SseEvent[]; rest: string } {
  const events: SseEvent[] = []
  let rest = buffer
  let cut = rest.indexOf('\n\n')
  while (cut >= 0) {
    const block = rest.slice(0, cut)
    rest = rest.slice(cut + 2)
    let name = 'message'
    const data: string[] = []
    let filled = false
    for (const row of block.split('\n')) {
      if (row.startsWith(':')) continue
      if (row.startsWith('event:')) {
        name = row.slice(6).trim()
        filled = true
      } else if (row.startsWith('data:')) {
        data.push(row.slice(5).trimStart())
        filled = true
      }
    }
    // Un battement (`: battement`) ou un `retry:` ne sont pas des événements.
    if (filled) events.push({ name, data: data.join('\n') })
    cut = rest.indexOf('\n\n')
  }
  return { events, rest }
}

/** Gigue complète sur 1, 2, 4… 30 s au plus : les clients ne reviennent pas ensemble. */
export function backoffMs(failures: number, random: () => number = Math.random): number {
  const ceiling = Math.min(30_000, 1000 * 2 ** Math.max(0, failures))
  return 250 + Math.floor(random() * ceiling)
}

/**
 * Les sujets à relire d'après un pouls. Un sujet vu pour la première fois
 * pose sa version sans rien relire ; un `boot` qui change fait tout relire ;
 * `owed` force la relecture (sujets passés au pouls après une coupure).
 */
export function changedTopics(
  topics: string[],
  previous: { boot: string | null; versions: ReadonlyMap<string, number> },
  pulse: { boot: string; versions: Record<string, number> },
  owed: ReadonlySet<string> = new Set(),
): string[] {
  const restarted = previous.boot !== null && previous.boot !== pulse.boot
  return topics.filter((topic) => {
    const version = pulse.versions[topic]
    if (version === undefined) return false
    const known = previous.versions.get(topic)
    return restarted || owed.has(topic) || (known !== undefined && known !== version)
  })
}
