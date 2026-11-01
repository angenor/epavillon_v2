/**
 * Démarre la surveillance de version, et fait de la navigation suivante un vrai
 * chargement quand une nouvelle version est en ligne : qui a choisi « Plus tard »
 * passe à la nouvelle version en changeant de page, sans rien perdre.
 */
export default defineNuxtPlugin(() => {
  const { stale, start } = useAppVersion()
  start()

  const router = useRouter()
  router.beforeEach((to, from) => {
    if (!stale.value || to.path === from.path) return
    window.location.assign(router.resolve(to).href)
    return false
  })
})
