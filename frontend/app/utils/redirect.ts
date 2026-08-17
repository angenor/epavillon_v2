/**
 * Destination de retour reçue de l'EXTÉRIEUR — `?redirect=` dans l'URL.
 *
 * SEULS LES CHEMINS INTERNES SONT ACCEPTÉS. Une valeur commençant par `//` ou
 * portant un schéma (`https:`, `javascript:`) ouvrirait une redirection vers un
 * autre site depuis un lien qui a l'air d'être le nôtre : c'est le motif
 * classique d'un courriel d'hameçonnage pointant vers le vrai domaine de la
 * plateforme. La règle est écrite ICI et à un seul endroit, parce qu'elle a
 * désormais trois appelants — la page de connexion, l'écran de rattachement à
 * une organisation, et la garde qui l'impose avant certaines actions. Une règle
 * de sécurité recopiée trois fois est une règle qui finira par diverger.
 *
 * `redirect` n'est jamais traduit : il est composé par le code et lu par lui
 * seul. Ce qu'il porte, en revanche, est un chemin DÉJÀ localisé.
 */
export function internalRedirect(
  raw: unknown,
  fallback: string,
): string {
  const value = Array.isArray(raw) ? raw[0] : raw
  if (typeof value !== 'string') return fallback
  if (!value.startsWith('/') || value.startsWith('//')) return fallback
  return value
}
