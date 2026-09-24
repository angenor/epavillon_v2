/**
 * La bibliothèque des documents, lue par le réseau et gardée pour le hors connexion.
 *
 * La liste se relit avec son empreinte : un `304` garde ce que le téléphone a, sans
 * rien retélécharger. Chaque réponse de l'API, changée ou non, rapproche les copies
 * de ce qu'elle sert (FR-034) — une API muette ne rapproche rien, donc n'efface rien.
 *
 * La liste gardée porte la personne pour qui elle a été servie : après une
 * déconnexion, les réservés s'y lisent comme l'API les servirait à qui n'a pas
 * l'accès.
 *
 * Les notes de correction se relisent avec la liste, par leur propre empreinte : une
 * note posée paraît sur la copie gardée sans la retélécharger (FR-050).
 *
 * La recherche dans le texte ne se fait qu'en ligne : hors connexion, on cherche
 * dans les titres, résumés et éditeurs gardés.
 */
import type { CorrectionNote, DocumentLibrary, LibraryDocument, PageHit } from '~/types/negotiation-documents'
import { estInchange } from '~/composables/api/etiquete'
import { estNouveau, marquerVu } from '~/utils/guide-nego/appareil-lecture'
import { pourUneAutrePersonne } from '~/utils/guide-nego/documents'
import { lireEnPersonne } from '~/utils/guide-nego/effacements'
import { lireCle, poserCle } from '~/utils/guide-nego/stockage'

interface BibliothequeGardee {
  bibliotheque: DocumentLibrary
  empreinte: string | null
  /** Les titres arrivent résolus : une autre langue ne se relit pas sous la même empreinte. */
  langue: string
  /** Nulle sans compte. */
  personne: string | null
}

interface NotesGardees {
  notes: CorrectionNote[]
  empreinte: string | null
  langue: string
  personne: string | null
}

const stockage = { lire: lireCle, poser: poserCle }
const LONGUEUR_MIN_TEXTE = 2

export function useGnDocuments() {
  const api = useApi().guideNegoDocuments
  const { rotation } = useApi()
  const copies = useGnCopies()
  const connexion = useGnConnexion()
  const session = useGnSession()
  const acces = useGnAcces()
  const { locale } = useI18n()

  const personne = () => session.compte.value.id ?? null
  // Ce téléphone sait la personne connectée et son accès ouvert : un réservé fermé
  // dit une lecture faite sans jeton, pas un accès perdu.
  const sembleAnonyme = (lu: Awaited<ReturnType<typeof api.relireLaBibliotheque>>) =>
    !estInchange(lu) && session.connectee.value && acces.ouvert.value && lu.valeur.documents.some((d) => d.restricted && !d.accessible)

  const lecture = useGnLecture<BibliothequeGardee>('documents', async (garde) => {
    // Sans la session ni l'accès connus, une lecture sans jeton passerait pour vraie.
    await Promise.all([session.assurer(), acces.assurer()])
    const langue = String(locale.value)
    const qui = personne()
    const connue = garde?.langue === langue && garde.personne === qui ? garde : null
    const lu = await lireEnPersonne(() => api.relireLaBibliotheque(connue?.empreinte ?? null), sembleAnonyme, rotation)
    let suivante: BibliothequeGardee
    if (!estInchange(lu)) suivante = { bibliotheque: lu.valeur, empreinte: lu.empreinte, langue, personne: qui }
    else if (connue) suivante = { ...connue, empreinte: lu.empreinte }
    else throw new Error('304 sans liste gardée')
    void copies.rapprocher(suivante.bibliotheque)
    return suivante
  })

  const lectureDesNotes = useGnLecture<NotesGardees>('corrections', async (garde) => {
    await Promise.all([session.assurer(), acces.assurer()])
    const langue = String(locale.value)
    const qui = personne()
    const connue = garde?.langue === langue && garde.personne === qui ? garde : null
    const lu = await api.notesDeCorrection(connue?.empreinte ?? null)
    if (!estInchange(lu)) return { notes: lu.valeur.notes, empreinte: lu.empreinte, langue, personne: qui }
    if (connue) return { ...connue, empreinte: lu.empreinte }
    throw new Error('304 sans notes gardées')
  })

  // Après la liste : sa lecture a fait tourner le jeton, et les notes d'un réservé ne vont qu'à qui a l'accès.
  async function rafraichir(): Promise<void> {
    await lecture.rafraichir()
    await lectureDesNotes.rafraichir()
  }

  watch(locale, () => void rafraichir())

  const bibliotheque = computed<DocumentLibrary | null>(() => {
    const garde = lecture.etat.value.valeur
    if (!garde) return null
    return garde.personne && garde.personne !== personne() ? pourUneAutrePersonne(garde.bibliotheque) : garde.bibliotheque
  })
  const documents = computed<LibraryDocument[]>(() => bibliotheque.value?.documents ?? [])
  const telecharges = computed(() => new Set(copies.copies.value.map((c) => c.id)))

  // Avance à chaque fiche ouverte : la marque se relit sur le téléphone.
  const vus = ref(0)
  const nouveaux = computed<Set<string>>(() => {
    void vus.value
    const maintenant = new Date()
    return new Set(documents.value.filter((d) => estNouveau(stockage, d, maintenant)).map((d) => d.id))
  })

  function marquerCommeVu(id: string): void {
    marquerVu(stockage, id, new Date().toISOString())
    vus.value += 1
  }

  const documentDe = (id: string): LibraryDocument | null => documents.value.find((d) => d.id === id) ?? null

  // Gardées pour une autre personne, les notes d'un réservé ne se lisent plus.
  const notes = computed<CorrectionNote[]>(() => {
    const garde = lectureDesNotes.etat.value.valeur
    if (!garde) return []
    if (!garde.personne || garde.personne === personne()) return garde.notes
    return garde.notes.filter((n) => documentDe(n.document_id)?.restricted === false)
  })
  const notesDe = (id: string): CorrectionNote[] => notes.value.filter((n) => n.document_id === id)

  const dansLeTexte = ref<Map<string, PageHit | null> | null>(null)
  let derniere = 0

  /** Rend nul hors connexion ou pour une recherche trop courte : on cherche alors dans la liste seule. */
  async function chercherDansLeTexte(q: string): Promise<void> {
    const demande = ++derniere
    const cherche = q.trim()
    if (cherche.length < LONGUEUR_MIN_TEXTE || !connexion.etat.value.enLigne) {
      dansLeTexte.value = null
      return
    }
    const lu = await api.rechercherDansLeTexte(cherche).catch(() => null)
    if (demande !== derniere) return
    dansLeTexte.value = lu ? new Map(lu.hits.map((h) => [h.document_id, h.pages[0] ?? null])) : null
  }

  return {
    etat: lecture.etat,
    rafraichir,
    bibliotheque,
    documents,
    telecharges,
    nouveaux,
    marquerCommeVu,
    documentDe,
    /** Les notes vivantes d'un document, lues avec la liste et gardées comme elle. */
    notesDe,
    /** Par document trouvé, sa première page — nulle pour un réservé sans accès. */
    dansLeTexte: readonly(dansLeTexte),
    chercherDansLeTexte,
  }
}

/** L'adresse de la bibliothèque telle qu'on l'a quittée, filtres et recherche compris. */
export function useGnRetourALaBibliotheque() {
  return useState<string>('gn-documents-retour', () => '/guide-nego/ressources/documents')
}
