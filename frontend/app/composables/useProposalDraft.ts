import type { ProposalDraft, SaveDraftPayload } from '~/types/proposal-form'
import type { CallId, EventId, IsoDateTime, ProposalId } from '~/types/shared'

/**
 * L'ENREGISTREMENT AUTOMATIQUE du dossier, et rien d'autre.
 *
 * POURQUOI IL EXISTE. Ce formulaire compte sept étapes et se remplit en
 * plusieurs fois, souvent par quelqu'un qui n'est pas un utilisateur expert :
 * demander « Enregistrer » à chaque étape, c'est perdre le dossier de qui ferme
 * l'onglet. La v1 n'enregistrait qu'à la soumission.
 *
 * QUATRE RÈGLES, ET CHACUNE ÉVITE UN DÉFAUT OBSERVABLE :
 *
 *  1. AUCUN ENREGISTREMENT AVANT LA PREMIÈRE MODIFICATION. Sans cela, ouvrir la
 *     page créerait une ligne vide en base — et le compteur du plafond de
 *     l'organisation avancerait pour une visite. La ligne naît à la première
 *     frappe, pas au premier affichage.
 *  2. UNE SEULE ÉCRITURE EN VOL. Une deuxième frappe pendant l'enregistrement ne
 *     déclenche pas un second appel : elle marque le brouillon à reprendre dès
 *     que le premier est revenu. Deux écritures concurrentes sur la même ligne,
 *     c'est la plus lente qui gagne — donc la plus ancienne.
 *  3. RIEN NE PART SANS ORGANISATION PORTEUSE. La colonne est NOT NULL et l'API
 *     refuse : une personne membre de deux organisations ouvre le formulaire
 *     sans porteur, et sa première frappe échouerait en 422. On attend l'étape 1
 *     plutôt que d'afficher un échec qui ne dit pas quoi corriger.
 *  4. L'HORODATAGE VIENT DU SERVEUR. L'écran n'affiche jamais sa propre horloge
 *     comme heure d'enregistrement : elles divergent, et c'est celle du serveur
 *     qui fait foi. La réponse porte `saved_at` ; on l'affiche telle quelle.
 *
 * L'ÉTAT EST RENDU SANS AMBIGUÏTÉ (`UiAutosave`) : « Enregistrement… »,
 * « Enregistré à 14:32 », « Modifications non enregistrées », « Échec de
 * l'enregistrement, réessayer ». Un point vert muet ne dit pas si le travail est
 * en sécurité.
 */

export type DraftSaveState = 'untouched' | 'dirty' | 'saving' | 'saved' | 'error'

export interface UseProposalDraftOptions {
  /** Le brouillon, tenu par l'écran ; ce composable ne fait que l'observer. */
  draft: Ref<ProposalDraft>
  callId: Ref<CallId | null>
  eventId: Ref<EventId | null>
  personId: Ref<string | null>
  /** Silence à observer après la dernière frappe, en millisecondes. */
  debounceMs?: number
}

export function useProposalDraft(options: UseProposalDraftOptions) {
  const api = useApi()
  const debounceMs = options.debounceMs ?? 1_500

  const proposalId = ref<ProposalId | null>(null)
  const referenceCode = ref<string | null>(null)
  const savedAt = ref<IsoDateTime | null>(null)
  const state = ref<DraftSaveState>('untouched')
  const error = ref<Error | null>(null)

  /** Une modification est arrivée pendant qu'on enregistrait. */
  let pending = false
  let timer: ReturnType<typeof setTimeout> | null = null
  /** L'observation est-elle armée ? Elle ne l'est qu'après le premier chargement. */
  let armed = false

  /** Reprise d'un brouillon existant : on adopte son identité, sans écrire. */
  function adopt(existing: {
    proposal_id: ProposalId
    reference_code: string
    saved_at: IsoDateTime
  }): void {
    proposalId.value = existing.proposal_id
    referenceCode.value = existing.reference_code
    savedAt.value = existing.saved_at
    state.value = 'saved'
  }

  /** Arme l'observation. À appeler une fois le brouillon initial posé. */
  function arm(): void {
    armed = true
  }

  async function save(): Promise<void> {
    const person = options.personId.value
    const callId = options.callId.value
    const eventId = options.eventId.value
    if (!person || !callId || !eventId) return

    // SANS PORTEUR, PAS DE LIGNE. `proposals.organization_id` est NOT NULL et
    // l'API refuse la création en 422 : partir quand même afficherait « Échec de
    // l'enregistrement » sans dire quoi corriger. Le brouillon reste « non
    // enregistré » jusqu'au choix de l'étape 1, qui déclenche l'écriture.
    if (!options.draft.value.organization_id) {
      state.value = 'dirty'
      return
    }

    if (state.value === 'saving') {
      pending = true
      return
    }

    state.value = 'saving'
    error.value = null
    const payload: SaveDraftPayload = {
      proposal_id: proposalId.value,
      call_id: callId,
      event_id: eventId,
      draft: options.draft.value,
    }

    try {
      const result = await api.proposals.saveDraft(person, payload)
      proposalId.value = result.proposal_id
      referenceCode.value = result.reference_code
      savedAt.value = result.saved_at
      state.value = 'saved'
    } catch (cause) {
      error.value = cause instanceof Error ? cause : new Error(String(cause))
      state.value = 'error'
    } finally {
      if (pending) {
        pending = false
        // Une frappe est arrivée pendant l'écriture : on repart, une seule fois.
        void save()
      }
    }
  }

  function scheduleSave(): void {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      timer = null
      void save()
    }, debounceMs)
  }

  watch(
    () => options.draft.value,
    () => {
      if (!armed) return
      state.value = 'dirty'
      scheduleSave()
    },
    { deep: true },
  )

  onBeforeUnmount(() => {
    if (timer) clearTimeout(timer)
  })

  return {
    proposalId,
    referenceCode,
    savedAt,
    state,
    error,
    adopt,
    arm,
    /**
     * Enregistrement immédiat — changement d'étape, envoi, départ de la page.
     *
     * Rien à faire quand tout est déjà écrit. La seule exception est le dossier
     * jamais enregistré : l'envoi a besoin d'une ligne, donc d'un identifiant,
     * et un dossier rempli sans qu'aucune modification n'ait été observée reste
     * possible — reprise d'un brouillon repris tel quel, par exemple.
     */
    saveNow: async () => {
      if (timer) {
        clearTimeout(timer)
        timer = null
      }
      if (state.value === 'saved') return
      if (state.value === 'untouched' && proposalId.value !== null) return
      await save()
    },
  }
}
