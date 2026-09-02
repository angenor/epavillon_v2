/**
 * Préfixe un fichier de `public/` par la base sous laquelle le site est servi.
 *
 * Un `src="/logos/…"` écrit en dur vise la RACINE du domaine. Servi sous `/v2`,
 * il ne tombe pas dans le vide : il tombe sur **l'autre application** hébergée
 * à cette adresse. Le symptôme est un logo manquant, la cause est ailleurs, et
 * on la cherche longtemps.
 *
 * Ne vaut que pour les fichiers de `public/`. Les liens de navigation passent
 * par `NuxtLink`, que le routeur préfixe déjà ; les médias téléversés portent
 * l'adresse que la base leur compose.
 */
export function assetUrl(chemin: string): string {
  const base = useRuntimeConfig().app.baseURL.replace(/\/$/, '')
  return `${base}${chemin.startsWith('/') ? chemin : `/${chemin}`}`
}
