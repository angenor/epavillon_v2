/**
 * S'inscrire aux activités du Pavillon, même sans réseau (FR-006, FR-007, R3).
 *
 * Le patron des réunions : une clé de file par séance (`inscription-pavillon:<id>`)
 * portant l'état voulu, la dernière intention gagne, la garde `mes-inscriptions-pavillon`
 * réécrite aussitôt. **L'annulation cherche son identifiant au départ**, dans « mes
 * inscriptions » relues : une inscription qui n'est jamais partie n'a rien à annuler, et
 * rien ne part. Complet, clos, pas encore ouvert sortent en 200 : ils deviennent un refus
 * dit en mots clairs.
 */
import type { Registration, RegistrationResult } from '~/types/programme/registration'
import type { ApiErrorCode } from '~/types/api-error'
import { estInchange } from '~/composables/api/etiquete'
import { formatDateTime, timeZoneCityLabel } from '~/utils/datetime'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { ecrireGarde, magasinDesEcritures } from '~/utils/guide-nego/garde'
import {
  avecLaFilePavillon,
  boutonDInscriptionPavillon,
  CLE_LECTURE_MES_INSCRIPTIONS_PAVILLON,
  appliquerIntentionPavillon,
  inscriptionAAnnuler,
  inscriptionDeLaSeance,
  marqueDeLActivite,
  PREFIXE_FILE_INSCRIPTION_PAVILLON,
  type IntentionPavillon,
} from '~/utils/guide-nego/pavillon'

interface InscriptionsLues {
  personne: string
  inscriptions: Registration[]
  empreinte: string | null
}

export interface IssueInscriptionPavillon {
  /** `en-attente` : pas de réseau, le geste partira à son retour. */
  issue: 'inscrite' | 'liste-attente' | 'annulee' | 'en-attente' | 'refusee'
  position: number | null
  /** Le message de l'API tel quel, ou la phrase de l'application pour une issue en 200. */
  message: string | null
}

const refus = (code: ApiErrorCode, status: number, message: string) => new ApiRequestError({ code, message }, status)

const introuvable = (erreur: unknown) => {
  const e = normalizeApiError(erreur)
  return e instanceof ApiRequestError && e.status === 404
}

export function useGnInscriptionsPavillon() {
  const api = useApi().pavillon
  const { t, locale } = useI18n()
  const session = useGnSession()
  const edition = useGnEdition()
  const pavillon = useGnPavillon()
  const file = useGnFile()
  // Par personne : un geste de A encore dans la file ne s'affiche pas chez B.
  const enFileParPersonne = useState<Record<string, Record<string, IntentionPavillon>>>('gn-inscriptions-pavillon-en-file', () => ({}))
  const enFile = computed(() => enFileParPersonne.value[session.compte.value.id ?? ''] ?? {})

  const lecture = useGnLecture<InscriptionsLues>(CLE_LECTURE_MES_INSCRIPTIONS_PAVILLON, async (garde) => {
    const personne = session.compte.value.id
    if (!personne) throw new Error('sans compte')
    const connue = garde?.personne === personne ? garde : null
    const lu = await api.mesInscriptions(connue?.empreinte ?? null)
    if (!estInchange(lu)) return { personne, inscriptions: lu.valeur, empreinte: lu.empreinte }
    if (connue) return { ...connue, empreinte: lu.empreinte }
    throw new Error('304 sans inscriptions gardées')
  })

  async function relireLaFile(): Promise<void> {
    const personne = session.compte.value.id
    if (!personne) return
    const intentions = (await magasinDesEcritures.lire().catch(() => [])).filter(
      (i) => i.personne === personne && i.cle.startsWith(PREFIXE_FILE_INSCRIPTION_PAVILLON),
    )
    enFileParPersonne.value = {
      ...enFileParPersonne.value,
      [personne]: Object.fromEntries(
        intentions.map((i) => [i.cle.slice(PREFIXE_FILE_INSCRIPTION_PAVILLON.length), i.corps as IntentionPavillon]),
      ),
    }
  }

  const lu = computed<InscriptionsLues | null>(() => {
    const valeur = lecture.etat.value.valeur
    return valeur && valeur.personne === session.compte.value.id ? valeur : null
  })

  const inscriptions = computed<Registration[]>(() =>
    avecLaFilePavillon(lu.value?.inscriptions ?? [], enFile.value, pavillon.activites.value, session.compte.value.id ?? '', new Date()),
  )

  const ville = (fuseau: string) => edition.edition.value?.city ?? timeZoneCityLabel(fuseau)
  const quand = (iso: string, fuseau: string) => formatDateTime(iso, { locale: locale.value, timeZone: fuseau })

  /** Complet, clos, pas encore ouvert : des issues en 200, dites comme un refus. */
  function refusDe(issue: RegistrationResult, fuseau: string): ApiRequestError | null {
    if (issue.status === 'full') return refus('CONFLICT', 409, t('gn-formulaire-inscription.issues.full'))
    if (issue.status === 'closed') {
      return refus('CONFLICT', 409, t('gn-formulaire-inscription.issues.closed', { date: quand(issue.closed_at, fuseau), ville: ville(fuseau) }))
    }
    if (issue.status === 'not_open_yet') {
      return refus('CONFLICT', 409, t('gn-formulaire-inscription.issues.not_open_yet', { date: quand(issue.opens_at, fuseau), ville: ville(fuseau) }))
    }
    return null
  }

  async function envoyer(sessionId: string, voulu: IntentionPavillon): Promise<void> {
    if (voulu.etat === 'inscrite') {
      const issue = await api.inscrire(sessionId, {
        answers: voulu.reponses,
        locale: locale.value,
        sensitive_data_consent: voulu.consentement,
      })
      const fuseau = pavillon.activite(sessionId)?.timezone ?? pavillon.fuseau.value ?? 'UTC'
      const refusee = refusDe(issue, fuseau)
      if (refusee) throw refusee
      return
    }
    // Relue au départ : la garde peut porter une inscription locale que le serveur ignore.
    const lues = await api.mesInscriptions(null)
    const cible = estInchange(lues) ? null : inscriptionAAnnuler(lues.valeur, sessionId)
    if (!cible) return
    try {
      await api.annuler(cible.id)
    } catch (erreur) {
      if (!introuvable(erreur)) throw erreur
    }
  }

  file.inscrireFamille(
    PREFIXE_FILE_INSCRIPTION_PAVILLON,
    (intention) => envoyer(intention.cle.slice(PREFIXE_FILE_INSCRIPTION_PAVILLON.length), intention.corps as IntentionPavillon),
    async () => {
      await lecture.relire()
      await relireLaFile()
      // Le compte des inscrits a bougé : « Complet » se relit.
      void pavillon.rafraichir()
    },
  )

  // Sans compte, la lecture échouerait, et l'échec passerait pour une panne de réseau.
  function assurer(): void {
    if (!session.connectee.value) return
    void relireLaFile()
    void lecture.rafraichir()
  }

  /** Ce que la personne voit tout de suite, et ce qu'elle retrouve à la réouverture sans réseau. */
  async function reecrireLaGarde(personne: string, appliquer: (i: Registration[]) => Registration[]) {
    const etat = lecture.etat.value
    const avant = etat.valeur?.personne === personne ? etat.valeur.inscriptions : []
    // Sans empreinte : l'état écrit ici n'est pas celui du serveur, un `304` ne doit pas le confirmer.
    const valeur: InscriptionsLues = { personne, inscriptions: appliquer(avant), empreinte: null }
    lecture.etat.value = { ...etat, valeur, pret: true }
    await ecrireGarde({ cle: CLE_LECTURE_MES_INSCRIPTIONS_PAVILLON, valeur, lu_a: etat.luA ?? new Date().toISOString(), empreinte: null })
  }

  function issueApresEnvoi(sessionId: string): IssueInscriptionPavillon {
    const i = inscriptionDeLaSeance(lu.value?.inscriptions ?? [], sessionId)
    if (i?.status === 'registered') return { issue: 'inscrite', position: null, message: null }
    if (i?.status === 'waitlisted') return { issue: 'liste-attente', position: i.waitlist_position, message: null }
    return { issue: 'annulee', position: null, message: null }
  }

  async function vouloir(sessionId: string, voulu: IntentionPavillon): Promise<IssueInscriptionPavillon> {
    const personne = session.compte.value.id
    if (!personne) return { issue: 'refusee', position: null, message: null }

    enFileParPersonne.value = { ...enFileParPersonne.value, [personne]: { ...enFile.value, [sessionId]: voulu } }
    await reecrireLaGarde(personne, (i) =>
      appliquerIntentionPavillon(i, sessionId, pavillon.activite(sessionId), voulu, personne, new Date()),
    )
    const cle = `${PREFIXE_FILE_INSCRIPTION_PAVILLON}${sessionId}`
    await file.poser(cle, voulu, null)
    const suite = (await file.partir()).find((s) => s.cle === cle)
    await relireLaFile()
    if (suite?.sort === 'envoyee') return issueApresEnvoi(sessionId)
    if (suite?.sort === 'refusee' || suite?.sort === 'perimee') return { issue: 'refusee', position: null, message: suite.message }
    return { issue: 'en-attente', position: null, message: null }
  }

  const inscription = (sessionId: string): Registration | null => inscriptionDeLaSeance(inscriptions.value, sessionId)

  return {
    inscriptions,
    inscription,
    /** Un geste attend encore le réseau pour cette activité. */
    aEnvoyer: (sessionId: string) => sessionId in enFile.value,
    marque: (sessionId: string, maintenant = new Date()) => {
      const a = pavillon.activite(sessionId)
      return a ? marqueDeLActivite(a, inscription(sessionId), maintenant) : null
    },
    bouton: (sessionId: string, maintenant = new Date()) => {
      const a = pavillon.activite(sessionId)
      return a ? boutonDInscriptionPavillon(a, inscription(sessionId), maintenant) : null
    },
    pret: computed(() => lecture.etat.value.pret),
    /** Lu au moins une fois pour cette personne : sinon, une liste vide ne veut pas dire « rien ». */
    connu: computed(() => lu.value !== null),
    luA: computed(() => lecture.etat.value.luA),
    /** Le refus d'un geste parti de la file, avec son message. */
    refus: computed(() => {
      const avis = file.avis.value
      return avis?.cle.startsWith(PREFIXE_FILE_INSCRIPTION_PAVILLON) ? avis : null
    }),
    assurer,
    rafraichir: lecture.rafraichir,
    /** « M'inscrire » ou « Rejoindre la liste d'attente », avec les réponses du formulaire. */
    inscrire: (sessionId: string, reponses: Record<string, unknown>, consentement = false) =>
      vouloir(sessionId, { etat: 'inscrite', reponses, consentement }),
    annuler: (sessionId: string) => vouloir(sessionId, { etat: 'annulee' }),
  }
}
