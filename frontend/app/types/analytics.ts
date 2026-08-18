/**
 * Schéma `analytics` — les projections que le TABLEAU DE BORD DU BACK-OFFICE
 * consomme. Dérivé de `docs/database/130_analytics.sql`.
 *
 * DEUX NATURES DE LECTURE, ET LA DIFFÉRENCE COMPTE À L'ÉCRAN :
 *
 *  · Les `mv_*` sont MATÉRIALISÉES, rafraîchies par le worker
 *    (`analytics.refresh_all`, tâche différée dans `platform.jobs`). Elles ne
 *    sont donc JAMAIS à la seconde, et l'écran doit le dire — d'où l'indicateur
 *    `analytique_perimee` de `v_operational_health`, qui mesure l'âge du dernier
 *    rafraîchissement réussi. Afficher un chiffre matérialisé sans son âge, c'est
 *    transformer « le tableau de bord affiche les chiffres d'hier » en
 *    signalement invérifiable.
 *  · `v_platform_overview` et `v_operational_health` sont des vues ORDINAIRES,
 *    exactes à la seconde. Le choix est délibéré en base : ces compteurs sont lus
 *    par une poignée d'administrateurs, et « j'ai validé cette proposition, je
 *    veux la voir dans le compteur » n'admet pas de latence.
 *
 * LES NOMS DE COLONNES SONT EN FRANÇAIS, comme dans le SQL du module. On ne les
 * traduit pas : renommer en chemin est le plus sûr moyen de ne plus savoir quelle
 * colonne on lit. Les libellés affichés, eux, viennent des fichiers i18n.
 *
 * CE FICHIER NE COUVRE PAS TOUT LE MODULE. `mv_organization_scorecard`,
 * `mv_session_attendance`, `mv_content_popularity` et `analytics.page_views`
 * viendront avec leurs écrans (A11, bilan de COP, mesure d'audience), comme
 * `types/live.ts` ne couvre que les incidents.
 */

import type { CallId, EventId, IsoDateTime, Numeric, PersonId, Uuid } from './shared'

// ---------------------------------------------------------------------------
// analytics.v_platform_overview — les compteurs de la page d'accueil
// ---------------------------------------------------------------------------

/**
 * Ligne UNIQUE de `analytics.v_platform_overview` — § 10.
 *
 * ELLE N'EST PAS FILTRÉE PAR ÉVÉNEMENT. La vue compte la plateforme entière,
 * ce qui la rend inutilisable telle quelle pour un administrateur restreint à
 * une seule édition (règle métier n° 8). Le tableau de bord n'en affiche donc
 * que les compteurs qui n'appartiennent à aucune édition — personnes,
 * organisations, doublons — et tire tout le reste des projections par événement.
 * Obligation d'API inscrite au prompt B7.
 */
export interface PlatformOverview {
  personnes_total: number
  personnes_actives: number
  personnes_anonymisees: number
  personnes_verifiees: number
  personnes_avec_compte: number
  inscriptions_aujourdhui: number
  inscriptions_7j: number
  inscriptions_30j: number

  organisations_total: number
  organisations_actives: number
  organisations_a_valider: number
  organisations_verifiees: number
  organisations_fusionnees: number
  /** `org.duplicate_candidates` non arbitrés — une ligne du bloc d'actions. */
  doublons_a_arbitrer: number

  evenements_total: number
  evenements_en_cours: number
  evenements_a_venir: number
  appels_ouverts: number

  propositions_total: number
  propositions_a_traiter: number
  propositions_acceptees: number
  propositions_rejetees: number
  propositions_deposees_7j: number
  revues_en_cours: number

  sessions_publiees: number
  sessions_en_direct: number
  sessions_7_prochains_jours: number
  inscriptions_sessions_total: number
  inscriptions_sessions_7j: number
  participations_effectives: number

  articles_publies: number
  articles_en_moderation: number

  calcule_le: IsoDateTime
}

// ---------------------------------------------------------------------------
// analytics.v_operational_health — la santé opérationnelle
// ---------------------------------------------------------------------------

/**
 * Gravité calculée EN BASE, par comparaison de la valeur à ses deux seuils.
 *
 * Les trois niveaux sont ceux de la vue et rien d'autre : l'écran ne recalcule
 * pas de gravité, il rend celle qu'on lui donne. Un seuil vit à côté de sa
 * mesure, pas dans un composant qui dériverait de son côté.
 */
export type HealthSeverity = 'ok' | 'attention' | 'critique'

/** Domaine émetteur — sert à grouper, jamais à décider d'une couleur. */
export type HealthDomain = 'platform' | 'engagement' | 'live' | 'analytics'

/**
 * Une ligne de `analytics.v_operational_health` — § 11.
 *
 * `valeur` est un COMPTE pour tous les indicateurs sauf `analytique_perimee`,
 * qui porte un âge EN MINUTES. C'est la seule exception, elle est documentée en
 * base, et elle explique pourquoi l'écran ne peut pas afficher « N » suivi d'un
 * mot unique pour toute la liste.
 */
export interface OperationalHealthRow {
  code: string
  /** Libellé français porté par la vue. Voir la note de `AdminDashboard`. */
  libelle: string
  domaine: HealthDomain
  valeur: number
  seuil_attention: number
  seuil_critique: number
  gravite: HealthSeverity
  /** Détail libre — plus ancien élément, dernière erreur, ventilation. */
  detail: Record<string, unknown>
  mesure_le: IsoDateTime
}

// ---------------------------------------------------------------------------
// analytics.mv_proposal_funnel — l'entonnoir des propositions
// ---------------------------------------------------------------------------

/**
 * Une ligne de `analytics.mv_proposal_funnel` — § 3. Grain : (événement, appel).
 *
 * LE GRAIN EST L'APPEL, PAS L'ÉVÉNEMENT, et c'est structurant : la v2 admet
 * plusieurs appels par édition (journée jeunesse, journée finance), dont la
 * sélectivité diffère considérablement. Un entonnoir agrégé au seul niveau de
 * l'édition masquerait ces écarts. Le jalon en cours n'en pose qu'un seul par
 * édition — la lecture reste la même quand il y en aura deux.
 */
export interface ProposalFunnelRow {
  event_id: EventId
  /** Clé non nulle (UUID nul = propositions hors appel), condition du rafraîchissement concurrent. */
  cle_appel: Uuid
  call_id: CallId | null
  evenement: string
  edition_year: number
  statut_evenement: string
  code_appel: string | null
  appel: string | null
  statut_appel: string | null
  appel_ouvre_le: IsoDateTime | null
  /** `COALESCE(extended_until, closes_at)` — l'échéance qui fait foi. */
  appel_ferme_le: IsoDateTime | null
  required_reviews: number | null

  total: number
  brouillons: number
  deposees: number
  en_attente_affectation: number
  en_revue: number
  modifications_demandees: number
  acceptees: number
  rejetees: number
  retirees: number
  annulees: number
  decidees: number
  en_instance: number

  /** Sélectivité du comité : acceptées / (acceptées + rejetées). */
  taux_acceptation: Numeric | null
  /** Rendement de l'appel : acceptées / déposées, retraits compris. */
  taux_acceptation_sur_depots: Numeric | null
  organisations_distinctes: number
  note_moyenne: Numeric | null
  /** Médiane et non moyenne : deux dossiers arbitrés très tard fausseraient la moyenne. */
  delai_median_decision_heures: Numeric | null
  premier_depot: IsoDateTime | null
  dernier_depot: IsoDateTime | null
  sessions_programmees: number
}

// ---------------------------------------------------------------------------
// analytics.mv_daily_submissions — la courbe des dépôts
// ---------------------------------------------------------------------------

/**
 * Un jour de `analytics.mv_daily_submissions` — § 4.
 *
 * LA SÉRIE EST CONTINUE, jours à zéro compris, et c'est garanti EN BASE. Une
 * courbe dont les jours vides sont absents est illisible : le frontend doit
 * alors reconstituer les trous, chaque écran le fait à sa manière, et deux
 * graphiques de la même donnée finissent par diverger. Un composant de courbe
 * ne rebouche donc AUCUN trou — s'il en trouve un, c'est la requête qui est en
 * cause.
 *
 * Les jours sont découpés en UTC par la vue, explicitement : un agrégat ne
 * change pas de valeur selon le fuseau de la session qui le calcule.
 */
export interface DailySubmissionRow {
  /** Jour au format `AAAA-MM-JJ`, découpé en UTC. */
  jour: string
  event_id: EventId
  evenement: string
  edition_year: number
  soumissions: number
  organisations_distinctes: number
  acceptations: number
  rejets: number
  soumissions_cumulees: number
  moyenne_mobile_7j: Numeric | null
}

// ---------------------------------------------------------------------------
// analytics.mv_daily_signups — la courbe des inscriptions de personnes
// ---------------------------------------------------------------------------

/**
 * Un jour de `analytics.mv_daily_signups` — § 2. Grain : (jour, pays).
 *
 * La ligne dont `cle_pays` vaut l'UUID nul porte le TOTAL du jour et existe
 * pour tous les jours de la série ; les lignes par pays n'existent que les jours
 * où ce pays a enregistré au moins une inscription.
 */
export interface DailySignupRow {
  jour: string
  /** UUID nul = toutes origines confondues. */
  cle_pays: Uuid
  country_id: Uuid | null
  pays_iso3: string | null
  pays_nom: string
  statut_oif: string
  inscriptions: number
  inscriptions_verifiees: number
  inscriptions_avec_compte: number
  inscriptions_cumulees: number
  moyenne_mobile_7j: Numeric | null
}

// ---------------------------------------------------------------------------
// analytics.mv_reviewer_workload — la charge du comité
// ---------------------------------------------------------------------------

/**
 * Une ligne de `analytics.mv_reviewer_workload` — § 7. Grain : (révisionniste,
 * événement).
 *
 * `revues_en_retard` est LA colonne que le bloc d'actions du tableau de bord
 * remonte : affectations dont l'échéance est dépassée sans revue soumise,
 * déports exclus. Le calcul appartient à la base — le refaire à partir des
 * affectations produirait un second retard, différent du premier.
 */
export interface ReviewerWorkloadRow {
  reviewer_id: PersonId
  event_id: EventId
  revisionniste: string
  evenement: string
  edition_year: number
  propositions_assignees: number
  deports: number
  revues_soumises: number
  revues_en_cours: number
  revues_en_retard: number
  revues_restantes: number
  taux_completion: Numeric | null
  note_moyenne_attribuee: Numeric | null
  note_moyenne_evenement: Numeric | null
  ecart_a_la_moyenne: Numeric | null
  /** Sévérité relative : au moins un point d'écart sur 20 face à la moyenne. */
  profil_notation: 'severe' | 'genereux' | 'dans_la_moyenne' | 'indetermine'
  prochaine_echeance: IsoDateTime | null
  derniere_revue_le: IsoDateTime | null
}

// ---------------------------------------------------------------------------
// analytics.mv_daily_registrations — la courbe des inscriptions aux activités
// ---------------------------------------------------------------------------

/**
 * Un jour de `analytics.mv_daily_registrations` — § 4 bis, AJOUTÉE AU MODÈLE LE
 * 17/08 POUR CET ÉCRAN.
 *
 * NE PAS LA CONFONDRE AVEC `mv_daily_signups`. Les deux disent « inscriptions »
 * et recouvrent deux faits sans rapport : `mv_daily_signups` compte des
 * CRÉATIONS DE COMPTE sur toute la plateforme, celle-ci compte des INSCRIPTIONS
 * À UNE ACTIVITÉ d'une édition donnée. La confusion n'est pas théorique — c'est
 * elle qui a rendu la projection nécessaire : `mv_daily_signups` ne porte pas
 * `event_id` et ne peut donc RIEN montrer à un administrateur détaché sur une
 * seule édition.
 */
export interface DailyRegistrationRow {
  jour: string
  event_id: EventId
  evenement: string
  edition_year: number
  /** Inscriptions non annulées créées ce jour-là, liste d'attente comprise. */
  inscriptions: number
  liste_attente: number
  /** Comptées à part, jamais soustraites des inscriptions du jour. */
  annulations: number
  presents: number
  /** Une personne inscrite à trois activités le même jour ne compte qu'une fois. */
  personnes_distinctes: number
  inscriptions_cumulees: number
  moyenne_mobile_7j: Numeric | null
}
