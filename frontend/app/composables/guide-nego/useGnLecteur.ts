/**
 * Le lecteur : la forme lisible **gardée d'abord**, le réseau ensuite (R14).
 *
 * La copie ne se lit qu'entière (`lireLaCopie` la vérifie) : une copie que le
 * navigateur a vidée redevient « non téléchargée », et sans réseau l'écran dit que
 * le document n'est pas sur le téléphone — **jamais une page blanche**. Une image
 * vidée depuis se relit au réseau ; sans réseau, sa page garde son texte et le dit.
 *
 * La page en cours se repère par un observateur d'intersection, et s'écrit sur le
 * téléphone au plus toutes les deux secondes, par document et par version.
 */
import type { DocumentReading, ReadingPage } from '~/types/negotiation-documents'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { lireProgression, noterOuverture, noterProgression } from '~/utils/guide-nego/appareil-lecture'
import { estUneFormeLisible, pageDeReprise } from '~/utils/guide-nego/forme-lisible'
import { lireCle, poserCle } from '~/utils/guide-nego/stockage'

export type EtatDuLecteur =
  | { etat: 'chargement' }
  | { etat: 'pret'; lecture: DocumentReading; source: 'copie' | 'reseau' }
  /** Pas de copie entière, et pas de réseau pour lire. */
  | { etat: 'absent' }
  | { etat: 'reserve' }
  /** Un lien externe : il s'ouvre dans le navigateur, depuis sa fiche. */
  | { etat: 'lien' }
  | { etat: 'introuvable' }
  /** L'API a refusé autrement : son message, tel quel. */
  | { etat: 'erreur'; message: string | null }

/** Ce que rend la demande d'une image : son adresse, ou pourquoi elle manque. */
export type ImageDePage = { adresse: string } | 'hors-connexion' | 'echec'

export interface Reprise {
  index: number
  label: string
  /** Instant ISO de la dernière page notée. */
  a: string
}

const stockage = { lire: lireCle, poser: poserCle }
const INTERVALLE_DE_NOTE_MS = 2000

export function useGnLecteur(id: Ref<string>) {
  const api = useApi().guideNegoDocuments
  const { rotation } = useApi()
  const copies = useGnCopies()
  const connexion = useGnConnexion()
  const session = useGnSession()
  const acces = useGnAcces()
  // Côté public ou réservé de la copie lue : ses images vivent dans le cache du même côté.
  let reserveDeLaCopie: boolean | null = null

  const etat = ref<EtatDuLecteur>({ etat: 'chargement' })
  const lecture = computed(() => (etat.value.etat === 'pret' ? etat.value.lecture : null))
  const reprise = ref<Reprise | null>(null)
  const pageEnCours = ref(1)

  async function lireAuReseau(): Promise<DocumentReading> {
    try {
      return (await api.lecture(id.value)).valeur
    } catch (erreur) {
      const refus = normalizeApiError(erreur)
      // Un jeton d'accès expiré fait lire en visiteuse : le réservé se refuse sans 401.
      const jetonExpire =
        refus instanceof ApiRequestError &&
        refus.code === 'NEGOTIATION_DOCUMENT_RESTRICTED' &&
        session.connectee.value &&
        acces.ouvert.value
      if (jetonExpire && (await rotation()) === 'renouvelee') return (await api.lecture(id.value)).valeur
      throw erreur
    }
  }

  function etatDuRefus(erreur: unknown): EtatDuLecteur {
    const refus = normalizeApiError(erreur)
    if (!(refus instanceof ApiRequestError)) return { etat: 'absent' }
    if (refus.code === 'NEGOTIATION_DOCUMENT_RESTRICTED') return { etat: 'reserve' }
    if (refus.code === 'NEGOTIATION_DOCUMENT_NOT_READABLE') return { etat: 'lien' }
    if (refus.code === 'NEGOTIATION_DOCUMENT_NOT_FOUND') return { etat: 'introuvable' }
    return { etat: 'erreur', message: refus.message }
  }

  async function ouvrir(): Promise<void> {
    etat.value = { etat: 'chargement' }
    reprise.value = null
    suivi.value = false
    reserveDeLaCopie = null
    const gardee = await copies.lireLaCopie(id.value).catch(() => null)
    try {
      if (gardee) {
        reserveDeLaCopie = gardee.reserve
        pret(gardee.lecture, 'copie')
        return
      }
      if (!connexion.etat.value.enLigne) {
        etat.value = { etat: 'absent' }
        return
      }
      const lue = await lireAuReseau()
      if (estUneFormeLisible(lue)) pret(lue, 'reseau')
      else etat.value = { etat: 'erreur', message: null }
    } catch (erreur) {
      etat.value = etatDuRefus(erreur)
    }
  }

  function pret(lue: DocumentReading, source: 'copie' | 'reseau'): void {
    const notee = lireProgression(stockage, id.value, lue.version)
    const page = pageDeReprise(lue, notee?.page ?? null)
    reprise.value = page && notee ? { index: page.index, label: page.label, a: notee.a } : null
    pageEnCours.value = page?.index ?? 1
    noterOuverture(stockage, id.value, new Date().toISOString())
    etat.value = { etat: 'pret', lecture: lue, source }
  }

  // --- Les images de page -------------------------------------------------------

  const adressesLocales: string[] = []

  function adresseLocale(image: Blob): { adresse: string } {
    const adresse = URL.createObjectURL(image)
    adressesLocales.push(adresse)
    return { adresse }
  }

  /** Au réseau par l'API : l'image d'un réservé se refuse sans 401 quand le jeton a expiré. */
  async function imageAuReseau(chemin: string): Promise<ImageDePage> {
    try {
      let reponse = await api.ressource(chemin)
      if (reponse?.status === 403 && session.connectee.value && acces.ouvert.value && (await rotation()) === 'renouvelee') {
        reponse = await api.ressource(chemin)
      }
      return reponse?.ok ? adresseLocale(await reponse.blob()) : 'echec'
    } catch {
      return connexion.etat.value.enLigne ? 'echec' : 'hors-connexion'
    }
  }

  /** L'image d'une page : gardée si elle l'est, au réseau sinon. */
  async function imageDe(page: ReadingPage): Promise<ImageDePage> {
    if (!page.image) return 'echec'
    if (reserveDeLaCopie !== null) {
      const gardee = await copies.imageDeLaCopie({ reserve: reserveDeLaCopie }, page.image).catch(() => null)
      if (gardee) return adresseLocale(gardee)
    }
    return connexion.etat.value.enLigne ? imageAuReseau(page.image) : 'hors-connexion'
  }

  // --- La page en cours ---------------------------------------------------------

  const visibles = new Set<number>()
  // Tant que l'écran ne s'est pas recalé sur la page de reprise, l'observateur voit la
  // page 1 : la noter effacerait la reprise de la prochaine ouverture.
  const suivi = ref(false)
  let observateur: IntersectionObserver | null = null
  const suivies = new Map<Element, number>()

  function observateurDesPages(): IntersectionObserver | null {
    if (observateur || typeof IntersectionObserver === 'undefined') return observateur
    // Une ligne au tiers de l'écran : la page qui la traverse est celle qu'on lit. Une bande
    // plus large verrait aussi la fin de la page d'avant, et la dirait en cours.
    observateur = new IntersectionObserver(
      (entrees) => {
        for (const entree of entrees) {
          const index = suivies.get(entree.target)
          if (index === undefined) continue
          if (entree.isIntersecting) visibles.add(index)
          else visibles.delete(index)
        }
        if (suivi.value && visibles.size) pageEnCours.value = Math.min(...visibles)
      },
      { rootMargin: '-35% 0px -64% 0px' },
    )
    return observateur
  }

  /** À poser sur l'élément de chaque page : `:ref="(el) => suivreLaPage(el, page.index)"`. */
  function suivreLaPage(element: Element | null, index: number): void {
    if (!element || suivies.has(element)) return
    suivies.set(element, index)
    observateurDesPages()?.observe(element)
  }

  let derniereNote = 0
  let noteDifferee: ReturnType<typeof setTimeout> | undefined

  function noterMaintenant(): void {
    clearTimeout(noteDifferee)
    noteDifferee = undefined
    const lue = lecture.value
    if (!lue) return
    derniereNote = Date.now()
    noterProgression(stockage, id.value, lue.version, pageEnCours.value, new Date().toISOString())
  }

  // Seule la lecture se note : ni l'ouverture, qui pose la page de reprise, ni le recalage.
  watch(pageEnCours, () => {
    if (!lecture.value || !suivi.value) return
    const attente = INTERVALLE_DE_NOTE_MS - (Date.now() - derniereNote)
    if (attente <= 0) noterMaintenant()
    else noteDifferee ??= setTimeout(noterMaintenant, attente)
  })

  // Au bas du document, la dernière page ne monte jamais dans la bande du haut.
  let attenteDuBas = 0
  function auDefilement(): void {
    if (attenteDuBas) return
    attenteDuBas = requestAnimationFrame(() => {
      attenteDuBas = 0
      const derniere = lecture.value?.pages.at(-1)
      const enBas = window.scrollY + window.innerHeight >= window.document.documentElement.scrollHeight - 1
      if (suivi.value && derniere && enBas) pageEnCours.value = derniere.index
    })
  }

  // Le téléphone verrouillé, l'application fermée par le système : la dernière page se note avant.
  function noterSiEnAttente(): void {
    if (noteDifferee) noterMaintenant()
  }
  function enArrierePlan(): void {
    if (window.document.visibilityState === 'hidden') noterSiEnAttente()
  }

  if (import.meta.client) {
    window.addEventListener('scroll', auDefilement, { passive: true })
    window.document.addEventListener('visibilitychange', enArrierePlan)
    window.addEventListener('pagehide', noterSiEnAttente)
  }

  onScopeDispose(() => {
    noterSiEnAttente()
    window.removeEventListener('scroll', auDefilement)
    window.document.removeEventListener('visibilitychange', enArrierePlan)
    window.removeEventListener('pagehide', noterSiEnAttente)
    cancelAnimationFrame(attenteDuBas)
    observateur?.disconnect()
    for (const adresse of adressesLocales) URL.revokeObjectURL(adresse)
  })

  watch(id, () => void ouvrir())

  return {
    etat: readonly(etat),
    lecture,
    reprise: readonly(reprise),
    pageEnCours: readonly(pageEnCours),
    ouvrir,
    imageDe,
    suivreLaPage,
    /**
     * Un saut — sommaire, passage, taille, rotation : l'observateur verrait passer les
     * pages voisines ; la page atteinte se pose, et se note comme une lecture.
     */
    async sauter(index: number, defiler: () => void): Promise<void> {
      suivi.value = false
      defiler()
      await new Promise((fin) => requestAnimationFrame(() => requestAnimationFrame(fin)))
      suivi.value = true
      pageEnCours.value = index
    },
    /** L'écran est recalé sur la page de reprise, ou au début : la page lue se suit désormais. */
    commencerLeSuivi: () => {
      suivi.value = true
    },
    /** En quittant : le retour en haut de l'écran suivant n'est pas une lecture. */
    arreterLeSuivi: () => {
      suivi.value = false
      if (noteDifferee) noterMaintenant()
    },
    /** « Début » : la reprise se tait, et la page 1 devient celle qu'on lit. */
    oublierLaReprise: () => (reprise.value = null),
  }
}
