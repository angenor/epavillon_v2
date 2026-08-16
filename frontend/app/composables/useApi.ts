/**
 * Point d'entrée UNIQUE vers l'API.
 *
 * Règle de projet : aucune page n'importe un mock et aucune page n'appelle
 * `$fetch` directement — tout passe par ici. Tant que l'API n'existe pas
 * (prompt B7), les écrans travaillent sur les données simulées de `app/mocks/`
 * et ce composable n'est appelé nulle part : il est déclaré maintenant pour que
 * le raccordement ne demande aucune réécriture des écrans.
 *
 * L'adresse vient de `NUXT_PUBLIC_API_BASE`, lue dans le `.env` de la racine du
 * dépôt (les scripts npm passent `--dotenv ../.env`).
 */
export function useApi() {
  const config = useRuntimeConfig()
  const baseURL = config.public.apiBase

  const { locale } = useI18n()

  const client = $fetch.create({
    baseURL,
    // Les cookies de session sont posés par l'API : la requête doit les porter.
    credentials: 'include',
    retry: 1,
    retryStatusCodes: [408, 409, 425, 429, 500, 502, 503, 504],
    onRequest({ options }) {
      // L'API résout les colonnes `platform.i18n_text` selon cet en-tête.
      options.headers.set('Accept-Language', locale.value)
    },
  })

  return {
    /** Adresse de base de l'API, telle que configurée. */
    baseURL,
    /** Client HTTP préconfiguré — à utiliser à la place de `$fetch`. */
    client,
    /** L'API est-elle configurée ? Faux tant que la variable n'est pas posée. */
    isConfigured: computed(() => baseURL.length > 0),
  }
}
