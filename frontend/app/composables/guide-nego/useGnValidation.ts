/**
 * La file de validation des signalements — **en ligne seulement** (FR-013, R4).
 *
 * Une décision prise sur une vue périmée de la source contredirait ADR-010 : rien
 * ici ne passe par la file d'écritures ni ne se garde. Sans réseau, le geste est
 * refusé et le dit.
 */
import type { ReportQueue, ReportQueueItem, RejectPayload } from '~/types/negotiation-reports'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { apresDecision } from '~/utils/guide-nego/signalements'

interface EtatValidation {
  file: ReportQueue | null
  luA: string | null
  enCours: boolean
  /** Le message de l'API tel quel ; nul quand elle s'est tue. */
  erreur: string | null
  /** La lecture a échoué sans réponse de l'API. */
  sansReseau: boolean
}

export type IssueDeDecision =
  | { ok: true; element: ReportQueueItem }
  | { ok: false; raison: 'hors-connexion' | 'refus' | 'panne'; code: string | null; message: string | null }

function echec(erreur: unknown): IssueDeDecision {
  const e = normalizeApiError(erreur)
  return e instanceof ApiRequestError
    ? { ok: false, raison: 'refus', code: e.code, message: e.message }
    : { ok: false, raison: 'panne', code: null, message: null }
}

export function useGnValidation() {
  const api = useApi().negotiationReports
  const acces = useGnAcces()
  const edition = useGnEdition()
  const connexion = useGnConnexion()
  const etat = useState<EtatValidation>('gn-validation', () => ({
    file: null,
    luA: null,
    enCours: false,
    erreur: null,
    sansReseau: false,
  }))

  async function charger(): Promise<void> {
    etat.value = { ...etat.value, enCours: true }
    try {
      const slug = (await edition.lue())?.slug
      if (!slug) throw new Error('aucune édition')
      const file = await api.fileDeValidation(slug)
      etat.value = { file, luA: new Date().toISOString(), enCours: false, erreur: null, sansReseau: false }
    } catch (erreur) {
      const e = normalizeApiError(erreur)
      const parle = e instanceof ApiRequestError
      etat.value = { ...etat.value, enCours: false, erreur: parle ? e.message : null, sansReseau: !parle }
    }
  }

  async function decider(geste: () => Promise<ReportQueueItem>): Promise<IssueDeDecision> {
    if (!connexion.etat.value.enLigne) return { ok: false, raison: 'hors-connexion', code: null, message: null }
    try {
      const element = await geste()
      if (etat.value.file) etat.value = { ...etat.value, file: apresDecision(etat.value.file, element) }
      return { ok: true, element }
    } catch (erreur) {
      return echec(erreur)
    }
  }

  return {
    etat: readonly(etat),
    peutValider: computed(() => acces.acces.value.can_validate_reports ?? false),
    aExaminer: computed(() => acces.acces.value.reports_to_review ?? null),
    charger,
    valider: (id: string) => decider(() => api.valider(id)),
    /** L'écran l'offre six secondes ; le serveur accepte tant que rien n'est publié. */
    annuler: (id: string) => decider(() => api.annuler(id)),
    refuser: (id: string, corps: RejectPayload) => decider(() => api.refuser(id, corps)),
    retirer: (id: string) => decider(() => api.retirer(id)),
  }
}
