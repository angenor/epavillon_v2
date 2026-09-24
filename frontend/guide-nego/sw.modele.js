/**
 * Modèle du service worker de Guide Négo. `modules/guide-nego-garde.ts` en tire
 * `guide-nego/sw.js` à la construction, en remplaçant VERSION et LISTE.
 *
 * TROIS RÈGLES COMMANDENT TOUT (research R5, ADR-019) :
 *
 * 1. `install` garde TOUTE la liste, ou échoue. Un service worker actif a donc
 *    toujours un cache complet — jamais une page sans ses fichiers.
 * 2. La navigation est servie CACHE D'ABORD. Le site est redéployé pendant la COP ;
 *    une salle au réseau saturé recevrait la nouvelle page vide, puis attendrait des
 *    fichiers qu'aucun cache ne porte. L'application s'ouvre sur la version gardée, et
 *    la nouvelle se garde en arrière-plan sans s'exécuter.
 * 3. Une version installée ne prend la main QU'AU CHARGEMENT D'UNE PAGE, sur demande
 *    de celle-ci — jamais au milieu d'un usage.
 *
 * Ce qui échappe à cette garde : les appels d'API — le drapeau se relit à chaque
 * ouverture, et l'arrêt d'urgence n'attend pas la mise à jour de l'application.
 */
const VERSION = '__VERSION__'
const LISTE = __LISTE__

/**
 * `gn-coquille-` et non `gn-` : l'étape 1 gardera des documents téléchargés dans ses
 * propres caches, qui ne doivent pas partir au premier déploiement du site. Le ménage
 * ci-dessous ne vise que la coquille.
 */
const PREFIXE = 'gn-coquille-'
const CACHE = `${PREFIXE}${VERSION}`

/** La portée, et la page vide qui sert toute navigation qui y tombe. */
const PORTEE = new URL('./', self.location).href

/** Les adresses gardées, absolues, pour les comparer à celles des requêtes. */
const GARDEES = new Set(LISTE.map((adresse) => new URL(adresse, self.location).href))

/**
 * Même adresse, même contenu : les fichiers de construction portent une empreinte dans
 * leur nom, les ressources de pdf.js sa version dans leur chemin.
 */
const estFichierDeConstruction = (adresse) => adresse.includes('/_nuxt/') || adresse.includes('/guide-nego/pdfjs/')

/**
 * Le manifeste que Nuxt relit d'heure en heure, l'heure dans l'adresse, pour savoir si
 * une version plus récente est en ligne. Il change sous la même adresse : repris d'une
 * version précédente, il annoncerait une mise à jour, et Nuxt rechargerait la page.
 */
const estLeDernierManifeste = (adresse) => adresse.pathname.endsWith('/builds/latest.json')

async function cachesDeLaCoquille() {
  const cles = await caches.keys()
  return cles.filter((cle) => cle.startsWith(PREFIXE) && cle !== CACHE)
}

/**
 * Reprend d'une version précédente ce qui n'a pas changé, et ne demande au réseau que
 * le reste. Un déploiement du site change la version sans changer les fichiers : sans
 * cela, chaque mise en ligne ferait reprendre un mégaoctet, polices comprises, sur un
 * réseau qui ne l'a pas.
 */
async function garderTout(cache) {
  const anciens = await Promise.all((await cachesDeLaCoquille()).map((cle) => caches.open(cle)))

  const aDemander = []
  for (const adresse of GARDEES) {
    // Une installation interrompue par le réseau reprend où elle s'était arrêtée.
    if (await cache.match(adresse)) continue
    const reprenable = estFichierDeConstruction(adresse) && !estLeDernierManifeste(new URL(adresse))
    const reprise = reprenable ? await trouverDansLesAnciens(anciens, adresse) : null
    if (reprise) await cache.put(adresse, reprise)
    else aDemander.push(adresse)
  }

  // `reload` : la garde ne doit rien reprendre du cache HTTP, qui peut porter l'ancienne
  // version sous la même adresse.
  await cache.addAll(aDemander.map((adresse) => new Request(adresse, { cache: 'reload' })))
}

async function trouverDansLesAnciens(anciens, adresse) {
  for (const ancien of anciens) {
    const reponse = await ancien.match(adresse)
    if (reponse) return reponse
  }
  return null
}

self.addEventListener('install', (evenement) => {
  evenement.waitUntil(caches.open(CACHE).then(garderTout))
})

self.addEventListener('activate', (evenement) => {
  evenement.waitUntil(
    cachesDeLaCoquille()
      .then((cles) => Promise.all(cles.map((cle) => caches.delete(cle))))
      .then(() => self.clients.claim()),
  )
})

/**
 * La page demande la main au chargement, et seulement là. Sans cela, une version
 * installée attendrait que l'application soit fermée pour de bon — ce qui, sur un
 * téléphone, peut ne jamais arriver.
 */
self.addEventListener('message', (evenement) => {
  if (evenement.data === 'prendre-la-main') self.skipWaiting()
})

async function cacheDAbord(requete, adresse) {
  const garde = await caches.match(adresse, { cacheName: CACHE })
  if (garde) return garde
  return fetch(requete)
}

/** Le réseau d'abord ; sans lui, la copie de cette version, que rien ne réécrit. */
async function reseauDAbord(requete, adresse) {
  try {
    return await fetch(requete)
  } catch (erreur) {
    const garde = await caches.match(adresse, { cacheName: CACHE })
    if (garde) return garde
    throw erreur
  }
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

  if (estLeDernierManifeste(adresse)) {
    evenement.respondWith(reseauDAbord(requete, `${adresse.origin}${adresse.pathname}`))
    return
  }

  if (GARDEES.has(adresse.href)) evenement.respondWith(cacheDAbord(requete, adresse.href))
})
