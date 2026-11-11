import type { IsoDateTime } from '~/types/shared'

/**
 * L'horloge d'un direct, en millisecondes, relue toutes les 30 s une fois monté.
 * `from` (l'heure de composition de l'API) fait rendre le même texte au serveur
 * et au navigateur : sans lui, « depuis 45 min » et « depuis 46 min » se
 * contredisent à l'hydratation.
 */
export function useLiveClock(from?: IsoDateTime) {
  const now = ref(from ? Date.parse(from) : Date.now())
  let ticker: ReturnType<typeof setInterval> | undefined
  onMounted(() => {
    now.value = Date.now()
    ticker = setInterval(() => (now.value = Date.now()), 30_000)
  })
  onBeforeUnmount(() => clearInterval(ticker))
  return now
}
