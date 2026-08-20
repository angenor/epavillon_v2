/**
 * PROPOSITIONS (A4 · A7) — sa part de `useApi()`.
 *
 * Même motif qu'`api/admin-events.ts`, `api/planner.ts`, `api/proposal-review.ts`
 * et `api/admin-organizations.ts` : la règle du projet est inchangée — aucune
 * page n'importe un mock, aucune page n'appelle `$fetch`. Les écrans appellent
 * `api.proposals.…`. Seule la place du CODE change, pour tenir `useApi.ts` sous
 * le garde-fou de mille lignes de `CLAUDE.md`, qu'il avait franchi le 20/08.
 *
 * ── POURQUOI CE BLOC-LÀ ET PAS UN AUTRE ─────────────────────────────────────
 *
 * C'était le plus gros resté en place — deux cents lignes, soit le cinquième du
 * fichier — et le seul à porter deux écrans : le FORMULAIRE DE DÉPÔT (A4), côté
 * organisation, et la LISTE DU BACK-OFFICE (A7), côté IFDD. Ils partagent la
 * même entité et rien d'autre : l'un écrit un brouillon, l'autre trie des
 * dossiers et affecte des évaluateurs.
 *
 * ── LE PÉRIMÈTRE, ENCORE ────────────────────────────────────────────────────
 *
 * Les lectures du back-office prennent `AdministeredEvents` et REFUSENT une
 * édition hors périmètre par `assertEventInScope` — pas une liste vide, un
 * refus. Une liste vide se lit comme « rien à traiter » ; c'est exactement la
 * confusion que la règle n° 8 interdit.
 *
 * Les appels du formulaire de dépôt, eux, n'en prennent pas : une organisation
 * dépose sur l'appel ouvert, elle n'administre rien.
 */

import type { AdministeredEvents } from '~/types/identity'
import type {
  SaveDraftPayload,
  SaveDraftResult,
  SubmitProposalPayload,
  SubmitProposalResult,
} from '~/types/proposal-form'
import type {
  AssignReviewerPayload,
  BulkResult,
  ChangeStatusPayload,
  ProposalFacet,
  ProposalListScreen,
} from '~/types/admin-proposals'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export interface ProposalsApiContext extends ApiTransport {
  /** Refuse une édition hors périmètre plutôt que de rendre une liste vide. */
  assertEventInScope: (eventId: Uuid, scope: AdministeredEvents) => void
}

export function createProposalsApi({ call, send, assertEventInScope }: ProposalsApiContext) {
  return {
  /** Liste du back-office (A7) : la vue prête à l'emploi, filtrée par périmètre. */
  dashboard: (eventId: Uuid, scope: AdministeredEvents) => {
    assertEventInScope(eventId, scope)
    return call(
      '/proposals/dashboard',
      (m) => m.proposalDashboard().filter((row) => row.event_id === eventId),
      { event_id: eventId },
    )
  },

  /**
   * TOUT L'ÉCRAN DE LA LISTE EN UNE RÉPONSE (A7) — les lignes de la vue, les
   * facettes des filtres avec leur décompte, les dossiers que la personne
   * n'a pas encore ouverts, le fuseau de l'édition et l'échéance de l'appel.
   *
   * UNE COMPOSITION, PAS SIX LECTURES. Les facettes se comptent sur le même
   * jeu de lignes que la liste : demandées à part, elles seraient mesurées à
   * un autre instant, et le « Retenu (17) » du filtre finirait par ne plus
   * correspondre aux lignes affichées.
   *
   * LE FILTRAGE ET LE TRI RESTENT CÔTÉ ÉCRAN tant que les données sont
   * simulées : quarante lignes tiennent en mémoire, et un tri serveur sur
   * quarante lignes serait un aller-retour pour rien. Au raccordement (B7),
   * ces paramètres deviendront ceux de la requête — c'est pourquoi ils vivent
   * dans l'URL et non dans un état de composant.
   */
  list: (eventId: Uuid, scope: AdministeredEvents, personId: Uuid | null): Promise<ProposalListScreen | null> => {
    assertEventInScope(eventId, scope)
    return call('/proposals/list', (m) => m.proposalListScreen(eventId, personId), {
      event_id: eventId,
    })
  },

  /** Composition du comité de l'appel, avec la charge de chacun. */
  committee: (eventId: Uuid, scope: AdministeredEvents): Promise<ProposalFacet[]> => {
    assertEventInScope(eventId, scope)
    return call('/proposals/committee', (m) => m.committeeOf(eventId), { event_id: eventId })
  },

  /**
   * LA MACHINE À ÉTATS, LUE ET NON RÉÉCRITE —
   * `programme.proposal_transitions_allowed`. L'écran n'affiche que les
   * transitions déclarées, avec leur permission et leur exigence de motif.
   * Ajouter un chemin en base ajoute une action, sans toucher au code.
   */
  transitionRules: () =>
    call('/proposals/transitions', (m) => m.proposalTransitionsAllowed),

  /**
   * ACTION GROUPÉE — confier une sélection à un membre du comité.
   *
   * La réponse dit ce qui A ÉTÉ FAIT et, dossier par dossier, ce qui ne l'a
   * pas été : déjà affecté, déporté, introuvable. Une action de masse qui ne
   * rend qu'un nombre laisse croire à un succès complet.
   */
  assignReviewer: (actorId: Uuid | null, payload: AssignReviewerPayload): Promise<BulkResult> =>
    send('/proposals/assignments', payload, (m) => m.assignReviewer(payload, actorId)),

  /**
   * ACTION GROUPÉE — changer le statut d'une sélection. Les transitions
   * refusées par `proposal_transitions_allowed` sont écartées avec leur
   * motif, comme le ferait le trigger.
   */
  changeStatus: (actorId: Uuid | null, payload: ChangeStatusPayload): Promise<BulkResult> =>
    send('/proposals/status', payload, (m) => m.changeProposalStatus(payload, actorId)),
  byId: (id: Uuid) => call(`/proposals/${id}`, (m) => m.allProposals.find((p) => p.id === id) ?? null),
  /** Dossiers d'une organisation, brouillons compris (A5). */
  forOrganization: (organizationId: Uuid) =>
    call(
      '/proposals',
      (m) => m.allProposals.filter((p) => p.organization_id === organizationId),
      { organization_id: organizationId },
    ),
  organizations: (id: Uuid) =>
    call(`/proposals/${id}/organizations`, (m) => m.proposalOrganizations.filter((o) => o.proposal_id === id)),
  speakers: (id: Uuid) =>
    call(`/proposals/${id}/speakers`, (m) => m.proposalSpeakers.filter((s) => s.proposal_id === id)),
  documents: (id: Uuid) =>
    call(`/proposals/${id}/documents`, (m) => m.proposalDocuments.filter((d) => d.proposal_id === id)),
  /** Fil d'échanges, filtré sur ce que le lecteur a le droit de voir. */
  comments: (id: Uuid, viewerId: Uuid, isCommittee: boolean) =>
    call(`/proposals/${id}/comments`, (m) =>
      m.proposalComments.filter((c) => {
        if (c.proposal_id !== id || c.deleted_at !== null) return false
        if (c.visibility === 'private') return c.author_id === viewerId
        if (c.visibility === 'committee') return isCommittee
        return true
      }),
    ),
  history: (id: Uuid) =>
    call(`/proposals/${id}/transitions`, (m) => m.proposalTransitions.filter((t) => t.proposal_id === id)),
  /**
   * OÙ L'ON DÉPOSE AUJOURD'HUI, et ce que l'organisation a déjà déposé.
   *
   * Le formulaire de soumission (A4) ne choisit pas son édition : il y en a
   * au plus une dont l'appel est ouvert (`ux_calls_one_per_event` et
   * `event.is_call_open()`). Ce contexte porte aussi le décompte du plafond
   * `max_proposals_per_organization`, que le trigger de recevabilité
   * appliquera de toute façon — l'écran doit pouvoir le dire AVANT sept
   * étapes de saisie, pas après.
   */
  formContext: (personId: Uuid, organizationIds: Uuid[]) =>
    call('/proposals/form-context', (m) => m.proposalFormContext(personId, organizationIds), {
      organization_ids: organizationIds.join(','),
    }),

  /** Brouillon en cours de la personne, pour reprendre où elle s'est arrêtée. */
  myDraft: (personId: Uuid) =>
    call('/proposals/draft', (m) => m.draftProposalOf(personId)),

  /**
   * UN DOSSIER EXISTANT, RECOMPOSÉ EN BROUILLON — c'est ce qui permet de le
   * MODIFIER (arbitrage du commanditaire du 17/08 : « tant que l'événement
   * n'est pas terminé, il peut modifier »).
   *
   * La recomposition n'est pas un `SELECT` : le formulaire travaille sur une
   * structure d'écran — français, heures murales, clés de liste — quand la
   * base range la même chose dans cinq tables. Elle appartient donc à l'API
   * (prompt B4), pas à la page : deux écrans qui la referaient chacun de
   * leur côté divergeraient sur le premier champ ajouté.
   */
  forEdit: (proposalId: Uuid) =>
    call(`/proposals/${proposalId}/draft`, (m) => m.editableProposal(proposalId)),

  /**
   * ENREGISTREMENT AUTOMATIQUE. Le premier appel CRÉE la ligne et rend son
   * numéro de dossier — `tg_assign_reference_code` s'exécute à l'insertion,
   * pas au dépôt. Les suivants ne font que dater.
   */
  saveDraft: (personId: Uuid, payload: SaveDraftPayload): Promise<SaveDraftResult> =>
    send(
      payload.proposal_id ? `/proposals/${payload.proposal_id}` : '/proposals',
      payload,
      // Un dossier DÉJÀ EN BASE se met à jour sans changer d'état : corriger
      // n'est pas déposer, et un dossier en évaluation ne repart pas au
      // comité parce qu'on a rectifié une faute de frappe.
      (m) => m.saveExistingProposal(payload) ?? m.saveProposalDraft(personId, payload),
      payload.proposal_id ? 'PUT' : 'POST',
    ),

  /**
   * RENVOI AU COMITÉ d'un dossier corrigé — `changes_requested → submitted`.
   *
   * Distinct du dépôt : la fenêtre de l'appel ne s'y applique pas. Le comité
   * demande ses corrections APRÈS la clôture, et le trigger de recevabilité
   * les refusait toutes jusqu'à sa correction du 17/08 — laissant
   * l'organisation devant un écran qui réclamait l'impossible.
   */
  resubmit: (payload: SaveDraftPayload): Promise<SubmitProposalResult> =>
    send(`/proposals/${payload.proposal_id}/resubmit`, payload, (m) => {
      const result = m.resubmitProposal(payload)
      if (!result) throw new Error(`Dossier ${payload.proposal_id} introuvable.`)
      return result
    }),

  /**
   * DÉPÔT — la transition `draft → submitted`. Les deux refus possibles sont
   * ceux de `tg_check_submission_eligibility()` : appel clos, plafond
   * atteint. Ils ne sont pas des erreurs de réseau mais des réponses, et
   * l'écran les rend comme telles.
   */
  submit: (personId: Uuid, payload: SubmitProposalPayload): Promise<SubmitProposalResult> =>
    send(`/proposals/${payload.proposal_id}/submit`, payload, (m) =>
      m.submitProposal(personId, payload),
    ),

  /**
   * LA PERSONNE QUI PORTE CETTE ADRESSE, si la plateforme la connaît.
   *
   * Même intention que `organizations.similar()` : ne pas créer une seconde
   * fiche pour quelqu'un qui existe déjà. La clé est l'adresse et rien
   * d'autre — `people.primary_email` est la clé de rapprochement du modèle,
   * et chercher par nom rapprocherait deux homonymes, ce qui est pire qu'un
   * doublon. Aucun appel ne rend l'annuaire entier : une plateforme ne
   * diffuse pas sa liste de contacts pour remplir un formulaire.
   */
  lookupSpeaker: (email: string) =>
    call('/people/lookup', (m) => m.lookupSpeakerByEmail(email), { email }),

  /**
   * L'HISTORIQUE CHAMP PAR CHAMP — `programme.proposal_history()`.
   *
   * Sous-produit du journal d'audit, et non une table entretenue à la main :
   * toute écriture y figure, y compris une correction faite en console. La
   * v1 tenait une table `activity_modifications` alimentée par le code
   * applicatif, qui ne couvrait que ce qui passait par le bon chemin.
   */
  fieldHistory: (id: Uuid) =>
    call(`/proposals/${id}/history`, (m) => m.proposalHistory(id)),

  themes: (id: Uuid) =>
    call(`/proposals/${id}/themes`, (m) => {
      const termIds = new Set(
        m.entityTerms
          .filter((t) => t.entity_table === 'proposals' && t.entity_id === id)
          .map((t) => t.term_id),
      )
      return m.taxonomyTerms.filter((t) => termIds.has(t.id))
    }),  }
}
