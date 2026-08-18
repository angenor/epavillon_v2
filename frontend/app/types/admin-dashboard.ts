/**
 * Contrat du TABLEAU DE BORD DU BACK-OFFICE (A6) — ce que l'écran demande, ce
 * qu'on lui répond.
 *
 * Ce fichier ne décrit AUCUNE table. Les projections du modèle vivent dans
 * `types/analytics.ts`, les incidents dans `types/live.ts`, la liste des
 * propositions dans `types/views.ts` ; rien ici ne les recopie.
 *
 * TROIS ZONES, ET LEUR ORDRE EST LE SUJET DE L'ÉCRAN :
 *
 *  1. CE QUI DEMANDE UNE ACTION. La seule zone qui coûte quelque chose si on ne
 *     la lit pas. Elle vient en premier, et elle reste lisible VIDE — un
 *     back-office où tout va bien ne doit pas ressembler à un écran cassé.
 *  2. LES CHIFFRES. De la consultation : entonnoir, courbes, répartitions. On y
 *     revient pour comprendre une tendance, pas pour agir dans la minute.
 *  3. LA SANTÉ OPÉRATIONNELLE. Ce qui casse en silence — l'outbox qui ne part
 *     plus, les courriels qui rebondissent. Trois niveaux de gravité, calculés
 *     en base avec leurs seuils.
 *
 * TOUT EST FILTRÉ PAR L'ÉVÉNEMENT SÉLECTIONNÉ, sans exception. Règle métier
 * n° 8 : un administrateur peut n'avoir accès qu'à une seule édition, et rien de
 * ce contrat ne doit lui laisser deviner l'existence des autres. Le filtrage
 * définitif appartient à l'API (`identity.administered_events()`) — ce contrat
 * décrit le comportement attendu de l'écran, pas un contrôle de sécurité.
 *
 * POURQUOI UNE COMPOSITION ET NON SIX LECTURES. L'écran ouvre sur cinq familles
 * d'alerte, trois projections analytiques et une vue de santé. Lues séparément,
 * elles produisent neuf allers-retours au chargement d'une page qu'on consulte
 * vingt fois par jour, et neuf instants de mesure différents dans un même écran.
 * La composition appartient donc à l'API (prompt B7).
 */

import type { CallForProposals } from './event/call'
import type { EventEdition } from './event/edition'
import type { OperationalHealthRow, ProposalFunnelRow } from './analytics'
import type { IncidentSeverity, IncidentScope } from './live'
import type { ColorHex, I18nText, IsoDateTime, IsoDate, TimeZoneName, Uuid } from './shared'

// ---------------------------------------------------------------------------
// Zone 1 — ce qui demande une action
// ---------------------------------------------------------------------------

/**
 * LES CINQ FAMILLES D'ALERTE, et le critère qui les réunit : chacune se règle
 * dans un écran du back-office, par quelqu'un de l'équipe, aujourd'hui.
 *
 * Ce qui n'y figure pas est aussi délibéré que ce qui y figure. Un dossier
 * déposé la veille n'est pas une alerte — c'est le fonctionnement normal ; il
 * n'en devient une qu'à l'approche de l'échéance et sans évaluation. Une liste
 * où l'on trouve ce qui n'appelle rien cesse d'être lue, et c'est alors la ligne
 * qui comptait qu'on rate.
 */
export type AdminActionKind =
  /** Dossiers déposés sans aucune revue, l'échéance approchant. */
  | 'proposals_unreviewed'
  /** Affectations de revue dont l'échéance est dépassée — `mv_reviewer_workload`. */
  | 'reviews_overdue'
  /** Paires d'organisations présumées identiques, non arbitrées. */
  | 'organization_duplicates'
  /** Chevauchements détectés par `detect_conflicts()`. JAMAIS bloquants. */
  | 'schedule_conflicts'
  /** Messages d'incident publiés et actifs à cet instant. */
  | 'active_incidents'

/**
 * Gravité D'AFFICHAGE d'une ligne d'action — deux niveaux, pas trois.
 *
 * `high` : ce qui a une échéance dépassée ou en cours de l'être, et ce qui est
 * visible du public (un incident actif l'est). `medium` : ce qui attend sans
 * date. Il n'y a délibérément pas de `low` — une ligne qui n'appelle rien n'a
 * pas sa place dans ce bloc, elle appartient aux chiffres.
 *
 * À NE PAS CONFONDRE avec `HealthSeverity` (zone 3), qui vient de la base avec
 * ses seuils, ni avec `IncidentSeverity`, qui est une donnée de l'incident.
 */
export type AdminActionSeverity = 'high' | 'medium'

/**
 * Un exemple nommé sous une ligne d'action. Trois au plus.
 *
 * POURQUOI DES EXEMPLES ET PAS SEULEMENT UN NOMBRE. « 7 revues en retard » ne
 * dit pas par où commencer ; « 7 revues en retard — Lemoine (3), Ben Amor (2) »
 * le dit. Au-delà de trois, la ligne cesse d'être un résumé et il faut ouvrir
 * l'écran concerné : c'est précisément ce que le lien propose.
 */
export interface AdminActionExample {
  /** Ce dont il s'agit : nom, titre de dossier, paire d'organisations. */
  label: string
  /** Précision courte : numéro de dossier, décompte, salle. Facultative. */
  hint: string | null
  /** Destination propre à l'exemple, relative et non localisée. */
  target: string | null
}

/**
 * UNE LIGNE DU BLOC D'ACTIONS — une famille, son décompte, ses exemples, son
 * écran.
 *
 * UNE LIGNE PAR FAMILLE, jamais une par élément. Quarante dossiers non évalués
 * produiraient quarante lignes, et le bloc censé se lire d'un coup d'œil
 * deviendrait la liste des propositions — qui existe déjà, avec ses filtres
 * (A7). Le décompte et trois exemples suffisent à décider ; le reste est un clic.
 */
export interface AdminAction {
  kind: AdminActionKind
  severity: AdminActionSeverity
  /** Éléments concernés. Une ligne à zéro n'est jamais émise. */
  count: number
  /** Échéance qui rend la ligne urgente ; nulle quand il n'y en a pas. */
  due_at: IsoDateTime | null
  examples: AdminActionExample[]
  /** L'écran qui règle l'affaire. Chemin relatif, non localisé. */
  target: string
}

// ---------------------------------------------------------------------------
// Zone 2 — les chiffres
// ---------------------------------------------------------------------------

/**
 * Un point de courbe : un jour, une valeur, un cumul.
 *
 * LE JOUR EST UNE DATE CIVILE (`AAAA-MM-JJ`), découpée en UTC par la projection.
 * Ce n'est pas un instant : le convertir en `Date` puis le reformater dans un
 * fuseau le décalerait d'un jour la moitié de l'année. Les composants de courbe
 * l'affichent tel quel, formaté par `Intl` en date seule.
 */
export interface TrendPoint {
  jour: IsoDate
  valeur: number
  cumul: number
}

/**
 * Une part de répartition — un pays, une thématique.
 *
 * `label` est un texte MULTILINGUE venu de la base (`reference.countries.name`,
 * `reference.taxonomy_terms.label`) : il se résout à l'affichage et ne passe
 * JAMAIS par un fichier i18n. `color` vient de `taxonomy_terms.color_hex` pour
 * les thématiques — figer ces couleurs dans la feuille de style est le défaut
 * n° 1 de la v1.
 */
export interface BreakdownSlice {
  /** Clé stable : code ISO du pays, code du terme de taxonomie. */
  key: string
  label: I18nText
  color: ColorHex | null
  count: number
  /** Part du total, entre 0 et 1. Calculée à la source pour ne pas diverger. */
  share: number
}

/**
 * LES CHIFFRES DE L'ÉDITION SÉLECTIONNÉE.
 *
 * `funnel` peut être nul : une édition sans appel ni dépôt n'a pas d'entonnoir,
 * et un entonnoir à zéro partout serait un graphique qui ment sur sa propre
 * existence.
 */
export interface DashboardFigures {
  /** `mv_proposal_funnel`, la ligne de l'appel de l'édition. */
  funnel: ProposalFunnelRow | null
  /** `mv_daily_submissions` — la courbe des dépôts. */
  submissions: TrendPoint[]
  /** `mv_daily_registrations` — la courbe des inscriptions aux activités. */
  registrations: TrendPoint[]
  /**
   * L'ÉCHÉANCE À MARQUER SUR LA COURBE DES DÉPÔTS — `event.effective_deadline()`,
   * donc la prolongation si elle existe. Sans ce repère, l'effet de dernière
   * minute (60 % des dépôts sur les 48 dernières heures, mesuré en v1) est
   * illisible : on voit un pic sans savoir devant quoi il se produit.
   */
  deadline: IsoDateTime | null
  /** Date d'ouverture de l'appel, second repère de la même courbe. */
  call_opens_at: IsoDateTime | null
  /** Organisations porteuses, par pays. Dix parts au plus, plus « autres ». */
  by_country: BreakdownSlice[]
  /** Dossiers par thématique. Un dossier en porte plusieurs : la somme dépasse le total. */
  by_theme: BreakdownSlice[]
  /**
   * ÂGE DES PROJECTIONS MATÉRIALISÉES — dernier `analytics.refresh_all` réussi.
   * Nul quand aucun rafraîchissement n'a jamais abouti. Affiché sans détour :
   * un chiffre matérialisé présenté comme instantané est un chiffre faux.
   */
  refreshed_at: IsoDateTime | null
}

// ---------------------------------------------------------------------------
// Zone 3 — la santé opérationnelle
// ---------------------------------------------------------------------------

/**
 * Un incident actif de l'édition — `live.active_incidents_for_event()`, ajoutée
 * au modèle le 17/08 pour cet écran.
 *
 * Elle DESCEND la hiérarchie (édition → journées → séances → organisations qui
 * y animent), là où `live.active_incidents(session)` la remonte. Sans elle, le
 * tableau de bord et l'écran A13 recomposeraient chacun ce balayage.
 */
export interface EventIncident {
  incident_id: Uuid
  scope: IncidentScope
  severity: IncidentSeverity
  kind_code: string
  title: I18nText | null
  message: I18nText
  /** Cible résolue par la fonction : nom de séance, de journée, d'organisation. */
  target_label: string | null
  display_from: IsoDateTime
  display_until: IsoDateTime | null
}

// ---------------------------------------------------------------------------
// La réponse entière
// ---------------------------------------------------------------------------

/**
 * TOUT L'ÉCRAN EN UNE RÉPONSE, pour l'édition sélectionnée.
 *
 * `timezone` est celui de l'ÉDITION, et toute date affichée le porte — un
 * chevauchement de créneaux ne se lit pas dans le fuseau du navigateur de la
 * personne qui arbitre.
 */
export interface AdminDashboard {
  edition: EventEdition
  timezone: TimeZoneName
  /** L'appel de l'édition, s'il y en a un. Zéro ou un, jamais deux. */
  call: CallForProposals | null
  actions: AdminAction[]
  figures: DashboardFigures
  /** `v_operational_health` — une ligne par indicateur, avec ses seuils. */
  health: OperationalHealthRow[]
  incidents: EventIncident[]
}
