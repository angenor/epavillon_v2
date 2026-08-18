/**
 * LA LISTE DES PROPOSITIONS DU BACK-OFFICE (A7), composée à partir de la vue et
 * des données simulées.
 *
 * CE FICHIER NE RECALCULE RIEN QUE LA VUE PORTE DÉJÀ. Les lignes viennent de
 * `mocks/views.ts`, c'est-à-dire de `programme.v_proposal_dashboard` — thèmes,
 * pays, révisionnistes, retards et rang y ont été ajoutés le 18/08 pour que cet
 * écran tienne en une requête. Ce qui s'ajoute ici est le seul travail que la
 * base ne fait pas : compter les FACETTES des filtres, croiser les dossiers non
 * lus avec la personne connectée, et calculer la RÉPONSE des actions groupées.
 *
 * LES BROUILLONS RESTENT DANS LA LISTE, et c'est une décision. « Propositions
 * reçues » invite à les écarter ; l'équipe de l'IFDD, elle, a besoin de voir à
 * trois jours de l'échéance qu'il reste cinq dossiers commencés et jamais
 * déposés — l'entonnoir du tableau de bord les compte déjà
 * (`mv_proposal_funnel.brouillons`), et la vue ne filtre que les dossiers
 * supprimés. Ils sont donc affichés, marqués comme tels, sans note ni rang, et
 * le filtre de statut permet de les écarter en un clic.
 *
 * LES ÉCRITURES SONT DES ÉCRITURES DE SESSION, comme au rattachement
 * d'organisation (`mocks/organization-search.ts`) : affecter un révisionniste ou
 * changer un statut modifie un tampon en mémoire que la lecture suivante
 * applique par-dessus les données simulées. Sans cela, l'écran annoncerait
 * « 6 dossiers confiés » puis se rechargerait comme si rien n'avait eu lieu — un
 * back-office qui ment sur ce qu'il vient de faire.
 *
 * LE PÉRIMÈTRE D'ADMINISTRATION N'EST PAS VÉRIFIÉ ICI : `useApi()` s'en charge
 * avant d'appeler ces fonctions (`assertEventInScope`), et l'API le vérifiera
 * pour de bon.
 */

import type {
  AssignReviewerPayload,
  BulkResult,
  BulkSkip,
  ChangeStatusPayload,
  ProposalFacet,
  ProposalFacets,
  ProposalListScreen,
} from '~/types/admin-proposals'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { ReviewAssignment } from '~/types/programme/review'
import type { ProposalDashboardRow } from '~/types/views'
import type { ParticipationMode } from '~/types/event/edition'
import type { Uuid } from '~/types/shared'
import { REVIEW_ASSIGNMENT } from './ids'
import { events } from './event'
import { callReviewers, callsForProposals } from './calls'
import { countries } from './reference'
import { organizations } from './org'
import { people } from './people'
import { allProposals } from './proposals'
import { proposalReads } from './proposal-reads'
import { reviewAssignments } from './reviews'
import { proposalTransitionsAllowed } from './proposal-workflow'
import { proposalDashboard } from './views'
import { effectiveDeadline } from '~/utils/call'

const organizationById = new Map(organizations.map((o) => [o.id, o]))
const countryByIso = new Map(countries.map((c) => [c.iso2, c]))
const personById = new Map(people.map((p) => [p.id, p]))

// ---------------------------------------------------------------------------
// Écritures de session
// ---------------------------------------------------------------------------

/** Statuts changés depuis le back-office pendant cette visite. */
const sessionStatuses = new Map<Uuid, ProposalStatus>()
/** Affectations ajoutées pendant cette visite. */
const sessionAssignments: ReviewAssignment[] = []
let sessionAssignmentSeq = 900

/**
 * Les affectations de la base ET celles de la visite. Les déports restent
 * exclus par l'appelant, comme le fait la vue.
 */
function allAssignments(): ReviewAssignment[] {
  return [...reviewAssignments, ...sessionAssignments]
}

/**
 * LE TAMPON EST PARTAGÉ AVEC LA FICHE D'ÉVALUATION (A8), et il doit l'être.
 *
 * Un dossier retenu depuis la fiche puis relu dans la liste doit y apparaître
 * retenu ; un dossier confié depuis la liste doit apparaître dans le comité de
 * la fiche. Deux tampons séparés donneraient deux vérités selon l'écran par
 * lequel on passe — le défaut qu'on reproche à la v1, à l'échelle d'une visite.
 * Ces deux accesseurs sont donc exportés : `mocks/proposal-review.ts` lit ici,
 * et n'écrit un statut que par `changeProposalStatus()`.
 */
export function sessionProposalStatus(proposalId: Uuid): ProposalStatus | undefined {
  return sessionStatuses.get(proposalId)
}

/** Affectations ajoutées pendant la visite, tous dossiers confondus. */
export function sessionReviewAssignments(): ReviewAssignment[] {
  return [...sessionAssignments]
}

/**
 * La vue, avec les écritures de session appliquées par-dessus.
 *
 * Le statut et les révisionnistes sont les deux seules colonnes qu'une action
 * groupée touche. Le rang et les notes, eux, ne bougent pas : ils dépendent des
 * revues, qu'aucune action de cet écran ne produit.
 */
function rowsWithSession(): ProposalDashboardRow[] {
  return proposalDashboard().map((row) => {
    const status = sessionStatuses.get(row.id)
    const added = sessionAssignments.filter((a) => a.proposal_id === row.id)
    if (!status && added.length === 0) return row

    return {
      ...row,
      status: status ?? row.status,
      assigned_reviewers: row.assigned_reviewers + added.length,
      reviewer_ids: [...row.reviewer_ids, ...added.map((a) => a.reviewer_id)],
      reviewers: [
        ...row.reviewers,
        ...added.map((a) => ({
          person_id: a.reviewer_id,
          name: personById.get(a.reviewer_id)?.display_name ?? '',
          due_at: a.due_at,
          submitted_at: null,
        })),
      ].sort((a, b) => a.name.localeCompare(b.name, 'fr')),
    }
  })
}

// ---------------------------------------------------------------------------
// Facettes
// ---------------------------------------------------------------------------

/**
 * Une facette compte SUR LE PÉRIMÈTRE, filtres non appliqués : « Retenu (17) »
 * doit rester lisible quand on a déjà coché « En évaluation ». Des décomptes
 * recalculés à chaque coche tomberaient tous à zéro sauf un, et le filtre
 * cesserait de dire ce qu'il reste à explorer.
 *
 * L'ordre est celui du décompte décroissant, sauf pour les statuts et les
 * formats, dont l'ordre est celui de l'ENUM : un cycle de vie se lit dans son
 * sens, pas par popularité.
 */
function countBy<T>(rows: ProposalDashboardRow[], extract: (row: ProposalDashboardRow) => T[]): Map<T, number> {
  const counts = new Map<T, number>()
  for (const row of rows) {
    for (const value of extract(row)) {
      counts.set(value, (counts.get(value) ?? 0) + 1)
    }
  }
  return counts
}

const STATUS_ORDER: ProposalStatus[] = [
  'draft',
  'submitted',
  'under_review',
  'changes_requested',
  'accepted',
  'rejected',
  'withdrawn',
  'cancelled',
]

const FORMAT_ORDER: ParticipationMode[] = ['in_person', 'online', 'hybrid']

function byCountDesc(a: ProposalFacet, b: ProposalFacet): number {
  return b.count - a.count
}

function facets(rows: ProposalDashboardRow[], unreadIds: Set<Uuid>): ProposalFacets {
  const statusCounts = countBy(rows, (row) => [row.status])
  const formatCounts = countBy(rows, (row) => [row.format])
  const themeCounts = countBy(rows, (row) => row.theme_codes)
  const countryCounts = countBy(rows, (row) =>
    row.organization_country_code ? [row.organization_country_code] : [],
  )
  const organizationCounts = countBy(rows, (row) => [row.organization_id])
  const reviewerCounts = countBy(rows, (row) => row.reviewer_ids)

  // Les libellés des thématiques viennent de la ligne elle-même : la vue les
  // porte déjà résolus par `term_badges()`. Les recharger depuis la taxonomie
  // serait refaire ce que la base a fait, avec un risque de divergence.
  const themeLabels = new Map(rows.flatMap((row) => row.themes.map((t) => [t.code, t] as const)))

  return {
    // Statut et format : aucun libellé, ce sont des ENUM que l'écran traduit.
    statuses: STATUS_ORDER.filter((status) => statusCounts.has(status)).map((status) => ({
      value: status,
      label: null,
      count: statusCounts.get(status) ?? 0,
    })),
    formats: FORMAT_ORDER.filter((format) => formatCounts.has(format)).map((format) => ({
      value: format,
      label: null,
      count: formatCounts.get(format) ?? 0,
    })),
    // Thématique, pays, organisation, personne : le libellé vient de la BASE.
    themes: [...themeCounts.entries()]
      .map(([code, count]) => ({
        value: code,
        label: themeLabels.get(code)?.label ?? code,
        color: themeLabels.get(code)?.color ?? null,
        count,
      }))
      .sort(byCountDesc),
    countries: [...countryCounts.entries()]
      .map(([iso2, count]) => ({
        value: iso2,
        label: countryByIso.get(iso2)?.name ?? iso2,
        count,
      }))
      .sort(byCountDesc),
    organizations: [...organizationCounts.entries()]
      .map(([id, count]) => ({
        value: id,
        // Le sigle abrège une liste déroulante que le nom légal rendrait
        // illisible ; le nom reste en second rang dans le composant.
        label: organizationById.get(id)?.legal_name ?? '',
        count,
      }))
      .sort(byCountDesc),
    reviewers: [...reviewerCounts.entries()]
      .map(([id, count]) => ({
        value: id,
        label: personById.get(id)?.display_name ?? '',
        count,
      }))
      .sort(byCountDesc),
    flags: [
      {
        value: 'unreviewed',
        label: null,
        // « Non évaluée » ne veut pas dire « sans révisionniste » : un dossier
        // confié à trois personnes dont aucune n'a rendu sa note est non évalué.
        count: rows.filter((row) => row.review_count === 0 && row.status !== 'draft').length,
      },
      {
        value: 'late',
        label: null,
        count: rows.filter((row) => row.overdue_reviews > 0).length,
      },
      {
        value: 'unread',
        label: null,
        count: rows.filter((row) => unreadIds.has(row.id)).length,
      },
    ],
  }
}

// ---------------------------------------------------------------------------
// L'écran
// ---------------------------------------------------------------------------

/**
 * `programme.unread_proposals_for()` rejouée : les dossiers de l'édition que
 * cette personne n'a JAMAIS ouverts. Les écritures de session ne s'y appliquent
 * pas — cet écran ne pose aucun accusé de lecture, c'est l'ouverture de la fiche
 * (A8) qui le fera.
 */
function unreadProposalsFor(personId: Uuid, eventId: Uuid): Uuid[] {
  return allProposals
    .filter((p) => p.event_id === eventId && p.deleted_at === null)
    .filter((p) => !proposalReads.some((r) => r.proposal_id === p.id && r.person_id === personId))
    .map((p) => p.id)
}

export function proposalListScreen(eventId: Uuid, personId: Uuid | null): ProposalListScreen | null {
  const event = events.find((e) => e.id === eventId)
  if (!event) return null

  const call = callsForProposals.find((c) => c.event_id === eventId) ?? null
  const rows = rowsWithSession().filter((row) => row.event_id === eventId)
  const unreadIds = personId ? unreadProposalsFor(personId, eventId) : []

  return {
    event_id: eventId,
    // Le fuseau de l'ÉDITION, jamais celui du navigateur : une échéance de revue
    // annoncée à 23 h 59 l'est à Belém, pas à Québec.
    timezone: event.timezone,
    city: event.city,
    deadline: call ? effectiveDeadline(call) : null,
    required_reviews: call?.required_reviews ?? null,
    rows,
    facets: facets(rows, new Set(unreadIds)),
    unread_ids: unreadIds,
  }
}

/** Composition du comité de cet appel — qui peut recevoir une affectation. */
export function committeeOf(eventId: Uuid): ProposalFacet[] {
  const call = callsForProposals.find((c) => c.event_id === eventId) ?? null
  if (!call) return []

  const assignments = allAssignments()

  return callReviewers
    .filter((r) => r.call_id === call.id)
    .map((r) => ({
      value: r.person_id,
      label: personById.get(r.person_id)?.display_name ?? '',
      // La charge actuelle, déports exclus : on ne confie pas douze dossiers de
      // plus à quelqu'un qui en porte déjà vingt, et `workload_cap` existe en
      // base pour cela.
      count: assignments.filter((a) => a.reviewer_id === r.person_id && a.recused_at === null).length,
    }))
    .sort((a, b) => String(a.label).localeCompare(String(b.label), 'fr'))
}

// ---------------------------------------------------------------------------
// Actions groupées
// ---------------------------------------------------------------------------

/**
 * AFFECTER UN RÉVISIONNISTE À PLUSIEURS DOSSIERS.
 *
 * Trois refus possibles, et chacun se dit : la personne est déjà affectée
 * (`ux_review_assignments` l'interdit en base), elle s'est DÉPORTÉE de ce
 * dossier — la lui réattribuer effacerait une déclaration d'impartialité —, ou
 * le dossier n'existe pas. Répondre « 9 dossiers confiés » sans nommer les trois
 * autres, c'est le défaut classique des actions de masse.
 */
export function assignReviewer(payload: AssignReviewerPayload, actorId: Uuid | null): BulkResult {
  const applied: Uuid[] = []
  const skipped: BulkSkip[] = []
  const assignments = allAssignments()

  for (const proposalId of payload.proposal_ids) {
    const proposal = allProposals.find((p) => p.id === proposalId)
    if (!proposal) {
      skipped.push({ proposal_id: proposalId, reference_code: '', reason: 'not_found' })
      continue
    }

    const existing = assignments.find(
      (a) => a.proposal_id === proposalId && a.reviewer_id === payload.reviewer_id,
    )

    if (existing?.recused_at) {
      skipped.push({ proposal_id: proposalId, reference_code: proposal.reference_code, reason: 'recused' })
      continue
    }
    if (existing) {
      skipped.push({
        proposal_id: proposalId,
        reference_code: proposal.reference_code,
        reason: 'already_assigned',
      })
      continue
    }

    sessionAssignments.push({
      id: REVIEW_ASSIGNMENT(++sessionAssignmentSeq),
      proposal_id: proposalId,
      reviewer_id: payload.reviewer_id,
      assigned_by: actorId,
      assigned_at: new Date().toISOString(),
      due_at: payload.due_at ?? null,
      recused_at: null,
      recusal_reason: null,
    })
    applied.push(proposalId)
  }

  return { applied, skipped }
}

/**
 * CHANGER LE STATUT DE PLUSIEURS DOSSIERS.
 *
 * La machine à états n'est pas réimplémentée : elle est LUE dans
 * `proposal_transitions_allowed`, comme le trigger `tg_guard_proposal_status()`
 * la lit. Une sélection est hétérogène par nature — sur douze dossiers, quatre
 * sont déposés, six en évaluation, deux déjà retenus — et la même action ne
 * s'applique donc pas à tous. Ce qui ne passe pas est nommé, avec sa raison.
 */
export function changeProposalStatus(payload: ChangeStatusPayload, actorId: Uuid | null): BulkResult {
  void actorId
  const applied: Uuid[] = []
  const skipped: BulkSkip[] = []

  for (const proposalId of payload.proposal_ids) {
    const proposal = allProposals.find((p) => p.id === proposalId)
    if (!proposal) {
      skipped.push({ proposal_id: proposalId, reference_code: '', reason: 'not_found' })
      continue
    }

    const from = sessionStatuses.get(proposalId) ?? proposal.status
    const rule = proposalTransitionsAllowed.find(
      (r) => r.from_status === from && r.to_status === payload.to_status,
    )

    if (!rule) {
      skipped.push({
        proposal_id: proposalId,
        reference_code: proposal.reference_code,
        reason: 'transition_not_allowed',
      })
      continue
    }
    // Le trigger REFUSE la transition sans motif : mieux vaut le dire ici que
    // laisser partir une requête dont on sait qu'elle sera rejetée.
    if (rule.requires_reason && !payload.reason?.trim()) {
      skipped.push({
        proposal_id: proposalId,
        reference_code: proposal.reference_code,
        reason: 'reason_required',
      })
      continue
    }

    sessionStatuses.set(proposalId, payload.to_status)
    applied.push(proposalId)
  }

  return { applied, skipped }
}
