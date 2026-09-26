/**
 * S'inscrire aux réunions de la Francophonie, même sans réseau (FR-008 à FR-011, R5, R6).
 *
 * Le patron de « Mon agenda » : une clé de file par réunion (`inscription-reunion:<id>`),
 * la dernière intention gagne, la garde `mes-inscriptions-reunions` est réécrite
 * aussitôt. **Un `client_ref` neuf à chaque « M'inscrire »** : rejoué, l'envoi rend
 * l'état courant ; après une désinscription, il réinscrit. Un `409` (complet, clos,
 * annulée) abandonne l'intention, et le message de l'API se dit par l'avis de file.
 *
 * La garde porte les liens de visio : ils en sortent dès l'intention de désinscription,
 * à la déconnexion et quand l'accès est perdu (`effacerLesLiensVisio`).
 */
import type { Ref } from 'vue'
import type { MeetingRegistration, MyMeetingRegistrations } from '~/types/negotiation-meetings'
import type { EtatLecture } from './useGnLecture'
import { estInchange } from '~/composables/api/etiquete'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { ecrireGarde, magasinDesEcritures, supprimerGarde } from '~/utils/guide-nego/garde'
import {
  appliquerIntentionInscription,
  avecLaFile,
  boutonDInscription,
  CLE_LECTURE_MES_INSCRIPTIONS_REUNIONS,
  etatDeLaReunion,
  inscriptionDe,
  lienVisio,
  PREFIXE_FILE_INSCRIPTION_REUNION,
  sansLesLiens,
  type IntentionInscription,
} from '~/utils/guide-nego/reunions'
import { referenceClient } from '~/utils/guide-nego/signalements'

interface InscriptionsLues {
  personne: string
  slug: string
  inscriptions: MyMeetingRegistrations
  empreinte: string | null
}

export interface IssueDeLInscription {
  /** `en-attente` : pas de réseau, le geste partira à son retour. */
  issue: 'inscrite' | 'liste-attente' | 'desinscrite' | 'en-attente' | 'refusee'
  position: number | null
  /** Le message de l'API, tel quel, quand elle a refusé. */
  message: string | null
}

const VIDE: MyMeetingRegistrations = { registrations: [], video: [] }

// Client seulement : l'effacement vient d'une lecture d'accès ou de compte, hors de tout écran.
let etatDesInscriptions: Ref<EtatLecture<InscriptionsLues>> | null = null
// Avance à chaque effacement : une lecture partie avant ne rapporte pas ses liens.
let generationDesLiens = 0

/** Déconnexion, session finie, accès perdu : aucun lien de visio ne reste. Ne lève jamais. */
export async function effacerLesLiensVisio(): Promise<void> {
  if (!import.meta.client) return
  generationDesLiens += 1
  const etat = etatDesInscriptions
  const valeur = etat?.value.valeur
  if (etat && valeur) etat.value = { ...etat.value, valeur: { ...valeur, inscriptions: sansLesLiens(valeur.inscriptions), empreinte: null } }
  await supprimerGarde(CLE_LECTURE_MES_INSCRIPTIONS_REUNIONS).catch(() => undefined)
}

export function useGnInscriptionsReunions() {
  const api = useApi().negotiationMeetings
  const session = useGnSession()
  const edition = useGnEdition()
  const reunions = useGnReunions()
  const file = useGnFile()
  // Par personne : un geste de A encore dans la file ne s'affiche pas chez B.
  const enFileParPersonne = useState<Record<string, Record<string, IntentionInscription>>>('gn-inscriptions-reunions-en-file', () => ({}))
  const enFile = computed(() => enFileParPersonne.value[session.compte.value.id ?? ''] ?? {})

  const lecture = useGnLecture<InscriptionsLues>(CLE_LECTURE_MES_INSCRIPTIONS_REUNIONS, async (garde) => {
    const personne = session.compte.value.id
    if (!personne) throw new Error('sans compte')
    const lue = await edition.lue()
    if (!lue) throw new Error('aucune édition')
    const connue = garde?.personne === personne && garde.slug === lue.slug ? garde : null
    const generation = generationDesLiens
    let valeur: InscriptionsLues
    try {
      const lu = await api.mesInscriptions(lue.slug, connue?.empreinte ?? null)
      if (!estInchange(lu)) valeur = { personne, slug: lue.slug, inscriptions: lu.valeur, empreinte: lu.empreinte }
      else if (connue) valeur = { ...connue, empreinte: lu.empreinte }
      else throw new Error('304 sans inscriptions gardées')
    } catch (erreur) {
      // L'API a parlé — sans accès, rien à montrer : ce n'est pas une panne de réseau.
      if (normalizeApiError(erreur) instanceof ApiRequestError) return { personne, slug: lue.slug, inscriptions: VIDE, empreinte: null }
      throw erreur
    }
    if (generation !== generationDesLiens) return { ...valeur, inscriptions: sansLesLiens(valeur.inscriptions), empreinte: null }
    // Une désinscription encore en file : son lien ne revient pas dans la garde.
    const quittees = Object.entries(enFile.value).filter(([, v]) => !v.inscrire).map(([id]) => id)
    return {
      ...valeur,
      inscriptions: { ...valeur.inscriptions, video: valeur.inscriptions.video.filter((v) => !quittees.includes(v.meeting_id)) },
    }
  })
  if (import.meta.client) etatDesInscriptions = lecture.etat

  async function relireLaFile(): Promise<void> {
    const personne = session.compte.value.id
    if (!personne) return
    const intentions = (await magasinDesEcritures.lire().catch(() => [])).filter(
      (i) => i.personne === personne && i.cle.startsWith(PREFIXE_FILE_INSCRIPTION_REUNION),
    )
    enFileParPersonne.value = {
      ...enFileParPersonne.value,
      [personne]: Object.fromEntries(
        intentions.map((i) => [i.cle.slice(PREFIXE_FILE_INSCRIPTION_REUNION.length), i.corps as IntentionInscription]),
      ),
    }
  }

  const lu = computed<InscriptionsLues | null>(() => {
    const valeur = lecture.etat.value.valeur
    return valeur && valeur.personne === session.compte.value.id ? valeur : null
  })

  const inscriptions = computed<MyMeetingRegistrations>(() =>
    avecLaFile(lu.value?.inscriptions ?? VIDE, enFile.value, reunions.reunions.value, new Date()),
  )

  file.inscrireFamille(
    PREFIXE_FILE_INSCRIPTION_REUNION,
    (intention) => {
      const id = intention.cle.slice(PREFIXE_FILE_INSCRIPTION_REUNION.length)
      const voulu = intention.corps as IntentionInscription
      return voulu.inscrire && voulu.client_ref ? api.inscrire(id, voulu.client_ref) : api.desinscrire(id)
    },
    async () => {
      await lecture.relire()
      await relireLaFile()
      // Le compte des inscrites a bougé : « Complet » se relit.
      void reunions.rafraichir()
    },
  )

  // Sans compte, la lecture échouerait, et l'échec passerait pour une panne de réseau.
  function assurer(): void {
    if (!session.connectee.value) return
    void relireLaFile()
    void lecture.rafraichir()
  }

  /** Ce que la personne voit tout de suite, et ce qu'elle retrouve à la réouverture sans réseau. */
  async function reecrireLaGarde(personne: string, slug: string, appliquer: (m: MyMeetingRegistrations) => MyMeetingRegistrations) {
    const etat = lecture.etat.value
    const avant = etat.valeur?.personne === personne && etat.valeur.slug === slug ? etat.valeur.inscriptions : VIDE
    // Sans empreinte : l'état écrit ici n'est pas celui du serveur, un `304` ne doit pas le confirmer.
    const valeur: InscriptionsLues = { personne, slug, inscriptions: appliquer(avant), empreinte: null }
    lecture.etat.value = { ...etat, valeur, pret: true }
    await ecrireGarde({ cle: CLE_LECTURE_MES_INSCRIPTIONS_REUNIONS, valeur, lu_a: etat.luA ?? new Date().toISOString(), empreinte: null })
  }

  function issueApresEnvoi(reunionId: string): IssueDeLInscription {
    const i = inscriptionDe(lu.value?.inscriptions ?? VIDE, reunionId)
    if (i?.status === 'registered') return { issue: 'inscrite', position: null, message: null }
    if (i?.status === 'waitlisted') return { issue: 'liste-attente', position: i.waitlist_position, message: null }
    return { issue: 'desinscrite', position: null, message: null }
  }

  async function vouloir(reunionId: string, voulu: IntentionInscription): Promise<IssueDeLInscription> {
    const personne = session.compte.value.id
    const slug = (await edition.gardee())?.slug ?? null
    if (!personne || !slug) return { issue: 'refusee', position: null, message: null }

    enFileParPersonne.value = { ...enFileParPersonne.value, [personne]: { ...enFile.value, [reunionId]: voulu } }
    await reecrireLaGarde(personne, slug, (m) =>
      appliquerIntentionInscription(m, reunionId, reunions.reunion(reunionId), voulu, new Date()),
    )
    const cle = `${PREFIXE_FILE_INSCRIPTION_REUNION}${reunionId}`
    await file.poser(cle, voulu, null)
    const suite = (await file.partir()).find((s) => s.cle === cle)
    await relireLaFile()
    if (suite?.sort === 'envoyee') return issueApresEnvoi(reunionId)
    if (suite?.sort === 'refusee' || suite?.sort === 'perimee') return { issue: 'refusee', position: null, message: suite.message }
    return { issue: 'en-attente', position: null, message: null }
  }

  const inscription = (reunionId: string): MeetingRegistration | null => inscriptionDe(inscriptions.value, reunionId)

  return {
    inscriptions,
    inscription,
    /** Le lien de visio, pour l'inscrite (ou toute admise sans inscription requise) ; sinon nul. */
    lienVisio: (reunionId: string) => lienVisio(inscriptions.value, reunionId),
    /** Un geste attend encore le réseau pour cette réunion. */
    aEnvoyer: (reunionId: string) => reunionId in enFile.value,
    etat: (reunionId: string, maintenant = new Date()) => {
      const r = reunions.reunion(reunionId)
      return r ? etatDeLaReunion(r, inscription(reunionId), maintenant) : null
    },
    bouton: (reunionId: string, maintenant = new Date()) => {
      const r = reunions.reunion(reunionId)
      return r ? boutonDInscription(r, inscription(reunionId), maintenant) : null
    },
    pret: computed(() => lecture.etat.value.pret),
    /** Lu au moins une fois pour cette personne : sinon, une liste vide ne veut pas dire « rien ». */
    connu: computed(() => lu.value !== null),
    luA: computed(() => lecture.etat.value.luA),
    /** Le refus d'un geste parti de la file, avec le message de l'API. */
    refus: computed(() => {
      const avis = file.avis.value
      return avis?.cle.startsWith(PREFIXE_FILE_INSCRIPTION_REUNION) ? avis : null
    }),
    assurer,
    rafraichir: lecture.rafraichir,
    /** « M'inscrire » ou « Rejoindre la liste d'attente » : une référence neuve à chaque geste. */
    inscrire: (reunionId: string) => vouloir(reunionId, { inscrire: true, client_ref: referenceClient() }),
    desinscrire: (reunionId: string) => vouloir(reunionId, { inscrire: false, client_ref: null }),
  }
}
