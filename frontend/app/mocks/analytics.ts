/**
 * Les projections de `analytics` (130_analytics.sql), reconstituées à partir des
 * données simulées.
 *
 * ELLES SONT DÉRIVÉES, PAS ÉCRITES À LA MAIN — même principe que `views.ts` :
 * c'est ce que fait la base, et c'est ce qui garantit qu'aucun chiffre du
 * tableau de bord ne contredise le dossier qu'on ouvre juste après. Un entonnoir
 * écrit à la main affiche « 18 déposées » quand la liste en montre 17, et
 * personne ne sait laquelle des deux ment.
 *
 * LES JOURS SONT DÉCOUPÉS EN UTC, explicitement, comme dans les vues : un
 * agrégat ne doit pas changer de valeur selon le fuseau de qui le calcule.
 *
 * LES SÉRIES SONT CONTINUES, jours à zéro compris. C'est une garantie de la
 * base, et les composants de courbe s'y fient : ils ne rebouchent aucun trou.
 *
 * UNE EXCEPTION ASSUMÉE — `operationalHealth()`. Cette vue-là mesure des tables
 * d'INFRASTRUCTURE (outbox, file de travaux, courriels, synchronisations visio,
 * partitions, journal de rafraîchissement) qui ne sont pas des données métier et
 * que `app/mocks/` ne simule pas. Ses lignes sont donc écrites telles que la vue
 * les rendrait, avec les SEUILS EXACTS de `130_analytics.sql` § 11 — c'est la
 * seule façon d'éprouver les trois niveaux de gravité de l'écran.
 */

import type {
  DailyRegistrationRow,
  DailySubmissionRow,
  HealthSeverity,
  OperationalHealthRow,
  PlatformOverview,
  ProposalFunnelRow,
  ReviewerWorkloadRow,
} from '~/types/analytics'
import { allProposals } from './proposals'
import { callsForProposals } from './calls'
import { duplicateCandidates, organizations } from './org'
import { events } from './event'
import { memberships } from './memberships'
import { people } from './people'
import { registrations } from './registrations'
import { reviewAssignments, reviews } from './reviews'
import { allSessions } from './sessions'
import { accounts } from './auth'
import type { OrganizationStatus } from '~/types/org'
import { effectiveDeadline, isCallOpen } from '~/utils/call'

/** Le jour civil d'un instant, découpé en UTC — `(x AT TIME ZONE 'UTC')::date`. */
function utcDay(value: string): string {
  return value.slice(0, 10) === value ? value : new Date(value).toISOString().slice(0, 10)
}

/** Série continue de jours, bornes comprises — `generate_series(..., '1 day')`. */
function dayRange(first: string, last: string): string[] {
  const days: string[] = []
  const end = Date.parse(`${last}T00:00:00Z`)
  for (let t = Date.parse(`${first}T00:00:00Z`); t <= end; t += 86_400_000) {
    days.push(new Date(t).toISOString().slice(0, 10))
  }
  return days
}

/** Moyenne mobile sur sept jours, arrondie à deux décimales comme en base. */
function movingAverage(values: number[], index: number): number {
  const window = values.slice(Math.max(0, index - 6), index + 1)
  return Math.round((window.reduce((sum, v) => sum + v, 0) / window.length) * 100) / 100
}

function round(value: number, digits: number): number {
  const factor = 10 ** digits
  return Math.round(value * factor) / factor
}

const UUID_NUL = '00000000-0000-0000-0000-000000000000'
/** Voir `platformOverview()` : un statut absent du jeu d'essai ne se compare pas en ligne. */
const FUSIONNEES: OrganizationStatus[] = ['merged']
const eventById = new Map(events.map((e) => [e.id, e]))
const personById = new Map(people.map((p) => [p.id, p]))

// ---------------------------------------------------------------------------
// analytics.mv_proposal_funnel — § 3
// ---------------------------------------------------------------------------

/**
 * L'entonnoir, par (événement, appel).
 *
 * DEUX TAUX D'ACCEPTATION, ET ILS NE DISENT PAS LA MÊME CHOSE : `taux_acceptation`
 * mesure la sélectivité du comité, sur les seuls dossiers tranchés ;
 * `taux_acceptation_sur_depots` mesure le rendement de l'appel, retraits
 * compris. Les confondre fait passer un appel où la moitié des organisations
 * abandonne pour un appel très sélectif.
 */
export function proposalFunnel(): ProposalFunnelRow[] {
  // Périmètre : tous les appels déclarés, plus les dépôts hors appel.
  const keys = new Map<string, { event_id: string; call_id: string | null }>()
  for (const call of callsForProposals) {
    keys.set(`${call.event_id}|${call.id}`, { event_id: call.event_id, call_id: call.id })
  }
  for (const proposal of allProposals) {
    if (proposal.deleted_at !== null) continue
    keys.set(`${proposal.event_id}|${proposal.call_id ?? ''}`, {
      event_id: proposal.event_id,
      call_id: proposal.call_id,
    })
  }

  return [...keys.values()].map(({ event_id, call_id }) => {
    const event = eventById.get(event_id)
    const call = callsForProposals.find((c) => c.id === call_id) ?? null
    const dossiers = allProposals.filter(
      (p) => p.deleted_at === null && p.event_id === event_id && p.call_id === call_id,
    )
    const count = (predicate: (p: (typeof dossiers)[number]) => boolean): number =>
      dossiers.filter(predicate).length

    const deposees = count((p) => p.submitted_at !== null)
    const acceptees = count((p) => p.status === 'accepted')
    const rejetees = count((p) => p.status === 'rejected')
    const retirees = count((p) => p.status === 'withdrawn')

    const notes = dossiers.map((p) => p.average_score).filter((n): n is number => n !== null)
    const delais = dossiers
      .filter((p) => p.decided_at !== null && p.submitted_at !== null)
      .map((p) => Date.parse(p.decided_at!) - Date.parse(p.submitted_at!))
      .sort((a, b) => a - b)
    // Médiane et non moyenne : deux dossiers arbitrés très tard déplacent la
    // moyenne de plusieurs semaines et donnent une image fausse du rythme.
    const mediane = delais.length === 0 ? null : (delais[Math.floor((delais.length - 1) / 2)] ?? null)

    const depots = dossiers
      .map((p) => p.submitted_at)
      .filter((d): d is string => d !== null)
      .sort()

    const sessionsProgrammees = allSessions.filter(
      (s) =>
        s.status !== 'cancelled' &&
        s.proposal_id !== null &&
        dossiers.some((p) => p.id === s.proposal_id),
    ).length

    return {
      event_id,
      cle_appel: call_id ?? UUID_NUL,
      call_id,
      evenement: event?.title.fr ?? '',
      edition_year: event?.edition_year ?? 0,
      statut_evenement: event?.status ?? '',
      code_appel: call?.code ?? null,
      appel: call?.title.fr ?? null,
      statut_appel: call?.status ?? null,
      appel_ouvre_le: call?.opens_at ?? null,
      // `event.effective_deadline()` : la prolongation prime la clôture initiale.
      appel_ferme_le: call ? effectiveDeadline(call) : null,
      required_reviews: call?.required_reviews ?? null,
      total: dossiers.length,
      brouillons: count((p) => p.status === 'draft'),
      deposees,
      en_attente_affectation: count((p) => p.status === 'submitted'),
      en_revue: count((p) => p.status === 'under_review'),
      modifications_demandees: count((p) => p.status === 'changes_requested'),
      acceptees,
      rejetees,
      retirees,
      annulees: count((p) => p.status === 'cancelled'),
      decidees: acceptees + rejetees,
      en_instance: Math.max(deposees - acceptees - rejetees - retirees, 0),
      taux_acceptation: acceptees + rejetees === 0 ? null : round(acceptees / (acceptees + rejetees), 4),
      taux_acceptation_sur_depots: deposees === 0 ? null : round(acceptees / deposees, 4),
      organisations_distinctes: new Set(dossiers.map((p) => p.organization_id)).size,
      note_moyenne:
        notes.length === 0 ? null : round(notes.reduce((s, n) => s + n, 0) / notes.length, 2),
      delai_median_decision_heures: mediane === null ? null : round(mediane / 3_600_000, 1),
      premier_depot: depots[0] ?? null,
      dernier_depot: depots[depots.length - 1] ?? null,
      sessions_programmees: sessionsProgrammees,
    }
  })
}

// ---------------------------------------------------------------------------
// analytics.mv_daily_submissions — § 4
// ---------------------------------------------------------------------------

/** Les dépôts jour par jour d'une édition, série continue. */
export function dailySubmissions(eventId: string): DailySubmissionRow[] {
  const event = eventById.get(eventId)
  const dossiers = allProposals.filter((p) => p.deleted_at === null && p.event_id === eventId)
  const call = callsForProposals.find((c) => c.event_id === eventId) ?? null

  const depots = dossiers.filter((p) => p.submitted_at !== null)
  const jours = depots.map((p) => utcDay(p.submitted_at!)).sort()
  const ouverture = call ? utcDay(call.opens_at) : null
  const debut = jours[0] ?? ouverture
  if (!debut) return []

  const echeance = call ? utcDay(effectiveDeadline(call)) : null
  const aujourdhui = new Date().toISOString().slice(0, 10)
  const dernierDepot = jours[jours.length - 1] ?? debut
  // La fenêtre s'arrête à l'échéance ou à aujourd'hui, au plus tôt des deux :
  // prolonger la courbe dans le futur produit une traîne de zéros qui écrase la
  // lecture de ce qui s'est réellement passé.
  const fin = [dernierDepot, ouverture ?? debut, echeance && echeance < aujourdhui ? echeance : aujourdhui]
    .filter((d): d is string => d !== null)
    .sort()
    .at(-1) as string

  const days = dayRange(debut, fin)
  const parJour = days.map(
    (jour) => depots.filter((p) => utcDay(p.submitted_at!) === jour).length,
  )

  let cumul = 0
  return days.map((jour, index) => {
    const soumissions = parJour[index] ?? 0
    cumul += soumissions
    const duJour = depots.filter((p) => utcDay(p.submitted_at!) === jour)
    const decides = dossiers.filter((p) => p.decided_at !== null && utcDay(p.decided_at) === jour)
    return {
      jour,
      event_id: eventId,
      evenement: event?.title.fr ?? '',
      edition_year: event?.edition_year ?? 0,
      soumissions,
      organisations_distinctes: new Set(duJour.map((p) => p.organization_id)).size,
      acceptations: decides.filter((p) => p.status === 'accepted').length,
      rejets: decides.filter((p) => p.status === 'rejected').length,
      soumissions_cumulees: cumul,
      moyenne_mobile_7j: movingAverage(parJour, index),
    }
  })
}

// ---------------------------------------------------------------------------
// analytics.mv_daily_registrations — § 4 bis, ajoutée au modèle le 17/08
// ---------------------------------------------------------------------------

/**
 * Les inscriptions aux activités, jour par jour, pour une édition.
 *
 * À NE PAS CONFONDRE avec `mv_daily_signups`, qui compte des créations de compte
 * sur toute la plateforme et ne porte pas `event_id` — c'est précisément ce qui
 * a rendu cette projection nécessaire pour un administrateur détaché sur une
 * seule édition.
 */
export function dailyRegistrations(eventId: string): DailyRegistrationRow[] {
  const event = eventById.get(eventId)
  const sessionIds = new Set(allSessions.filter((s) => s.event_id === eventId).map((s) => s.id))
  const inscriptions = registrations.filter((r) => sessionIds.has(r.session_id))
  if (inscriptions.length === 0) return []

  const jours = inscriptions.map((r) => utcDay(r.created_at)).sort()
  const debut = jours[0] as string
  const aujourdhui = new Date().toISOString().slice(0, 10)
  const finEdition = event ? utcDay(event.ends_at) : aujourdhui
  const fin = [jours[jours.length - 1] as string, finEdition].sort().at(-1) as string
  const days = dayRange(debut, fin < aujourdhui ? fin : aujourdhui)

  const parJour = days.map(
    (jour) =>
      inscriptions.filter((r) => utcDay(r.created_at) === jour && r.status !== 'cancelled').length,
  )

  let cumul = 0
  return days.map((jour, index) => {
    const duJour = inscriptions.filter((r) => utcDay(r.created_at) === jour)
    const retenues = duJour.filter((r) => r.status !== 'cancelled')
    cumul += retenues.length
    return {
      jour,
      event_id: eventId,
      evenement: event?.title.fr ?? '',
      edition_year: event?.edition_year ?? 0,
      inscriptions: retenues.length,
      liste_attente: duJour.filter((r) => r.status === 'waitlisted').length,
      // Comptées à part, jamais soustraites : une annulation du 12 ne retire
      // rien à l'inscription du 3, elle raconte un autre fait.
      annulations: duJour.filter((r) => r.status === 'cancelled').length,
      presents: duJour.filter((r) => r.joined_at !== null).length,
      personnes_distinctes: new Set(retenues.map((r) => r.person_id)).size,
      inscriptions_cumulees: cumul,
      moyenne_mobile_7j: movingAverage(parJour, index),
    }
  })
}

// ---------------------------------------------------------------------------
// analytics.mv_reviewer_workload — § 7
// ---------------------------------------------------------------------------

/**
 * La charge du comité, par révisionniste et par édition.
 *
 * `revues_en_retard` est la colonne que le bloc d'actions remonte : échéance
 * dépassée, aucune revue soumise, DÉPORTS EXCLUS. Un membre qui s'est déporté
 * pour conflit d'intérêts n'est pas en retard — le compter le serait deux fois :
 * une fois faux, une fois injuste.
 */
export function reviewerWorkload(eventId: string, at: number = Date.now()): ReviewerWorkloadRow[] {
  const dossiers = allProposals.filter((p) => p.deleted_at === null && p.event_id === eventId)
  const dossierIds = new Set(dossiers.map((p) => p.id))
  const affectations = reviewAssignments.filter((a) => dossierIds.has(a.proposal_id))
  const revues = reviews.filter((r) => dossierIds.has(r.proposal_id))
  const event = eventById.get(eventId)

  const soumises = revues.filter((r) => r.submitted_at !== null)
  const moyenneEvenement =
    soumises.length === 0
      ? null
      : round(soumises.reduce((s, r) => s + (r.score_out_of_20 ?? 0), 0) / soumises.length, 2)

  const reviewerIds = new Set([
    ...affectations.map((a) => a.reviewer_id),
    ...revues.map((r) => r.reviewer_id),
  ])

  return [...reviewerIds].map((reviewerId) => {
    const mesAffectations = affectations.filter((a) => a.reviewer_id === reviewerId)
    const actives = mesAffectations.filter((a) => a.recused_at === null)
    const mesRevues = revues.filter((r) => r.reviewer_id === reviewerId)
    const mesSoumises = mesRevues.filter((r) => r.submitted_at !== null)

    const enRetard = actives.filter(
      (a) =>
        a.due_at !== null &&
        Date.parse(a.due_at) < at &&
        !mesSoumises.some((r) => r.proposal_id === a.proposal_id),
    ).length

    const notes = mesSoumises.map((r) => r.score_out_of_20).filter((n): n is number => n !== null)
    const moyenne =
      notes.length === 0 ? null : round(notes.reduce((s, n) => s + n, 0) / notes.length, 2)
    const ecart = moyenne !== null && moyenneEvenement !== null ? round(moyenne - moyenneEvenement, 2) : null

    const echeances = actives
      .map((a) => a.due_at)
      .filter((d): d is string => d !== null)
      .sort()

    return {
      reviewer_id: reviewerId,
      event_id: eventId,
      revisionniste: personById.get(reviewerId)?.display_name ?? '',
      evenement: event?.title.fr ?? '',
      edition_year: event?.edition_year ?? 0,
      propositions_assignees: actives.length,
      deports: mesAffectations.length - actives.length,
      revues_soumises: mesSoumises.length,
      revues_en_cours: mesRevues.length - mesSoumises.length,
      revues_en_retard: enRetard,
      revues_restantes: Math.max(actives.length - mesSoumises.length, 0),
      taux_completion: actives.length === 0 ? null : round(mesSoumises.length / actives.length, 4),
      note_moyenne_attribuee: moyenne,
      note_moyenne_evenement: moyenneEvenement,
      ecart_a_la_moyenne: ecart,
      // Un point d'écart sur 20 est un biais réel ; en deçà, c'est du bruit.
      profil_notation:
        ecart === null ? 'indetermine' : ecart <= -1 ? 'severe' : ecart >= 1 ? 'genereux' : 'dans_la_moyenne',
      prochaine_echeance: echeances[0] ?? null,
      derniere_revue_le:
        mesSoumises
          .map((r) => r.submitted_at)
          .filter((d): d is string => d !== null)
          .sort()
          .at(-1) ?? null,
    }
  })
}

// ---------------------------------------------------------------------------
// analytics.v_platform_overview — § 10
// ---------------------------------------------------------------------------

/**
 * Les compteurs de la plateforme, exacts à la seconde comme la vue — qui n'est
 * délibérément PAS matérialisée : « j'ai validé cette proposition, je veux la
 * voir dans le compteur » n'admet pas de latence.
 *
 * ELLE N'EST PAS FILTRÉE PAR ÉDITION, et le tableau de bord n'en affiche donc
 * que ce qui n'appartient à aucune : personnes, organisations, doublons.
 */
export function platformOverview(at: number = Date.now()): PlatformOverview {
  const vivantes = allProposals.filter((p) => p.deleted_at === null)
  const depuis = (jours: number): number => at - jours * 86_400_000
  const inscriptionsVivantes = registrations.filter((r) => r.status !== 'cancelled')

  return {
    personnes_total: people.length,
    personnes_actives: people.filter((p) => p.status === 'active').length,
    personnes_anonymisees: people.filter((p) => p.status === 'anonymized').length,
    personnes_verifiees: people.filter((p) => p.email_verified_at !== null).length,
    personnes_avec_compte: new Set(accounts.map((a) => a.person_id)).size,
    inscriptions_aujourdhui: people.filter(
      (p) => utcDay(p.created_at) === new Date(at).toISOString().slice(0, 10),
    ).length,
    inscriptions_7j: people.filter((p) => Date.parse(p.created_at) >= depuis(7)).length,
    inscriptions_30j: people.filter((p) => Date.parse(p.created_at) >= depuis(30)).length,

    organisations_total: organizations.length,
    organisations_actives: organizations.filter((o) => o.status === 'active').length,
    organisations_a_valider: organizations.filter((o) => o.status === 'candidate').length,
    organisations_verifiees: organizations.filter((o) => o.verified_at !== null).length,
    // Le statut est déclaré à part : écrit en ligne, il serait comparé aux
    // seuls statuts que portent les données simulées, et le compilateur
    // refuserait un statut absent du jeu d'essai. Même écueil que la liste des
    // éditions publiques dans `useApi()`.
    organisations_fusionnees: organizations.filter((o) => FUSIONNEES.includes(o.status)).length,
    doublons_a_arbitrer: duplicateCandidates.filter((d) => d.reviewed_at === null).length,

    evenements_total: events.length,
    evenements_en_cours: events.filter((e) => e.status === 'ongoing').length,
    evenements_a_venir: events.filter((e) => e.status === 'announced' && Date.parse(e.starts_at) > at)
      .length,
    // `event.is_call_open()` rejouée par l'utilitaire d'A3 : la règle de la
    // fenêtre d'appel vit à un seul endroit, sinon deux écrans finissent par ne
    // pas s'accorder sur ce qu'« ouvert » veut dire.
    appels_ouverts: callsForProposals.filter((c) => isCallOpen(c, at)).length,

    propositions_total: vivantes.length,
    propositions_a_traiter: vivantes.filter(
      (p) => p.status === 'submitted' || p.status === 'under_review',
    ).length,
    propositions_acceptees: vivantes.filter((p) => p.status === 'accepted').length,
    propositions_rejetees: vivantes.filter((p) => p.status === 'rejected').length,
    propositions_deposees_7j: vivantes.filter(
      (p) => p.submitted_at !== null && Date.parse(p.submitted_at) >= depuis(7),
    ).length,
    revues_en_cours: reviews.filter((r) => r.submitted_at === null).length,

    sessions_publiees: allSessions.filter((s) => s.published_at !== null && s.status !== 'cancelled')
      .length,
    sessions_en_direct: allSessions.filter((s) => s.status === 'live').length,
    sessions_7_prochains_jours: allSessions.filter(
      (s) =>
        (s.status === 'planned' || s.status === 'scheduled') &&
        Date.parse(s.starts_at) >= at &&
        Date.parse(s.starts_at) <= at + 7 * 86_400_000,
    ).length,
    inscriptions_sessions_total: inscriptionsVivantes.length,
    inscriptions_sessions_7j: inscriptionsVivantes.filter(
      (r) => Date.parse(r.created_at) >= depuis(7),
    ).length,
    participations_effectives: registrations.filter((r) => r.joined_at !== null).length,

    // Le module Publications est hors jalon : ses tables ne sont pas simulées.
    articles_publies: 0,
    articles_en_moderation: 0,

    calcule_le: new Date(at).toISOString(),
  }
}

// ---------------------------------------------------------------------------
// analytics.v_operational_health — § 11
// ---------------------------------------------------------------------------

/**
 * L'état de santé, tel que la vue le rendrait.
 *
 * SEULE PROJECTION ÉCRITE À LA MAIN de ce fichier, et la raison en est simple :
 * elle mesure des tables d'infrastructure — `platform.outbox_events`,
 * `platform.jobs`, `engagement.email_messages`, `live.meetings`,
 * `live.provider_webhook_events`, les partitions mensuelles, le journal de
 * rafraîchissement — dont AUCUNE n'est une donnée métier. Les simuler ligne à
 * ligne pour recompter ensuite ce que la vue agrège n'apprendrait rien.
 *
 * LES SEUILS SONT CEUX DU MODÈLE, recopiés du § 11 sans les arrondir : c'est ce
 * qui rend la gravité vérifiable. La règle d'alerte vit à côté de la mesure, pas
 * dans un composant qui dériverait de son côté.
 *
 * L'ÉTAT SIMULÉ EST CELUI D'UN SYSTÈME QUI VA À PEU PRÈS BIEN, avec deux
 * ennuis : des courriels en rebond au-delà du seuil d'attention, et une
 * synchronisation visio en échec. Un jeu tout au vert ne permettrait pas de
 * vérifier que les trois gravités se distinguent ; un jeu tout au rouge ferait
 * de l'écran une alarme permanente, qu'on cesse de lire.
 */
export function operationalHealth(at: number = Date.now()): OperationalHealthRow[] {
  const mesure = new Date(at).toISOString()
  const ilYA = (minutes: number): string => new Date(at - minutes * 60_000).toISOString()

  const lignes: Array<Omit<OperationalHealthRow, 'gravite' | 'mesure_le'>> = [
    {
      code: 'outbox_non_publie',
      libelle: 'Événements de domaine non publiés',
      domaine: 'platform',
      valeur: 12,
      seuil_attention: 100,
      seuil_critique: 1000,
      detail: { plus_ancien: ilYA(4), tentatives_max: 1 },
    },
    {
      code: 'outbox_en_echec',
      libelle: "Événements d'outbox en échec répété",
      domaine: 'platform',
      valeur: 0,
      seuil_attention: 1,
      seuil_critique: 10,
      detail: { derniere_erreur: null },
    },
    {
      code: 'travaux_file_morte',
      libelle: 'Travaux en file morte',
      domaine: 'platform',
      valeur: 2,
      seuil_attention: 1,
      seuil_critique: 25,
      detail: { taches: { 'media.generate_renditions': 2 } },
    },
    {
      code: 'travaux_en_retard',
      libelle: 'Travaux échus non pris en charge',
      domaine: 'platform',
      valeur: 6,
      seuil_attention: 50,
      seuil_critique: 500,
      detail: { plus_ancien: ilYA(11) },
    },
    {
      code: 'travaux_bloques',
      libelle: 'Travaux verrouillés depuis plus de 15 minutes',
      domaine: 'platform',
      valeur: 0,
      seuil_attention: 1,
      seuil_critique: 10,
      detail: { plus_ancien: null, workers: [] },
    },
    {
      code: 'rappels_en_retard',
      libelle: "Rappels programmés non envoyés à l'heure",
      domaine: 'engagement',
      valeur: 0,
      seuil_attention: 1,
      seuil_critique: 50,
      detail: { plus_ancien: null },
    },
    {
      code: 'emails_rebond_7j',
      libelle: 'Courriels en rebond ou signalés (7 jours)',
      domaine: 'engagement',
      valeur: 34,
      seuil_attention: 20,
      seuil_critique: 100,
      detail: { rebonds_durs: 29, plaintes: 5 },
    },
    {
      code: 'emails_en_echec',
      libelle: 'Courriels en échec technique (7 jours)',
      domaine: 'engagement',
      valeur: 1,
      seuil_attention: 5,
      seuil_critique: 50,
      detail: { derniere_erreur: 'SMTP 421 : trop de connexions simultanées' },
    },
    {
      code: 'emails_en_attente',
      libelle: 'Courriels en file depuis plus de 15 minutes',
      domaine: 'engagement',
      valeur: 3,
      seuil_attention: 20,
      seuil_critique: 200,
      detail: { plus_ancien: ilYA(22) },
    },
    {
      code: 'visio_reunions_desynchronisees',
      libelle: 'Réunions non synchronisées chez le fournisseur',
      domaine: 'live',
      valeur: 1,
      seuil_attention: 1,
      seuil_critique: 5,
      detail: { derniere_erreur: 'HTTP 401 : jeton d’API expiré' },
    },
    {
      code: 'visio_inscriptions_desynchronisees',
      libelle: 'Inscriptions visio à rattraper',
      domaine: 'live',
      valeur: 4,
      seuil_attention: 5,
      seuil_critique: 50,
      detail: { abandonnees: 0, derniere_erreur: 'HTTP 429 : quota atteint' },
    },
    {
      code: 'visio_webhooks_en_echec',
      libelle: 'Webhooks fournisseur non traités',
      domaine: 'live',
      valeur: 0,
      seuil_attention: 5,
      seuil_critique: 50,
      detail: { plus_ancien: null },
    },
    {
      code: 'partitions_manquantes',
      libelle: 'Partitions du mois prochain non créées',
      domaine: 'platform',
      valeur: 0,
      seuil_attention: 1,
      seuil_critique: 1,
      detail: { tables: [] },
    },
    {
      // L'ÂGE, EN MINUTES, du dernier rafraîchissement complet réussi. Seul
      // indicateur dont `valeur` n'est pas un compte — la vue le documente.
      code: 'analytique_perimee',
      libelle: 'Minutes depuis le dernier rafraîchissement analytique',
      domaine: 'analytics',
      valeur: 26,
      seuil_attention: 120,
      seuil_critique: 1440,
      detail: { dernier_succes: ilYA(26) },
    },
  ]

  const gravite = (row: (typeof lignes)[number]): HealthSeverity =>
    row.valeur >= row.seuil_critique ? 'critique' : row.valeur >= row.seuil_attention ? 'attention' : 'ok'

  const rang: Record<HealthSeverity, number> = { critique: 0, attention: 1, ok: 2 }

  return lignes
    .map((row) => ({ ...row, gravite: gravite(row), mesure_le: mesure }))
    .sort((a, b) => rang[a.gravite] - rang[b.gravite] || a.code.localeCompare(b.code))
}

/**
 * Dernier `analytics.refresh_all` réussi, déduit de l'indicateur de fraîcheur.
 * Le tableau de bord l'affiche : un chiffre matérialisé donné pour instantané
 * est un chiffre faux.
 */
export function lastAnalyticsRefresh(at: number = Date.now()): string | null {
  const row = operationalHealth(at).find((r) => r.code === 'analytique_perimee')
  const detail = row?.detail as { dernier_succes?: string } | undefined
  return detail?.dernier_succes ?? null
}
