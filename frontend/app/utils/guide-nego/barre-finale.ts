/**
 * `/guide-nego` sans barre finale est hors de la portée du service worker, `/guide-nego/` :
 * rechargée sans réseau, elle ne s'ouvre pas. Le serveur la redirige, le routeur aussi.
 */
export function sansBarreFinale(chemin: string, base = '/'): boolean {
  return chemin === `${base.replace(/\/+$/, '')}/guide-nego`
}

export function avecBarreFinale(chemin: string, recherche = ''): string {
  return `${chemin}/${recherche}`
}
