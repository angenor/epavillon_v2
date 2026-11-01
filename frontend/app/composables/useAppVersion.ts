/**
 * Nouvelle version en ligne : la page ouverte interroge `version.json` toutes les
 * cinq minutes et à chaque retour sur l'onglet. Construction différente : un
 * bandeau propose de recharger (`stale`) ; construction antérieure à l'instant
 * imposé par la mise en ligne : rechargement d'office après un décompte
 * (`forced`). État de module, navigateur seulement : en rendu serveur il serait
 * partagé entre visiteurs.
 */

const INTERVAL_MS = 5 * 60 * 1000

const stale = ref(false)
const forced = ref(false)
const dismissed = ref(false)
let started = false

interface VersionInfo {
  build: string
  forceBefore: string | null
}

export function useAppVersion() {
  const config = useRuntimeConfig()

  async function check(): Promise<void> {
    if (forced.value) return
    try {
      const url = `${config.app.baseURL.replace(/\/$/, '')}/version.json`
      const info = await $fetch<VersionInfo>(url, { cache: 'no-store', retry: 0 })
      if (!info.build || info.build === config.public.buildTime) return
      stale.value = true
      if (info.forceBefore && config.public.buildTime < info.forceBefore) forced.value = true
    } catch {
      // Serveur en cours de redémarrage pendant une mise en ligne : on réessaiera.
    }
  }

  function start(): void {
    if (started || import.meta.server) return
    started = true
    window.setInterval(check, INTERVAL_MS)
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') void check()
    })
  }

  function reload(): void {
    window.location.reload()
  }

  return { stale: readonly(stale), forced: readonly(forced), dismissed, check, start, reload }
}
