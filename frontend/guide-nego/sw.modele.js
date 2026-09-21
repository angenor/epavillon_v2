/**
 * Modèle du service worker de Guide Négo. `modules/guide-nego-garde.ts` en tire
 * `guide-nego/sw.js` à la construction, en remplaçant VERSION et LISTE.
 *
 * DEUX RÈGLES COMMANDENT TOUT (research R5) :
 *
 * 1. `install` garde TOUTE la liste, ou échoue. Un service worker actif a donc
 *    toujours un cache complet — jamais une page sans ses fichiers.
 * 2. La navigation est servie CACHE D'ABORD. Le site est redéployé pendant la COP ;
 *    une salle au réseau saturé recevrait la nouvelle page vide, puis attendrait des
 *    fichiers qu'aucun cache ne porte. L'application s'ouvre sur la version gardée, et
 *    la nouvelle se garde en arrière-plan sans s'exécuter.
 *
 * Ce qui échappe à cette garde : les appels d'API — le drapeau se relit à chaque
 * ouverture, et l'arrêt d'urgence n'attend pas la mise à jour de l'application.
 */
const VERSION = '__VERSION__'
const LISTE = __LISTE__

const CACHE = `gn-${VERSION}`
/** La portée, et la page vide qui sert toute navigation qui y tombe. */
const PORTEE = new URL('./', self.location).href

/** Les adresses gardées, absolues, pour les comparer à celles des requêtes. */
const GARDEES = new Set(LISTE.map((adresse) => new URL(adresse, self.location).href))

self.addEventListener('install', (evenement) => {
  // `reload` : la garde ne doit rien reprendre du cache HTTP, qui peut porter l'ancienne
  // version sous la même adresse.
  evenement.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll([...GARDEES].map((adresse) => new Request(adresse, { cache: 'reload' })))),
  )
})

self.addEventListener('activate', (evenement) => {
  evenement.waitUntil(
    caches
      .keys()
      .then((cles) => Promise.all(cles.filter((cle) => cle.startsWith('gn-') && cle !== CACHE).map((cle) => caches.delete(cle))))
      .then(() => self.clients.claim()),
  )
})

async function cacheDAbord(requete, adresse) {
  const garde = await caches.match(adresse, { cacheName: CACHE })
  if (garde) return garde
  return fetch(requete)
}

self.addEventListener('fetch', (evenement) => {
  const requete = evenement.request
  if (requete.method !== 'GET') return

  const adresse = new URL(requete.url)
  if (adresse.origin !== self.location.origin) return

  if (requete.mode === 'navigate') {
    if (!adresse.href.startsWith(PORTEE)) return
    evenement.respondWith(cacheDAbord(requete, PORTEE))
    return
  }

  if (GARDEES.has(adresse.href)) evenement.respondWith(cacheDAbord(requete, adresse.href))
})
