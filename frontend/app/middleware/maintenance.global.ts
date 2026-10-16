/**
 * Middleware GLOBAL `maintenance` : la plateforme entière fermée aux
 * non-administrateurs, le temps d'une reprise de données.
 *
 * Commandé par le drapeau `platform.maintenance`, allumé en base sans
 * redéploiement. Restent ouvertes la page d'attente et l'entrée (connexion, mot
 * de passe oublié) : sans elles, aucun administrateur n'entrerait.
 *
 * Même règle que la garde `auth` : une session non tranchée au rendu serveur
 * n'est pas une absence de session. La page se rend alors dans le navigateur,
 * qui tourne le jeton et tranche.
 *
 * L'API refuse de son côté les écritures des non-administrateurs.
 */
const OPEN_ROUTES = new Set(['auth-login', 'auth-forgot-password', 'auth-reset-password'])

export default defineNuxtRouteMiddleware(async (to) => {
  const name = baseRouteName(to.name)
  if (name !== null && OPEN_ROUTES.has(name)) return

  const features = useFeatureStore()
  await features.ensureLoaded()
  const localePath = useLocalePath()
  const onWaitingPage = name === 'maintenance'

  if (!features.isEnabled(PLATFORM_MAINTENANCE_FLAG)) {
    // Rouverte entre-temps : on ne laisse pas annoncer une fermeture finie.
    if (onWaitingPage) return navigateTo(localePath('/'))
    return
  }
  if (onWaitingPage) return

  const auth = useAuthStore()
  await auth.ensureLoaded()
  if (!auth.isResolved) {
    if (import.meta.server) useSessionDeferred().value = true
    return
  }

  if (auth.isAuthenticated) {
    const scope = useAdminScopeStore()
    await scope.ensureLoaded()
    if (scope.canAdminister) return
  }

  return navigateTo(localePath('maintenance'))
})
