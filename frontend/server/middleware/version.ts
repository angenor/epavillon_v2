/**
 * La version servie, interrogée par les pages déjà ouvertes (`useAppVersion`),
 * à `<base>/version.json`.
 *
 * `build` est l'instant de construction de CE serveur : un onglet ouvert sur une
 * construction différente sait qu'une nouvelle version est en ligne.
 * `forceBefore` (variable `NUXT_FORCE_RELOAD_BEFORE`, posée par
 * `deploy.sh update --imposer`) : tout onglet construit avant cet instant se
 * recharge sans attendre l'avis de la personne.
 *
 * Intercepteur plutôt que route : une route GET typée par Nitro change le type
 * de retour de `$fetch` dans tout le site. Hors de `/api`, que le relais de
 * production envoie à l'API et non au site.
 */
export default defineEventHandler((event) => {
  const config = useRuntimeConfig(event)
  const path = `${config.app.baseURL.replace(/\/$/, '')}/version.json`
  if (getRequestURL(event).pathname !== path) return
  setResponseHeader(event, 'cache-control', 'no-store')
  return { build: config.public.buildTime, forceBefore: config.forceReloadBefore || null }
})
