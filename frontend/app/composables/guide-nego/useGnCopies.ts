/**
 * Les documents gardés sur le téléphone, branchés sur l'application.
 *
 * Les règles vivent dans `utils/guide-nego/copies.ts` et `a-telecharger.ts`, pures
 * et testées. Ici, ce que le navigateur seul fournit : Cache Storage, le flux du
 * téléchargement, `navigator.storage`. **Complet ou rien** (FR-026) : tout se lit en
 * mémoire, et rien ne s'écrit avant le dernier octet.
 *
 * L'état vit au niveau du module, sur le client seulement : le serveur n'a ni cache
 * ni copie, et les contrôleurs d'annulation ne se sérialisent pas.
 */
import type { DocumentLibrary, DocumentReading, LibraryDocument } from '~/types/negotiation-documents'
import { creerFileATelecharger, type FileATelecharger, type IssueDeTelechargement } from '~/utils/guide-nego/a-telecharger'
import { CLE_PERSISTANCE, oublierLesVersionsDisparues } from '~/utils/guide-nego/appareil-lecture'
import {
  appliquerLaReconciliation,
  cacheDe,
  cleDeCopie,
  copieIntacte,
  demanderLaPersistance,
  effacerLesReserves,
  FORMAT_DE_COPIE,
  garderUneCopie,
  RECONCILIATION_VIDE,
  relireLaPersistance,
  remplacerLaLecture,
  retirerUneCopie,
  toutRetirer as toutRetirerDesDepots,
  verifierLesCopies,
  type CacheDeDocuments,
  type Copie,
  type DemandeDeTelechargement,
  type Depots,
  type Entree,
  type LesCaches,
  type Persistance,
  type Reconciliation,
} from '~/utils/guide-nego/copies'
import { lireEnPersonne } from '~/utils/guide-nego/effacements'
import { estUneFormeLisible } from '~/utils/guide-nego/forme-lisible'
import { magasinATelecharger, magasinDesCopies } from '~/utils/guide-nego/garde'
import { lireCle, poserCle } from '~/utils/guide-nego/stockage'

export type IssueDemandee = IssueDeTelechargement | 'en-attente' | 'impossible'

export interface Progression {
  recus: number
  /** Nul quand ni la liste ni la réponse ne disent la taille. */
  total: number | null
}

/** Des refus passagers : ils se rejouent, la demande reste. */
const STATUTS_DE_PANNE = new Set([408, 425, 429])

const CACHE_MUET: CacheDeDocuments = {
  lire: async () => null,
  contient: async () => false,
  poser: async () => {
    throw new Error('Cache Storage indisponible')
  },
  retirer: async () => undefined,
  cles: async () => [],
}

/** Cache Storage, sans jamais lever sauf à l'écriture — celle-ci doit se savoir refusée. */
function cachesDuNavigateur(): LesCaches {
  if (typeof caches === 'undefined') return { ouvrir: async () => CACHE_MUET, supprimer: async () => undefined }
  return {
    ouvrir: async (nom) => {
      const cache = await caches.open(nom).catch(() => null)
      if (!cache) return CACHE_MUET
      return {
        lire: async (cle) => (await cache.match(cle).catch(() => undefined)) ?? null,
        contient: async (cle) => (await cache.keys(cle).catch(() => [])).length > 0,
        poser: (cle, reponse) => cache.put(cle, reponse),
        retirer: async (cle) => void (await cache.delete(cle).catch(() => false)),
        cles: async () => (await cache.keys().catch(() => [])).map((requete) => requete.url),
      }
    },
    supprimer: async (nom) => void (await caches.delete(nom).catch(() => false)),
  }
}

const depots = (): Depots => ({
  caches: cachesDuNavigateur(),
  copies: magasinDesCopies,
  aTelecharger: magasinATelecharger,
})

// Garder, vérifier, rapprocher et effacer se font l'un après l'autre : la
// vérification prendrait pour orphelines les entrées d'une copie en train d'être posée.
let verrou: Promise<unknown> = Promise.resolve()
function enSerie<T>(travail: () => Promise<T>): Promise<T> {
  const suite = verrou.then(travail, travail)
  verrou = suite.catch(() => undefined)
  return suite
}

const copies = ref<Copie[]>([])
const progressions = ref<Record<string, Progression>>({})
const persistance = ref<Persistance | null>(null)
/** Les documents demandés sans réseau, qui partiront à son retour. */
const enAttente = ref<string[]>([])
const enCours = new Map<string, { controleur: AbortController; reserve: boolean; fin: Promise<IssueDeTelechargement> }>()
let file: FileATelecharger | null = null
// La bibliothèque relue juste avant que la file ne parte : ce qu'elle sert fait foi.
let servisPourLaFile = new Map<string, LibraryDocument>()
// Avance à chaque effacement des réservés : une copie réservée lue avant ne s'écrit plus.
let generationDesReserves = 0
// Avance à chaque « Tout retirer » : un téléchargement parti avant ne s'écrit plus.
let generationDesCopies = 0

async function recharger(): Promise<void> {
  copies.value = (await magasinDesCopies.lire()) ?? copies.value
  enAttente.value = ((await magasinATelecharger.lire()) ?? []).map((d) => d.id)
}

/**
 * Déconnexion, session finie, accès perdu : les réservés s'effacent, et un
 * téléchargement réservé en cours ne s'écrira pas. Ne lève jamais. **Jamais sur une
 * API muette** : l'appelant n'appelle qu'après une réponse.
 */
export async function effacerLesCopiesReservees(): Promise<void> {
  if (!import.meta.client) return
  generationDesReserves += 1
  for (const { controleur, reserve } of enCours.values()) if (reserve) controleur.abort()
  await enSerie(() => effacerLesReserves(depots())).catch(() => undefined)
  await recharger().catch(() => undefined)
}

async function lireEnEntier(reponse: Response, recu: (octets: number) => void): Promise<Uint8Array<ArrayBuffer>> {
  if (!reponse.body) {
    const tout = new Uint8Array(await reponse.arrayBuffer())
    recu(tout.byteLength)
    return tout
  }
  const lecteur = reponse.body.getReader()
  const morceaux: Uint8Array[] = []
  let total = 0
  for (;;) {
    const { done, value } = await lecteur.read()
    if (done) break
    morceaux.push(value)
    total += value.byteLength
    recu(value.byteLength)
  }
  const tout = new Uint8Array(total)
  let decalage = 0
  for (const morceau of morceaux) {
    tout.set(morceau, decalage)
    decalage += morceau.byteLength
  }
  return tout
}

export function useGnCopies() {
  const api = useApi().guideNegoDocuments
  const { rotation } = useApi()
  const connexion = useGnConnexion()
  const session = useGnSession()
  const acces = useGnAcces()
  // Un réservé refusé à une personne dont ce téléphone sait l'accès ouvert : le jeton
  // d'accès a expiré, et une lecture publique ne reçoit pas de 401 pour le dire.
  const accesConnu = () => session.connectee.value && acces.ouvert.value
  const baseDeLApi = String(useRuntimeConfig().public.apiBase ?? '')
  const cleDe = (chemin: string): string => cleDeCopie(chemin, baseDeLApi, window.location.href)

  function avancer(id: string, octets: number): void {
    const courante = progressions.value[id]
    if (!courante) return
    const recus = courante.total === null ? courante.recus + octets : Math.min(courante.total, courante.recus + octets)
    progressions.value = { ...progressions.value, [id]: { ...courante, recus } }
  }

  /**
   * Une ressource en entier, comptée dans la progression. Sans taille annoncée par la
   * liste, chaque réponse ajoute la sienne au total : la lecture, puis le PDF.
   */
  async function lireLaRessource(chemin: string, signal: AbortSignal, id: string, totalAnnonce: boolean) {
    let reponse = await api.ressource(chemin, signal)
    if (reponse?.status === 403 && accesConnu()) {
      const issue = await rotation()
      if (issue === 'injoignable') return { refus: false }
      if (issue === 'renouvelee') reponse = await api.ressource(chemin, signal)
    }
    if (!reponse) return null
    if (!reponse.ok) return { refus: reponse.status < 500 && !STATUTS_DE_PANNE.has(reponse.status) }
    ajouterAuTotal(id, totalAnnonce ? 0 : Number(reponse.headers.get('content-length')))
    const octets = await lireEnEntier(reponse, (n) => avancer(id, n))
    return { octets, reponse }
  }

  function ajouterAuTotal(id: string, octets: number): void {
    const courante = progressions.value[id]
    if (!courante || !(octets > 0)) return
    progressions.value = { ...progressions.value, [id]: { ...courante, total: (courante.total ?? 0) + octets } }
  }

  /** Sans API, le PDF des documents d'exemple : un fichier du site, lu en entier. */
  async function lireLePdfDExemple(signal: AbortSignal, id: string): Promise<Uint8Array<ArrayBuffer>> {
    const reponse = await fetch(api.adresseDuFichier(id), { signal })
    if (!reponse.ok) throw new Error(`PDF d'exemple : ${reponse.status}`)
    ajouterAuTotal(id, Number(reponse.headers.get('content-length')))
    return lireEnEntier(reponse, (n) => avancer(id, n))
  }

  async function executer(demande: DemandeDeTelechargement, controleur: AbortController): Promise<IssueDeTelechargement> {
    const { id, reserve } = demande
    const signal = controleur.signal
    const generation = generationDesReserves
    const generationGardee = generationDesCopies
    progressions.value = { ...progressions.value, [id]: { recus: 0, total: demande.octets } }
    const totalAnnonce = demande.octets !== null
    try {
      const chemin = api.cheminDeLaLecture(id)
      const lue = await lireLaRessource(chemin, signal, id, totalAnnonce)
      let octets: Uint8Array<ArrayBuffer>
      let empreinte: string | null
      if (lue === null) {
        // Sans API : la forme lisible du jeu d'exemple.
        const exemple = await api.lecture(id)
        octets = new TextEncoder().encode(JSON.stringify(exemple.valeur))
        empreinte = exemple.empreinte
      } else if ('refus' in lue) {
        return lue.refus ? 'refus' : 'panne'
      } else {
        octets = lue.octets
        empreinte = lue.reponse.headers.get('etag')
      }
      const lecture = JSON.parse(new TextDecoder().decode(octets)) as DocumentReading

      // Le fichier entier, jamais une plage : `cache.put` refuse un 206.
      const cheminDuPdf = api.cheminDuFichier(id)
      let pdf: Uint8Array<ArrayBuffer>
      if (lue === null) {
        pdf = await lireLePdfDExemple(signal, id)
      } else {
        const fichier = await lireLaRessource(cheminDuPdf, signal, id, totalAnnonce)
        if (!fichier || 'refus' in fichier) return fichier?.refus ? 'refus' : 'panne'
        pdf = fichier.octets
      }

      const entrees: Entree[] = [
        {
          cle: cleDe(chemin),
          reponse: new Response(octets, {
            headers: { 'Content-Type': 'application/json', ...(empreinte ? { ETag: empreinte } : {}) },
          }),
        },
        {
          cle: cleDe(cheminDuPdf),
          reponse: new Response(pdf, { headers: { 'Content-Type': 'application/pdf' } }),
        },
      ]
      const copie: Copie = {
        id,
        format: FORMAT_DE_COPIE,
        version: lecture.version,
        reading_etag: empreinte,
        reserve,
        gardee_a: new Date().toISOString(),
        octets: octets.byteLength + pdf.byteLength,
        cles: [entrees[0]!.cle, entrees[1]!.cle],
      }
      // Sous le verrou, et au dernier moment : ni annulé, ni réservé effacé entre-temps.
      const gardee = await enSerie(async (): Promise<IssueDeTelechargement> => {
        if (signal.aborted || generationGardee !== generationDesCopies || (reserve && generation !== generationDesReserves)) {
          return 'annule'
        }
        return (await garderUneCopie(depots(), copie, entrees)) ? 'reussi' : 'place'
      })
      if (gardee !== 'reussi') return gardee

      // Une fois, sans attendre et sans réessayer : il ne compte qu'une copie réussie.
      void api.compterUnTelechargement(id).catch(() => undefined)
      await recharger()
      return 'reussi'
    } catch (erreur) {
      if (signal.aborted) return 'annule'
      return erreur instanceof ApiRequestError ? 'refus' : 'panne'
    } finally {
      const { [id]: _termine, ...restantes } = progressions.value
      progressions.value = restantes
    }
  }

  function lancer(demande: DemandeDeTelechargement): Promise<IssueDeTelechargement> {
    const deja = enCours.get(demande.id)
    if (deja) return deja.fin
    const controleur = new AbortController()
    const fin = executer(demande, controleur).finally(() => enCours.delete(demande.id))
    enCours.set(demande.id, { controleur, reserve: demande.reserve, fin })
    return fin
  }

  if (import.meta.client && !file) {
    file = creerFileATelecharger({
      magasin: magasinATelecharger,
      // Le côté public ou réservé, et la taille, viennent de la liste servie à l'instant.
      telecharger: async (d) => {
        const servi = servisPourLaFile.get(d.id)
        if (!servi || servi.source !== 'file' || !servi.accessible) return 'refus'
        return lancer({ ...d, reserve: servi.restricted, octets: servi.reading_bytes })
      },
    })
    persistance.value = (lireCle(CLE_PERSISTANCE) as Persistance | null) ?? null
  }

  function noterLaPersistance(issue: Persistance): void {
    persistance.value = issue
    poserCle(CLE_PERSISTANCE, issue)
  }

  /**
   * Télécharger un document de la bibliothèque. Sans réseau, la demande attend son
   * retour. Tant que le droit de garder n'est pas accordé, il se demande — **avant
   * tout `await`**, dans le geste de la personne.
   */
  async function telecharger(document: LibraryDocument): Promise<IssueDemandee> {
    if (document.source !== 'file' || !document.accessible) return 'impossible'
    const persistee =
      persistance.value === null || persistance.value === 'refusee'
        ? demanderLaPersistance(navigator.storage).then(noterLaPersistance)
        : Promise.resolve()
    const demande: DemandeDeTelechargement = {
      id: document.id,
      reserve: document.restricted,
      octets: document.reading_bytes,
      demande_a: new Date().toISOString(),
    }
    if (!connexion.etat.value.enLigne) {
      await file?.demander(demande)
      await recharger()
      await persistee
      return 'en-attente'
    }
    const issue = await lancer(demande)
    await persistee
    return issue
  }

  /** Annuler vaut aussi pour la demande en attente : elle ne repartira pas. */
  async function annuler(id: string): Promise<void> {
    enCours.get(id)?.controleur.abort()
    await magasinATelecharger.retirer(id).catch(() => undefined)
    await recharger()
  }

  async function retirer(id: string): Promise<void> {
    await annuler(id)
    await enSerie(() => retirerUneCopie(depots(), id)).catch(() => undefined)
    await recharger()
  }

  /** Vrai si plus rien n'est gardé : un magasin bloqué laisse des copies, et l'écran doit le dire. */
  async function toutRetirer(): Promise<boolean> {
    generationDesCopies += 1
    for (const { controleur } of enCours.values()) controleur.abort()
    await enSerie(() => toutRetirerDesDepots(depots())).catch(() => undefined)
    await recharger().catch(() => undefined)
    return copies.value.length === 0
  }

  /**
   * À chaque ouverture : une copie que le navigateur a vidée redevient « non
   * téléchargée », et un refus de persistance se relit.
   */
  async function verifier(): Promise<string[]> {
    if (persistance.value === 'refusee') noterLaPersistance(await relireLaPersistance(navigator.storage))
    const perdues = await enSerie(() => verifierLesCopies(depots())).catch((): string[] => [])
    await recharger()
    return perdues
  }

  /** À chaque lecture réussie de la liste : dépubliés et réservés sans accès s'effacent. */
  async function rapprocher(bibliotheque: DocumentLibrary): Promise<Reconciliation> {
    const servis = bibliotheque.documents.map((d) => ({
      id: d.id,
      version: d.version,
      restricted: d.restricted,
      accessible: d.accessible,
      reading_etag: d.reading_etag,
    }))
    const r = await enSerie(() => appliquerLaReconciliation(depots(), servis)).catch(
      (): Reconciliation => ({ ...RECONCILIATION_VIDE }),
    )
    for (const id of r.lectureARelire) await relireLaLecture(id)
    // La version d'une copie encore gardée garde sa reprise, même si la liste en nomme une autre.
    oublierLesVersionsDisparues({ lire: lireCle, poser: poserCle }, [...bibliotheque.documents, ...copies.value])
    await recharger()
    return r
  }

  /**
   * Le choix « Texte agrandi » a changé : la lecture seule se relit, le PDF gardé ne
   * bouge pas (règle 6). Sans réseau ou sur un refus, la copie reste telle qu'elle est.
   */
  async function relireLaLecture(id: string): Promise<void> {
    const chemin = api.cheminDeLaLecture(id)
    const lue = await api.ressource(chemin).catch(() => null)
    if (!lue?.ok) return
    const octets = new Uint8Array(await lue.arrayBuffer())
    const empreinte = lue.headers.get('etag')
    await enSerie(() =>
      remplacerLaLecture(depots(), id, {
        reponse: new Response(octets, {
          headers: { 'Content-Type': 'application/json', ...(empreinte ? { ETag: empreinte } : {}) },
        }),
        octets: octets.byteLength,
        empreinte,
      }),
    ).catch(() => false)
  }

  /**
   * La forme lisible et les octets du PDF gardés, **si la copie est entière** et se
   * lit. Sinon la copie se retire et le lecteur la traite en « non téléchargée »,
   * plutôt que d'ouvrir une page blanche — ou de la dire téléchargée ailleurs.
   */
  async function lireLaCopie(
    id: string,
  ): Promise<{ lecture: DocumentReading; pdf: Uint8Array<ArrayBuffer>; reserve: boolean } | null> {
    const copie = await magasinDesCopies.lireUne(id)
    if (!copie) return null
    const lue = await enSerie(async () => {
      const d = depots()
      if (await copieIntacte(d, copie)) {
        const cache = await d.caches.ouvrir(cacheDe(copie.reserve))
        const forme: unknown = await (await cache.lire(copie.cles[0]))?.json().catch(() => null)
        const pdf = await (await cache.lire(copie.cles[1]))?.arrayBuffer().catch(() => null)
        if (estUneFormeLisible(forme) && pdf) return { lecture: forme, pdf: new Uint8Array(pdf) }
      }
      await retirerUneCopie(d, id)
      return null
    }).catch(() => null)
    if (!lue) {
      await recharger()
      return null
    }
    return { ...lue, reserve: copie.reserve }
  }

  /**
   * Fait partir ce qui a été demandé sans réseau. La bibliothèque se relit d'abord :
   * un document dépublié ou devenu réservé depuis la demande part du bon côté, ou
   * pas du tout. Deux déclencheurs à la fois : une fois.
   */
  async function partir(): Promise<void> {
    if (!file || !connexion.etat.value.enLigne) return
    if (!((await magasinATelecharger.lire()) ?? []).length) return
    const servie = await lireEnPersonne(
      () => api.bibliotheque(),
      (lu) => accesConnu() && lu.valeur.documents.some((d) => d.restricted && !d.accessible),
      rotation,
    ).catch(() => null)
    if (!servie) return
    await rapprocher(servie.valeur)
    servisPourLaFile = new Map(servie.valeur.documents.map((d) => [d.id, d]))
    if ((await file.partir()).length) await recharger()
  }

  return {
    copies: readonly(copies),
    progressions: readonly(progressions),
    /** Nul tant qu'aucun téléchargement ne l'a demandée ; « refusée » se dit dans « Mes documents ». */
    persistance: readonly(persistance),
    enAttente: readonly(enAttente),
    recharger,
    telecharger,
    annuler,
    retirer,
    toutRetirer,
    verifier,
    rapprocher,
    lireLaCopie,
    partir,
  }
}
