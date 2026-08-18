/**
 * LE TABLEAU DE BORD DU BACK-OFFICE (A6), composé à partir des projections et
 * des données simulées.
 *
 * CE FICHIER NE RECOMPTE RIEN QUI SOIT DÉJÀ COMPTÉ. L'entonnoir, les courbes et
 * la charge du comité viennent de `mocks/analytics.ts`, la liste des dossiers de
 * `mocks/views.ts`, les chevauchements de `mocks/conflicts.ts`, les incidents de
 * `mocks/incidents.ts`. Ce qu'il ajoute est le seul travail que la base ne fait
 * pas : décider CE QUI DEMANDE UNE ACTION, et le ranger par urgence.
 *
 * AUCUNE VUE DU MODÈLE NE RÉPOND À LA ZONE 1, et c'est normal : « ce qui demande
 * une action » n'est pas une donnée, c'est un jugement — cinq familles réunies
 * par un critère métier (quelqu'un de l'équipe peut le régler aujourd'hui) et un
 * seuil de proximité d'échéance. Cette composition appartiendra à l'API (prompt
 * B7), pas à une vue SQL de plus.
 *
 * LE PÉRIMÈTRE D'ADMINISTRATION N'EST PAS VÉRIFIÉ ICI : `useApi()` s'en charge
 * avant d'appeler ces fonctions (`assertEventInScope`), et l'API le vérifiera
 * pour de bon. Une composition qui filtrerait elle aussi donnerait deux règles à
 * tenir d'accord.
 */

import type {
  AdminAction,
  AdminActionExample,
  AdminDashboard,
  BreakdownSlice,
  DashboardFigures,
  DashboardKpi,
  TrendPoint,
} from '~/types/admin-dashboard'
import type { ProposalFunnelRow } from '~/types/analytics'
import type { Numeric } from '~/types/shared'
import { allProposals } from './proposals'
import { callsForProposals } from './calls'
import { countries, entityTerms, taxonomyTerms } from './reference'
import { duplicateCandidates, organizations } from './org'
import { reviewAssignments } from './reviews'
import { events } from './event'
import { detectConflicts } from './conflicts'
import { activeIncidentsForEvent } from './incidents'
import {
  dailyRegistrations,
  dailySubmissions,
  lastAnalyticsRefresh,
  operationalHealth,
  proposalFunnel,
  reviewerWorkload,
} from './analytics'
import { proposalDashboard } from './views'
import { effectiveDeadline } from '~/utils/call'

/**
 * FENÊTRE D'ALERTE SUR L'ÉCHÉANCE — vingt et un jours.
 *
 * Un dossier déposé sans évaluation n'est pas une anomalie : c'est l'état normal
 * d'un dossier récent. Il ne devient une alerte qu'à l'approche de l'échéance qui
 * lui est applicable — celle de son affectation de revue, ou à défaut la clôture
 * de l'appel —, quand le temps de l'évaluer commence à manquer.
 *
 * TROIS SEMAINES EST UN RÉGLAGE D'ÉCRAN, et c'est une faiblesse assumée : le
 * modèle ne porte aucun délai de ce genre. Il devra devenir un paramètre
 * d'édition le jour où l'IFDD voudra en changer — écart inscrit au journal.
 */
const DEADLINE_ALERT_DAYS = 21

/** Trois exemples au plus : au-delà, la ligne cesse d'être un résumé. */
const MAX_EXAMPLES = 3

const organizationById = new Map(organizations.map((o) => [o.id, o]))
const countryById = new Map(countries.map((c) => [c.id, c]))
const termById = new Map(taxonomyTerms.map((t) => [t.id, t]))

// ---------------------------------------------------------------------------
// Zone 1 — ce qui demande une action
// ---------------------------------------------------------------------------

function unreviewedProposals(eventId: string, at: number): AdminAction | null {
  const call = callsForProposals.find((c) => c.event_id === eventId) ?? null
  const callDeadline = call ? Date.parse(effectiveDeadline(call)) : null

  /**
   * L'ÉCHÉANCE QUI S'APPLIQUE À UN DOSSIER n'est pas toujours celle de l'appel.
   *
   * Un dossier confié à des révisionnistes porte SA date : celle de
   * l'affectation la plus proche (`review_assignments.due_at`). C'est elle qui
   * dit quand le comité l'attend, et elle tombe couramment bien avant la clôture
   * — le comité évalue au fil de l'eau. Ne regarder que la clôture ferait
   * découvrir un dossier vierge de revue le jour où plus personne n'a le temps.
   * Un dossier sans aucune affectation, lui, n'a que la clôture pour horizon.
   */
  function deadlineOf(proposalId: string): number | null {
    const dues = reviewAssignments
      .filter((a) => a.proposal_id === proposalId && a.recused_at === null && a.due_at !== null)
      .map((a) => Date.parse(a.due_at as string))
    return dues.length > 0 ? Math.min(...dues) : callDeadline
  }

  /** Personne n'a été chargé de ce dossier — déports exclus, comme partout. */
  function isUnassigned(proposalId: string): boolean {
    return !reviewAssignments.some((a) => a.proposal_id === proposalId && a.recused_at === null)
  }

  const concernes = proposalDashboard()
    .filter((row) => row.event_id === eventId)
    .filter((row) => row.status === 'submitted' || row.status === 'under_review')
    .filter((row) => row.review_count === 0)
    .map((row) => ({ row, deadline: deadlineOf(row.id), unassigned: isUnassigned(row.id) }))
    /*
     * DEUX FAÇONS D'ENTRER DANS CETTE LIGNE, et la seconde est la plus grave.
     *
     * L'échéance applicable approche (ou est passée) : le temps d'évaluer
     * commence à manquer. Ou bien AUCUN RÉVISIONNISTE N'EST AFFECTÉ — et alors
     * aucune échéance ne court, personne n'ouvrira ce dossier, et attendre la
     * proximité de la clôture pour s'en apercevoir est exactement le piège. Un
     * dossier déposé la veille et déjà confié à trois personnes, lui, n'est pas
     * une anomalie : c'est le fonctionnement normal.
     */
    .filter(
      (entry): entry is { row: (typeof entry)['row']; deadline: number; unassigned: boolean } =>
        entry.deadline !== null &&
        (entry.unassigned || entry.deadline - at <= DEADLINE_ALERT_DAYS * 86_400_000),
    )
    .sort((a, b) => a.deadline - b.deadline)

  if (concernes.length === 0) return null

  return {
    kind: 'proposals_unreviewed',
    severity: 'high',
    count: concernes.length,
    due_at: new Date(concernes[0]?.deadline ?? at).toISOString(),
    examples: concernes.slice(0, MAX_EXAMPLES).map<AdminActionExample>(({ row }) => ({
      label: row.title.fr,
      hint: row.reference_code,
      target: `/admin/propositions/${row.id}`,
    })),
    // La liste des propositions, DÉJÀ RÉGLÉE sur le problème : la ligne ne
    // renvoie pas vers une liste où il faudrait refaire le tri à la main.
    target: '/admin/propositions?filtre=non-evaluees',
  }
}

function overdueReviews(eventId: string, at: number): AdminAction | null {
  const enRetard = reviewerWorkload(eventId, at)
    .filter((row) => row.revues_en_retard > 0)
    .sort((a, b) => b.revues_en_retard - a.revues_en_retard)

  const total = enRetard.reduce((sum, row) => sum + row.revues_en_retard, 0)
  if (total === 0) return null

  const echeances = enRetard
    .map((row) => row.prochaine_echeance)
    .filter((d): d is string => d !== null)
    .sort()

  return {
    kind: 'reviews_overdue',
    severity: 'high',
    count: total,
    due_at: echeances[0] ?? null,
    examples: enRetard.slice(0, MAX_EXAMPLES).map<AdminActionExample>((row) => ({
      label: row.revisionniste,
      hint: String(row.revues_en_retard),
      target: `/admin/evaluations?revisionniste=${row.reviewer_id}`,
    })),
    target: '/admin/evaluations?filtre=en-retard',
  }
}

/**
 * LES DOUBLONS NE SONT PAS FILTRÉS PAR ÉDITION, et ils ne peuvent pas l'être :
 * une organisation n'appartient à aucune COP. La ligne remonte donc pour tout
 * administrateur — elle ne révèle rien d'une autre édition, seulement deux
 * fiches à confondre. C'est le défaut n° 1 de la v1, et le laisser courir coûte
 * une organisation en double dans chaque écran de la plateforme.
 */
function pendingDuplicates(): AdminAction | null {
  const enAttente = duplicateCandidates
    .filter((d) => d.reviewed_at === null)
    .sort((a, b) => b.score - a.score)
  if (enAttente.length === 0) return null

  return {
    kind: 'organization_duplicates',
    severity: 'medium',
    count: enAttente.length,
    due_at: null,
    examples: enAttente.slice(0, MAX_EXAMPLES).map<AdminActionExample>((pair) => {
      const gauche = organizationById.get(pair.left_id)
      const droite = organizationById.get(pair.right_id)
      return {
        label: `${gauche?.legal_name ?? '?'} · ${droite?.legal_name ?? '?'}`,
        // Le score de rapprochement, tel que `compute_trust_score()` le rend :
        // c'est lui qui dit par quelle paire commencer.
        hint: `${pair.score.toFixed(1)}`,
        target: `/admin/organisations/doublons/${pair.id}`,
      }
    }),
    target: '/admin/organisations/doublons',
  }
}

/**
 * LES CHEVAUCHEMENTS SONT SIGNALÉS, JAMAIS BLOQUÉS — règle métier n° 2. Cette
 * ligne existe pour qu'ils se voient, pas pour empêcher quoi que ce soit : les
 * organisations proposent librement, l'équipe arbitre. Le seul garde-fou dur se
 * situe à la publication du programme.
 *
 * `detect_conflicts()` ne connaît pas la notion de conflit « résolu » : un
 * chevauchement disparaît quand on déplace un bloc, pas quand on le déclare
 * réglé. Non résolu signifie donc simplement : encore détecté à cet instant.
 */
function scheduleConflicts(eventId: string): AdminAction | null {
  const conflits = detectConflicts(eventId)
  if (conflits.length === 0) return null

  // « Bloquant » au sens du modèle — un seul stand, un seul direct : deux
  // séances qui ne peuvent MATÉRIELLEMENT pas se tenir en même temps. Le mot
  // décrit la gravité, pas un refus d'écriture : rien n'est jamais bloqué.
  const graves = conflits.filter((c) => c.severity === 'blocking')

  return {
    kind: 'schedule_conflicts',
    severity: graves.length > 0 ? 'high' : 'medium',
    count: conflits.length,
    due_at: null,
    examples: [...graves, ...conflits.filter((c) => c.severity !== 'blocking')]
      .slice(0, MAX_EXAMPLES)
      .map<AdminActionExample>((conflit) => ({
        label: `${conflit.session_a_title} · ${conflit.session_b_title}`,
        hint: conflit.subject_label,
        target: '/admin/programmation',
      })),
    target: '/admin/programmation?filtre=conflits',
  }
}

function activeIncidents(eventId: string, at: number): AdminAction | null {
  const actifs = activeIncidentsForEvent(eventId, at)
  if (actifs.length === 0) return null

  return {
    kind: 'active_incidents',
    // UN INCIDENT ACTIF EST VU DU PUBLIC. Même bénin, il s'affiche en bandeau
    // sur la programmation : il ne peut pas attendre le lendemain.
    severity: 'high',
    count: actifs.length,
    due_at: null,
    examples: actifs.slice(0, MAX_EXAMPLES).map<AdminActionExample>((incident) => ({
      label: incident.title?.fr ?? incident.message.fr,
      hint: incident.target_label,
      target: `/admin/incidents/${incident.incident_id}`,
    })),
    target: '/admin/incidents',
  }
}

/** Le bloc entier, rangé : gravité d'abord, échéance la plus proche ensuite. */
function buildActions(eventId: string, at: number): AdminAction[] {
  const lignes = [
    unreviewedProposals(eventId, at),
    overdueReviews(eventId, at),
    activeIncidents(eventId, at),
    scheduleConflicts(eventId),
    pendingDuplicates(),
  ].filter((action): action is AdminAction => action !== null)

  const rang = { high: 0, medium: 1 } as const
  return lignes.sort(
    (a, b) =>
      rang[a.severity] - rang[b.severity] ||
      (a.due_at ?? '9999').localeCompare(b.due_at ?? '9999') ||
      b.count - a.count,
  )
}

// ---------------------------------------------------------------------------
// Zone 2 — les répartitions
// ---------------------------------------------------------------------------

/**
 * Huit parts au plus, la queue regroupée. Au-delà, une répartition cesse
 * d'informer : les dernières barres sont trop courtes pour se comparer entre
 * elles, et la carte devient plus haute que ce qu'elle apprend.
 */
const MAX_SLICES = 8

function withShare(
  slices: Array<Omit<BreakdownSlice, 'share'>>,
  total: number,
): BreakdownSlice[] {
  const ranges = slices.sort((a, b) => b.count - a.count)
  // Un seul reste ne se regroupe PAS : « 1 autres » est une faute, « 1 autre »
  // n'apprend rien de plus que la part elle-même, et la onzième barre coûte
  // moins que la ligne qui la masque.
  const groupe = ranges.length > MAX_SLICES + 1
  const tete = groupe ? ranges.slice(0, MAX_SLICES) : ranges
  const queue = groupe ? ranges.slice(MAX_SLICES) : []
  const assemble = [...tete]

  if (queue.length > 0) {
    assemble.push({
      key: 'other',
      // Libellé multilingue construit ici parce qu'il DÉSIGNE DES DONNÉES — le
      // reste d'une répartition — et non un élément d'interface. Il suit donc
      // la même règle que les autres parts : il se résout, il ne se traduit pas.
      label: { fr: `${queue.length} autres`, en: `${queue.length} others` },
      color: null,
      count: queue.reduce((sum, slice) => sum + slice.count, 0),
    })
  }

  return assemble.map((slice) => ({
    ...slice,
    share: total === 0 ? 0 : slice.count / total,
  }))
}

/**
 * Les dossiers déposés, par PAYS DE L'ORGANISATION PORTEUSE.
 *
 * Le pays vient de la fiche d'organisation, pas de la personne qui dépose : une
 * chargée de projet basée à Paris qui dépose pour une ONG sénégalaise dépose un
 * dossier sénégalais. C'est la même règle que `v_public_schedule`, qui expose
 * `organization_country_code`.
 */
function byCountry(eventId: string): BreakdownSlice[] {
  const deposes = allProposals.filter(
    (p) => p.deleted_at === null && p.event_id === eventId && p.submitted_at !== null,
  )
  const parPays = new Map<string, Omit<BreakdownSlice, 'share'>>()

  for (const proposal of deposes) {
    const country = countryById.get(organizationById.get(proposal.organization_id)?.country_id ?? '')
    const key = country?.iso2 ?? 'unknown'
    const existant = parPays.get(key)
    if (existant) {
      existant.count += 1
      continue
    }
    parPays.set(key, {
      key,
      // Le nom du pays est une donnée multilingue de `reference.countries` : il
      // se résout à l'affichage, il ne se traduit pas dans un fichier i18n.
      label: country?.name ?? { fr: 'Pays non renseigné', en: 'Country not provided' },
      color: null,
      count: 1,
    })
  }

  return withShare([...parPays.values()], deposes.length)
}

/**
 * Les dossiers déposés, par THÉMATIQUE.
 *
 * UN DOSSIER EN PORTE PLUSIEURS : la somme des parts dépasse le nombre de
 * dossiers, et c'est exact. La part est donc rapportée au nombre de DOSSIERS, ce
 * qui se lit « 40 % des dossiers touchent à l'adaptation » — et non au nombre de
 * rattachements, qui ne veut rien dire.
 *
 * LIBELLÉ ET COULEUR VIENNENT DE LA BASE (`reference.taxonomy_terms`), où un
 * administrateur les modifie. Les figer dans la feuille de style est exactement
 * le défaut n° 1 de la v1.
 */
function byTheme(eventId: string): BreakdownSlice[] {
  const deposes = allProposals.filter(
    (p) => p.deleted_at === null && p.event_id === eventId && p.submitted_at !== null,
  )
  const dossierIds = new Set(deposes.map((p) => p.id))
  const parTerme = new Map<string, number>()

  for (const lien of entityTerms) {
    if (lien.entity_table !== 'proposals' || !dossierIds.has(lien.entity_id)) continue
    const term = termById.get(lien.term_id)
    if (!term || term.taxonomy_code !== 'activity_theme') continue
    parTerme.set(term.code, (parTerme.get(term.code) ?? 0) + 1)
  }

  const slices = [...parTerme.entries()].map(([code, count]) => {
    const term = taxonomyTerms.find((t) => t.code === code)
    return {
      key: code,
      label: term?.label ?? { fr: code },
      color: term?.color_hex ?? null,
      count,
    }
  })

  return withShare(slices, deposes.length)
}

/**
 * Une projection quotidienne, ramenée aux quatre valeurs qu'une courbe affiche.
 *
 * LA MOYENNE MOBILE VIENT DE LA BASE, elle ne se recalcule pas ici. Les deux
 * projections la portent (`moyenne_mobile_7j`), et une seconde moyenne calculée
 * à l'écran finirait par ne plus correspondre à celle qu'un export SQL rendrait.
 */
function toTrend<T extends { jour: string; moyenne_mobile_7j: Numeric | null }>(
  rows: T[],
  valeur: (row: T) => number,
  cumul: (row: T) => number,
): TrendPoint[] {
  return rows.map((row) => ({
    jour: row.jour,
    valeur: valeur(row),
    cumul: cumul(row),
    moyenne_7j: row.moyenne_mobile_7j,
  }))
}

// ---------------------------------------------------------------------------
// Zone 2 — les six indicateurs de tête
// ---------------------------------------------------------------------------

/**
 * VINGT ET UN JOURS D'ÉTINCELLE. Trois semaines se lisent dans une carte de la
 * largeur d'une colonne ; cent jours y produisent un peigne dont on ne retire
 * rien. La courbe complète est juste en dessous, avec son axe et ses repères —
 * l'étincelle ne la remplace pas, elle dit seulement « ça monte » ou « ça
 * retombe ».
 */
const SPARK_DAYS = 21

/** Fenêtre de comparaison : les sept derniers jours face aux sept précédents. */
const DELTA_DAYS = 7

function spark(points: TrendPoint[]): number[] {
  return points.slice(-SPARK_DAYS).map((point) => point.valeur)
}

/**
 * VARIATION SUR SEPT JOURS, ET `null` QUAND LA SÉRIE EST TROP COURTE.
 *
 * Une variation demande deux semaines de série pour vouloir dire quelque chose.
 * Calculée sur quatre jours, elle compare une semaine pleine à une semaine
 * tronquée et affiche un effondrement qui n'a pas eu lieu — le genre de chiffre
 * qu'on cite en réunion avant de découvrir qu'il ne mesurait rien.
 */
function weeklyDelta(points: TrendPoint[]): number | null {
  if (points.length < DELTA_DAYS * 2) return null
  const sum = (rows: TrendPoint[]) => rows.reduce((total, point) => total + point.valeur, 0)
  return sum(points.slice(-DELTA_DAYS)) - sum(points.slice(-DELTA_DAYS * 2, -DELTA_DAYS))
}

/** Seuil de la carte d'échéance : une semaine, au-delà rien ne presse. */
const DEADLINE_WARNING_DAYS = 7

/**
 * LES SIX CHIFFRES DE TÊTE, chacun tracé à une colonne du modèle.
 *
 * AUCUN N'EST INVENTÉ ICI, et c'est la contrainte qui a décidé de la liste :
 * dépôts, sélectivité et séances issues des dossiers viennent de
 * `mv_proposal_funnel`, l'avancement du comité de `mv_reviewer_workload`, les
 * inscriptions de `mv_daily_registrations`, l'échéance de
 * `event.effective_deadline()`. Les indicateurs qu'on aurait aimé y mettre —
 * « activités restant à placer », qui demanderait de compter les séances sans
 * salle — n'y sont pas : aucune projection ne les porte, et les calculer à
 * l'écran donnerait un chiffre qui divergerait du planificateur.
 *
 * LA COULEUR DIT UN ÉTAT, jamais l'importance. La plupart de ces chiffres sont
 * neutres : ils ne sont ni bons ni mauvais, ils situent. Colorer les six revient
 * à n'en signaler aucun.
 */
function buildKpis(
  eventId: string,
  at: number,
  funnel: ProposalFunnelRow | null,
  submissions: TrendPoint[],
  registrations: TrendPoint[],
  deadline: string | null,
  opensAt: string | null,
): DashboardKpi[] {
  const charge = reviewerWorkload(eventId, at)
  const attendues = charge.reduce((total, row) => total + row.propositions_assignees, 0)
  const rendues = charge.reduce((total, row) => total + row.revues_soumises, 0)
  const enRetard = charge.reduce((total, row) => total + row.revues_en_retard, 0)

  /**
   * Jours entiers restants. Négatif ou nul quand l'échéance est derrière —
   * l'écran le dit alors en toutes lettres plutôt que d'afficher « 0 jour », qui
   * se lirait « aujourd'hui ».
   */
  const joursRestants =
    deadline === null ? null : Math.ceil((Date.parse(deadline) - at) / 86_400_000)

  /**
   * DURÉE TOTALE DE LA FENÊTRE DE DÉPÔT, de l'ouverture à l'échéance qui fait
   * foi. C'est elle qui donne son sens aux jours restants : « 44 jours » sur une
   * fenêtre de deux mois n'est pas « 44 jours » sur une fenêtre de six.
   */
  const fenetre =
    deadline === null || opensAt === null
      ? null
      : Math.max(1, Math.ceil((Date.parse(deadline) - Date.parse(opensAt)) / 86_400_000))

  const seances = funnel?.sessions_programmees ?? null

  return [
    {
      key: 'submissions',
      value: funnel?.deposees ?? null,
      out_of: null,
      delta: weeklyDelta(submissions),
      at: null,
      spark: spark(submissions),
      tone: 'neutral',
    },
    {
      key: 'deadline',
      value: joursRestants,
      out_of: fenetre,
      delta: null,
      at: deadline,
      spark: [],
      // Une échéance PASSÉE n'est pas une alerte : c'est un fait acquis, et la
      // laisser en rouge pendant les six mois qui suivent une COP apprend à
      // l'équipe à ne plus regarder la couleur. Gris pour ce qui est clos.
      tone:
        joursRestants !== null && joursRestants > 0 && joursRestants <= DEADLINE_WARNING_DAYS
          ? 'warning'
          : 'neutral',
    },
    {
      key: 'review_progress',
      value: attendues === 0 ? null : rendues,
      out_of: attendues === 0 ? null : attendues,
      delta: null,
      at: null,
      spark: [],
      tone:
        enRetard > 0
          ? 'danger'
          : attendues > 0 && rendues >= attendues
            ? 'success'
            : 'neutral',
    },
    {
      key: 'acceptance_rate',
      // `null` quand aucun dossier n'a été tranché — et non zéro.
      value: funnel?.taux_acceptation ?? null,
      out_of: funnel?.decidees ?? null,
      delta: null,
      at: null,
      spark: [],
      tone: 'neutral',
    },
    /*
     * SÉANCES CRÉÉES, SANS DÉNOMINATEUR — et l'absence de dénominateur est le
     * fruit d'une vérification, pas d'un oubli.
     *
     * `sessions_programmees` compte les séances issues d'un dossier de l'appel,
     * annulations exclues, QUEL QUE SOIT LE SORT DU DOSSIER : la vue joint les
     * propositions sans filtrer sur « acceptée ». Les deux ensembles ne sont donc
     * pas emboîtés, et le jeu de données le montre — 18 séances pour 16 dossiers
     * retenus. Écrire « 18 sur 16 » affirmerait un rapport qui n'existe pas.
     *
     * Le chiffre qu'on aurait voulu — « combien de dossiers retenus attendent
     * encore une séance », l'écart n° 57 du journal — demande de compter les
     * dossiers acceptés sans séance. Aucune projection ne le porte ; il viendra
     * avec elle, pas avec une soustraction fausse.
     */
    {
      key: 'scheduled',
      value: seances,
      out_of: null,
      delta: null,
      at: null,
      spark: [],
      tone: 'neutral',
    },
    {
      key: 'registrations',
      value: registrations.at(-1)?.cumul ?? null,
      out_of: null,
      delta: weeklyDelta(registrations),
      at: null,
      spark: spark(registrations),
      tone: 'neutral',
    },
  ]
}

function buildFigures(eventId: string, at: number): DashboardFigures {
  const call = callsForProposals.find((c) => c.event_id === eventId) ?? null
  const funnel =
    proposalFunnel().find((row) => row.event_id === eventId && row.call_id === (call?.id ?? null)) ??
    null

  const submissions = dailySubmissions(eventId)
  const registrations = dailyRegistrations(eventId)

  const submissionPoints = toTrend(
    submissions,
    (row) => row.soumissions,
    (row) => row.soumissions_cumulees,
  )
  const registrationPoints = toTrend(
    registrations,
    (row) => row.inscriptions,
    (row) => row.inscriptions_cumulees,
  )
  // `event.effective_deadline()` : la prolongation prime la clôture initiale.
  const deadline = call ? effectiveDeadline(call) : null

  return {
    // LES INDICATEURS EN PREMIER, comme à l'écran : ils répondent à « où en
    // est-on », que ni les courbes ni les répartitions ne disent.
    kpis: buildKpis(
      eventId,
      at,
      funnel,
      submissionPoints,
      registrationPoints,
      deadline,
      call?.opens_at ?? null,
    ),
    funnel,
    submissions: submissionPoints,
    registrations: registrationPoints,
    deadline,
    call_opens_at: call?.opens_at ?? null,
    by_country: byCountry(eventId),
    by_theme: byTheme(eventId),
    refreshed_at: lastAnalyticsRefresh(at),
  }
}

// ---------------------------------------------------------------------------
// La réponse entière
// ---------------------------------------------------------------------------

/**
 * Tout l'écran, pour une édition. Rend `null` quand l'édition n'existe pas —
 * l'écran affiche alors son état vide plutôt qu'un tableau de bord de néant.
 */
export function adminDashboard(eventId: string, at: number = Date.now()): AdminDashboard | null {
  const edition = events.find((e) => e.id === eventId)
  if (!edition) return null

  return {
    edition,
    timezone: edition.timezone,
    call: callsForProposals.find((c) => c.event_id === eventId) ?? null,
    actions: buildActions(eventId, at),
    figures: buildFigures(eventId, at),
    health: operationalHealth(at),
    incidents: activeIncidentsForEvent(eventId, at),
  }
}
