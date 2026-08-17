/**
 * Middleware `auth` — réserve une page aux personnes connectées.
 *
 * Écrit avec le prompt A1 parce que la page de connexion doit savoir où renvoyer
 * ensuite ; il sera posé sur les écrans des prompts A2 et A5, qui en sont les
 * premiers usagers réels.
 *
 * CE N'EST PAS UN CONTRÔLE DE SÉCURITÉ. Un middleware de navigation s'exécute
 * dans le navigateur : il empêche d'AFFICHER une page, pas d'obtenir des
 * données. La protection vraie est côté API — `identity.has_permission()`, avec
 * sa portée. Ici on évite un écran vide et un aller-retour inutile.
 *
 * LA DESTINATION VOULUE EST CONSERVÉE dans `?redirect=`, et la page de connexion
 * y ramène après coup. Le nom du paramètre reste technique et non traduit : il
 * est composé par le code et jamais lu par personne.
 */
export default defineNuxtRouteMiddleware(async (to) => {
  const auth = useAuthStore()
  await auth.ensureLoaded()

  if (auth.isAuthenticated) return

  const localePath = useLocalePath()
  return navigateTo({
    path: localePath('/auth/login'),
    query: { redirect: to.fullPath },
  })
})
