-- =============================================================================
-- ePavillon v2 — 130_analytics.sql
-- Module Analytique : mesure d'audience, projections de lecture et tableaux de
-- bord du back-office.
--
-- Dépend de : 000, 010, 020, 030, 040, 050, 060, 070, 075, 080, 090, 100, 110, 120
--
-- DÉCISION STRUCTURANTE N°1 — LES COMPTEURS SONT MATÉRIALISÉS, PAS RECALCULÉS
-- En v1, chaque écran d'administration recalculait ses compteurs par des
-- requêtes agrégées sur les tables métier : « nombre total d'utilisateurs,
-- inscriptions par jour », « nombre de soumissions par jour, activités
-- approuvées, rejetées », « organisations : leurs activités, leurs membres,
-- nombre d'activités validées, ratio ». Ces balayages s'exécutaient en
-- concurrence avec le trafic public — et pendant une COP, le trafic public est
-- précisément à son maximum au moment où l'équipe consulte ses tableaux de bord.
-- Ouvrir la page « statistiques » ralentissait la page « programmation ».
--
-- En v2, les agrégats sont MATÉRIALISÉS et rafraîchis par le worker
-- (platform.jobs, tâche `analytics.refresh_all`) : les écrans lisent des tables
-- déjà calculées, indexées, dont le coût de lecture est constant. Le
-- rafraîchissement s'effectue en `CONCURRENTLY`, ce qui ne bloque aucun lecteur
-- — au prix d'une contrainte à ne jamais oublier : PostgreSQL exige un index
-- UNIQUE sur chaque vue matérialisée pour pouvoir la rafraîchir ainsi. Chaque
-- `analytics.mv_*` de ce fichier porte donc son index `ux_*`, sur des colonnes
-- NON NULLES et sans expression (les deux conditions du rafraîchissement
-- concurrent). Les clés naturellement nullables (appel facultatif, pays inconnu,
-- contenu sans identifiant) sont matérialisées en colonne `cle_*` avec un UUID
-- sentinelle, jamais laissées à NULL.
--
-- DÉCISION STRUCTURANTE N°2 — AUCUNE CLÉ ÉTRANGÈRE SORTANTE
-- Ce module ne fait que LIRE. Il ne déclare aucune FK vers les autres schémas,
-- n'attache aucun trigger aux tables métier et n'écrit que dans ses propres
-- tables (`analytics.page_views`, `analytics.refresh_log`). C'est ce qui permet
-- de le déporter un jour sur un réplica en lecture seule — ou sur une base
-- d'analyse distincte — sans rien changer : il n'y a pas un seul lien
-- d'intégrité à couper, donc pas une seule écriture à réorienter. Les
-- énumérations sont converties en texte dans les projections pour la même
-- raison : la sortie reste lisible et transportable hors de cette base.
--
-- DÉCISION STRUCTURANTE N°3 — MESURE D'AUDIENCE SANS DONNÉE PERSONNELLE
-- `analytics.page_views` compte des consultations, pas des personnes : ni
-- adresse IP, ni identifiant de compte, ni agent utilisateur brut. Seule une
-- empreinte de visiteur hachée, salée avec un secret tournant, y figure. Voir
-- le commentaire de la section 1 : c'est du respect du RGPD par conception, pas
-- une anonymisation appliquée après coup.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Mesure d'audience
--
-- Table de faits partitionnée par mois, comme platform.audit_log et
-- engagement.email_messages : volumétrie de journal, purge réglementaire par
-- DROP PARTITION plutôt que par DELETE massif, et écriture qui ne fragmente
-- jamais les index métier.
--
-- RGPD PAR CONCEPTION — ce qui n'est PAS stocké ici est aussi important que ce
-- qui l'est :
--   * pas d'adresse IP, même tronquée : une IP est une donnée à caractère
--     personnel, et la conserver imposerait base légale, durée de conservation
--     et droit d'accès sur une table de plusieurs millions de lignes ;
--   * pas de `person_id` : le lien entre une personne et les pages qu'elle
--     consulte n'a aucun usage métier ici. Seul un drapeau `is_authenticated`
--     distingue le public du connecté ;
--   * `visitor_hash` est une empreinte NON RÉVERSIBLE (SHA-256 salé puis
--     tronqué à 128 bits) calculée par l'API. Le sel tourne chaque jour et vit
--     dans le coffre, jamais en base : deux visites du même navigateur à deux
--     jours d'intervalle produisent deux empreintes sans lien calculable. On
--     peut donc compter des visiteurs uniques sur une journée sans être capable
--     de suivre quiconque dans le temps.
-- La table ne peut ainsi pas devenir, par dérive d'usage, un journal de
-- navigation nominatif.
-- -----------------------------------------------------------------------------
CREATE TABLE analytics.page_views (
    id               uuid        NOT NULL DEFAULT platform.uuid_v7(),
    viewed_at        timestamptz NOT NULL DEFAULT now(),

    -- Contenu consulté, désigné comme dans reference.entity_terms :
    -- (schéma, table, identifiant). Aucune FK — le module reste autonome et la
    -- mesure survit à la suppression du contenu mesuré.
    content_schema   text        NOT NULL,
    content_table    text        NOT NULL,
    content_id       uuid,
    -- Chemin normalisé (sans paramètres de requête) : couvre aussi les pages
    -- sans entité (accueil, recherche, formulaires).
    path             text        NOT NULL,
    locale           text,

    -- Empreinte de visiteur : voir analytics.visitor_fingerprint().
    visitor_hash     bytea       CHECK (visitor_hash IS NULL OR octet_length(visitor_hash) = 16),
    -- Empreinte de visite (onglet) : sert à mesurer une profondeur de parcours
    -- sans jamais rapprocher deux visites entre elles.
    session_hash     bytea       CHECK (session_hash IS NULL OR octet_length(session_hash) = 16),

    -- Provenance : nom d'hôte du référent uniquement (jamais l'URL complète,
    -- qui peut contenir des identifiants de session ou des termes de recherche).
    referrer_host    text,
    utm_source       text,
    utm_medium       text,
    utm_campaign     text,

    -- Catégorie d'appareil déduite côté API. L'agent utilisateur brut, qui est
    -- un quasi-identifiant, n'est pas conservé.
    device_kind      text        CHECK (device_kind IN ('desktop', 'mobile', 'tablet', 'bot', 'other')),
    -- Pays déduit à la volée puis oublié : seul le code ISO est conservé, ce qui
    -- ne permet aucune ré-identification.
    country_iso2     char(2),
    is_authenticated boolean     NOT NULL DEFAULT false,
    -- Temps passé mesuré côté navigateur, en millisecondes.
    duration_ms      integer     CHECK (duration_ms IS NULL OR duration_ms >= 0),

    PRIMARY KEY (viewed_at, id)
) PARTITION BY RANGE (viewed_at);

-- Partition « fourre-tout » : aucune écriture de mesure ne peut échouer si le
-- worker de maintenance a pris du retard. Une page vue perdue est bénigne, une
-- requête publique en erreur ne l'est pas.
CREATE TABLE analytics.page_views_default PARTITION OF analytics.page_views DEFAULT;

CREATE INDEX ix_page_views_content
    ON analytics.page_views (content_schema, content_table, content_id, viewed_at DESC)
    WHERE content_id IS NOT NULL;
CREATE INDEX ix_page_views_path     ON analytics.page_views (path, viewed_at DESC);
CREATE INDEX ix_page_views_campaign ON analytics.page_views (utm_campaign, viewed_at DESC)
    WHERE utm_campaign IS NOT NULL;

COMMENT ON TABLE analytics.page_views IS
    'Mesure d''audience partitionnée par mois, sans donnée personnelle directe : ni IP, ni identifiant de compte, empreinte de visiteur hachée et salée quotidiennement.';
COMMENT ON COLUMN analytics.page_views.visitor_hash IS
    'Empreinte non réversible (SHA-256 salé, tronqué à 128 bits). Le sel tourne chaque jour : aucun suivi possible d''un jour sur l''autre.';
COMMENT ON COLUMN analytics.page_views.referrer_host IS
    'Nom d''hôte du référent seulement. L''URL complète est écartée : elle peut porter des identifiants ou des termes de recherche.';
COMMENT ON COLUMN analytics.page_views.content_id IS
    'Identifiant du contenu consulté, sans clé étrangère : la mesure survit à la suppression du contenu et à l''extraction du module.';

-- Amorce des partitions du trimestre courant ; le worker appellera ensuite
-- platform.ensure_month_partition() mois par mois (cf. v_operational_health,
-- indicateur `partitions_manquantes`).
DO $$
DECLARE
    v_month date;
BEGIN
    FOR v_month IN
        SELECT generate_series(date_trunc('month', now()),
                               date_trunc('month', now()) + interval '2 months',
                               interval '1 month')::date
    LOOP
        PERFORM platform.ensure_month_partition('analytics', 'page_views', v_month);
    END LOOP;
END
$$;

-- Calcul de l'empreinte de visiteur. Le sel du jour est fourni par l'API depuis
-- le coffre : la base ne le détient pas et ne peut donc pas reconstituer les
-- empreintes, même avec un accès complet aux données.
CREATE OR REPLACE FUNCTION analytics.visitor_fingerprint(p_raw text, p_daily_salt text)
RETURNS bytea
LANGUAGE sql
IMMUTABLE
STRICT
PARALLEL SAFE
AS $$
    -- Troncature à 16 octets : suffisante pour distinguer des visiteurs sur une
    -- journée, et seconde barrière à l'inversion par force brute.
    SELECT substring(public.digest(p_daily_salt || ':' || p_raw, 'sha256') FROM 1 FOR 16);
$$;

COMMENT ON FUNCTION analytics.visitor_fingerprint(text, text) IS
    'Empreinte de visiteur non réversible. Le sel tourne quotidiennement et vit dans le coffre : la base ne peut pas ré-identifier.';

-- -----------------------------------------------------------------------------
-- 2. Inscriptions de personnes par jour
--
-- Écran « nombre total d'utilisateurs, inscriptions par jour » du back-office.
--
-- La série est CONTINUE : `generate_series` produit une ligne pour chaque jour
-- de la période, y compris les jours sans aucune inscription. Une courbe dont
-- les jours à zéro sont absents est illisible — le frontend est obligé de
-- reconstituer les trous, chaque écran le fait à sa manière, et deux graphiques
-- de la même donnée finissent par diverger. La continuité est donc garantie en
-- base, une fois pour toutes.
--
-- Grain : une ligne par (jour, pays). La ligne dont `cle_pays` vaut l'UUID nul
-- porte le TOTAL du jour et est présente pour tous les jours de la période ;
-- les lignes par pays n'existent que les jours où ce pays a enregistré au moins
-- une inscription. Un cumul sur une série creuse reste exact, la courbe
-- cumulée n'en souffre pas.
--
-- Les dates sont découpées en UTC, explicitement : un agrégat ne doit pas
-- changer de valeur selon le fuseau de la session qui le rafraîchit.
-- -----------------------------------------------------------------------------
CREATE MATERIALIZED VIEW analytics.mv_daily_signups AS
WITH bornes AS (
    SELECT
        COALESCE(min(p.created_at AT TIME ZONE 'UTC')::date, CURRENT_DATE) AS premier_jour,
        GREATEST(COALESCE(max(p.created_at AT TIME ZONE 'UTC')::date, CURRENT_DATE), CURRENT_DATE) AS dernier_jour
    FROM identity.people p
),
calendrier AS (
    SELECT g::date AS jour
    FROM bornes b
    CROSS JOIN LATERAL generate_series(
        -- Borne de sécurité : cinq ans de série au plus, pour qu'une date
        -- aberrante importée de la v1 ne produise pas un million de lignes.
        GREATEST(b.premier_jour, b.dernier_jour - 1826)::timestamp,
        b.dernier_jour::timestamp,
        interval '1 day'
    ) AS g
),
personnes AS (
    SELECT
        (p.created_at AT TIME ZONE 'UTC')::date       AS jour,
        p.country_id,
        c.iso3::text                                  AS pays_iso3,
        platform.t(c.name)                            AS pays_nom,
        COALESCE(c.oif_status::text, 'none')          AS statut_oif,
        p.email_verified_at,
        p.status,
        EXISTS (SELECT 1 FROM identity.accounts a WHERE a.person_id = p.id) AS a_un_compte
    FROM identity.people p
    LEFT JOIN reference.countries c ON c.id = p.country_id
),
totaux AS (
    SELECT
        cal.jour,
        '00000000-0000-0000-0000-000000000000'::uuid AS cle_pays,
        NULL::uuid  AS country_id,
        NULL::text  AS pays_iso3,
        'Tous pays'::text AS pays_nom,
        'all'::text AS statut_oif,
        count(pe.jour)                                                            AS inscriptions,
        count(*) FILTER (WHERE pe.email_verified_at IS NOT NULL)                  AS inscriptions_verifiees,
        count(*) FILTER (WHERE pe.a_un_compte)                                    AS inscriptions_avec_compte,
        count(*) FILTER (WHERE pe.country_id IS NULL)                             AS inscriptions_sans_pays,
        count(*) FILTER (WHERE pe.statut_oif = 'member')                          AS inscriptions_oif_membre,
        count(*) FILTER (WHERE pe.statut_oif = 'associate')                       AS inscriptions_oif_associe,
        count(*) FILTER (WHERE pe.statut_oif = 'observer')                        AS inscriptions_oif_observateur,
        count(*) FILTER (WHERE pe.jour IS NOT NULL AND pe.statut_oif = 'none')    AS inscriptions_hors_oif,
        count(*) FILTER (WHERE pe.status = 'anonymized')                          AS inscriptions_anonymisees
    FROM calendrier cal
    LEFT JOIN personnes pe ON pe.jour = cal.jour
    GROUP BY cal.jour
),
par_pays AS (
    SELECT
        pe.jour,
        pe.country_id AS cle_pays,
        pe.country_id,
        pe.pays_iso3,
        pe.pays_nom,
        pe.statut_oif,
        count(*)                                                   AS inscriptions,
        count(*) FILTER (WHERE pe.email_verified_at IS NOT NULL)   AS inscriptions_verifiees,
        count(*) FILTER (WHERE pe.a_un_compte)                     AS inscriptions_avec_compte,
        0::bigint                                                  AS inscriptions_sans_pays,
        count(*) FILTER (WHERE pe.statut_oif = 'member')           AS inscriptions_oif_membre,
        count(*) FILTER (WHERE pe.statut_oif = 'associate')        AS inscriptions_oif_associe,
        count(*) FILTER (WHERE pe.statut_oif = 'observer')         AS inscriptions_oif_observateur,
        count(*) FILTER (WHERE pe.statut_oif = 'none')             AS inscriptions_hors_oif,
        count(*) FILTER (WHERE pe.status = 'anonymized')           AS inscriptions_anonymisees
    FROM personnes pe
    WHERE pe.country_id IS NOT NULL
    GROUP BY pe.jour, pe.country_id, pe.pays_iso3, pe.pays_nom, pe.statut_oif
),
assemble AS (
    SELECT * FROM totaux
    UNION ALL
    SELECT * FROM par_pays
)
SELECT
    a.jour,
    a.cle_pays,
    a.country_id,
    a.pays_iso3,
    a.pays_nom,
    a.statut_oif,
    a.inscriptions,
    a.inscriptions_verifiees,
    a.inscriptions_avec_compte,
    a.inscriptions_sans_pays,
    a.inscriptions_oif_membre,
    a.inscriptions_oif_associe,
    a.inscriptions_oif_observateur,
    a.inscriptions_hors_oif,
    a.inscriptions_anonymisees,
    sum(a.inscriptions) OVER (PARTITION BY a.cle_pays ORDER BY a.jour
                              ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS inscriptions_cumulees,
    -- Moyenne glissante sur sept jours : lisse l'effet « ouverture des
    -- inscriptions un mardi » qui rend la courbe brute inexploitable.
    round(avg(a.inscriptions) OVER (PARTITION BY a.cle_pays ORDER BY a.jour
                                    ROWS BETWEEN 6 PRECEDING AND CURRENT ROW), 2) AS moyenne_mobile_7j
FROM assemble a;

CREATE UNIQUE INDEX ux_mv_daily_signups ON analytics.mv_daily_signups (jour, cle_pays);
CREATE INDEX ix_mv_daily_signups_pays   ON analytics.mv_daily_signups (cle_pays, jour DESC);

COMMENT ON MATERIALIZED VIEW analytics.mv_daily_signups IS
    'Inscriptions de personnes par jour, série continue (jours à zéro compris), ventilées par pays et par statut OIF. Ligne cle_pays = UUID nul : total du jour.';
COMMENT ON COLUMN analytics.mv_daily_signups.cle_pays IS
    'Clé de ventilation NON NULLE (UUID nul = toutes origines confondues) : condition du rafraîchissement CONCURRENTLY.';
COMMENT ON COLUMN analytics.mv_daily_signups.inscriptions_cumulees IS
    'Total courant depuis le début de la série. Sur les lignes par pays, le cumul reste exact malgré les jours absents.';

-- -----------------------------------------------------------------------------
-- 3. Entonnoir des propositions
--
-- Écran « nombre de soumissions, activités approuvées, rejetées », par événement
-- ET par appel — la v2 admettant plusieurs appels par édition (journée jeunesse,
-- journée finance...), un entonnoir agrégé au seul niveau de l'événement
-- masquerait des écarts de sélectivité considérables d'un appel à l'autre.
--
-- Les appels sans aucune proposition figurent dans la projection, avec des
-- compteurs à zéro : un appel ouvert que personne n'a vu est précisément
-- l'information que l'équipe doit voir remonter.
-- -----------------------------------------------------------------------------
CREATE MATERIALIZED VIEW analytics.mv_proposal_funnel AS
WITH perimetre AS (
    -- Tous les appels déclarés, y compris ceux restés sans dépôt...
    SELECT c.event_id, c.id AS call_id
    FROM event.calls_for_proposals c
    UNION
    -- ...et les propositions hors appel (programmation directe de l'IFDD).
    SELECT p.event_id, p.call_id
    FROM programme.proposals p
    WHERE p.deleted_at IS NULL
),
dossiers AS (
    SELECT
        p.id AS proposal_id, p.event_id, p.call_id, p.status,
        p.submitted_at, p.decided_at, p.organization_id, p.average_score
    FROM programme.proposals p
    WHERE p.deleted_at IS NULL
),
sessions_issues AS (
    SELECT p.event_id, p.call_id, count(*) AS sessions_programmees
    FROM programme.sessions s
    JOIN programme.proposals p ON p.id = s.proposal_id
    WHERE s.status <> 'cancelled'
    GROUP BY p.event_id, p.call_id
),
agregats AS (
    SELECT
        pe.event_id,
        pe.call_id,
        count(d.proposal_id)                                                AS total,
        count(*) FILTER (WHERE d.status = 'draft')                          AS brouillons,
        count(*) FILTER (WHERE d.submitted_at IS NOT NULL)                  AS deposees,
        count(*) FILTER (WHERE d.status = 'submitted')                      AS en_attente_affectation,
        count(*) FILTER (WHERE d.status = 'under_review')                   AS en_revue,
        count(*) FILTER (WHERE d.status = 'changes_requested')              AS modifications_demandees,
        count(*) FILTER (WHERE d.status = 'accepted')                       AS acceptees,
        count(*) FILTER (WHERE d.status = 'rejected')                       AS rejetees,
        count(*) FILTER (WHERE d.status = 'withdrawn')                      AS retirees,
        count(*) FILTER (WHERE d.status = 'cancelled')                      AS annulees,
        count(DISTINCT d.organization_id)                                   AS organisations_distinctes,
        min(d.submitted_at)                                                 AS premier_depot,
        max(d.submitted_at)                                                 AS dernier_depot,
        round(avg(d.average_score), 2)                                      AS note_moyenne,
        -- Délai MÉDIAN et non moyen : deux dossiers arbitrés six mois plus tard
        -- (recours, appel prolongé) déplacent la moyenne de plusieurs semaines
        -- et donnent une image fausse du rythme réel du comité.
        percentile_cont(0.5) WITHIN GROUP (ORDER BY (d.decided_at - d.submitted_at))
            FILTER (WHERE d.decided_at IS NOT NULL AND d.submitted_at IS NOT NULL) AS delai_median_decision,
        avg(d.decided_at - d.submitted_at)
            FILTER (WHERE d.decided_at IS NOT NULL AND d.submitted_at IS NOT NULL) AS delai_moyen_decision
    FROM perimetre pe
    LEFT JOIN dossiers d
           ON d.event_id = pe.event_id
          AND d.call_id IS NOT DISTINCT FROM pe.call_id
    GROUP BY pe.event_id, pe.call_id
)
SELECT
    a.event_id,
    COALESCE(a.call_id, '00000000-0000-0000-0000-000000000000'::uuid) AS cle_appel,
    a.call_id,
    platform.t(e.title)          AS evenement,
    e.edition_year,
    e.status::text               AS statut_evenement,
    c.code                       AS code_appel,
    platform.t(c.title)          AS appel,
    c.status::text               AS statut_appel,
    c.opens_at                   AS appel_ouvre_le,
    COALESCE(c.extended_until, c.closes_at) AS appel_ferme_le,
    c.required_reviews,
    a.total,
    a.brouillons,
    a.deposees,
    a.en_attente_affectation,
    a.en_revue,
    a.modifications_demandees,
    a.acceptees,
    a.rejetees,
    a.retirees,
    a.annulees,
    (a.acceptees + a.rejetees)                                          AS decidees,
    GREATEST(a.deposees - a.acceptees - a.rejetees - a.retirees, 0)     AS en_instance,
    -- Deux ratios volontairement distincts : le premier mesure la sélectivité
    -- du comité (sur les dossiers réellement tranchés), le second le rendement
    -- de l'appel (sur tout ce qui a été déposé, retraits compris).
    round(a.acceptees::numeric / NULLIF(a.acceptees + a.rejetees, 0), 4) AS taux_acceptation,
    round(a.acceptees::numeric / NULLIF(a.deposees, 0), 4)              AS taux_acceptation_sur_depots,
    a.organisations_distinctes,
    a.note_moyenne,
    a.delai_median_decision,
    round(EXTRACT(EPOCH FROM a.delai_median_decision)::numeric / 3600, 1) AS delai_median_decision_heures,
    a.delai_moyen_decision,
    a.premier_depot,
    a.dernier_depot,
    COALESCE(si.sessions_programmees, 0)                                AS sessions_programmees
FROM agregats a
JOIN event.events e ON e.id = a.event_id
LEFT JOIN event.calls_for_proposals c ON c.id = a.call_id
LEFT JOIN sessions_issues si
       ON si.event_id = a.event_id
      AND si.call_id IS NOT DISTINCT FROM a.call_id;

CREATE UNIQUE INDEX ux_mv_proposal_funnel ON analytics.mv_proposal_funnel (event_id, cle_appel);
CREATE INDEX ix_mv_proposal_funnel_annee  ON analytics.mv_proposal_funnel (edition_year DESC);

COMMENT ON MATERIALIZED VIEW analytics.mv_proposal_funnel IS
    'Entonnoir des propositions par événement et par appel : dépôts, revue, acceptations, rejets, retraits, taux d''acceptation et délai médian de décision.';
COMMENT ON COLUMN analytics.mv_proposal_funnel.cle_appel IS
    'Identifiant d''appel NON NUL (UUID nul = propositions hors appel) : condition du rafraîchissement CONCURRENTLY.';
COMMENT ON COLUMN analytics.mv_proposal_funnel.delai_median_decision IS
    'Médiane du délai dépôt -> décision. La médiane, et non la moyenne, pour ne pas être déformée par quelques dossiers arbitrés très tardivement.';

-- -----------------------------------------------------------------------------
-- 4. Soumissions par jour et par événement
--
-- Courbe de dépôt d'un appel : elle dit à l'IFDD si une prolongation d'échéance
-- a produit un effet, et si l'effet de dernière minute (60 % des dépôts sur les
-- 48 dernières heures, observé en v1) se reproduit. Série continue, comme pour
-- les inscriptions : un jour sans dépôt est une information, pas une absence.
-- -----------------------------------------------------------------------------
CREATE MATERIALIZED VIEW analytics.mv_daily_submissions AS
WITH fenetres AS (
    SELECT
        e.id AS event_id,
        COALESCE(
            min((p.submitted_at AT TIME ZONE 'UTC')::date),
            min((c.opens_at AT TIME ZONE 'UTC')::date)
        ) AS debut,
        GREATEST(
            max((p.submitted_at AT TIME ZONE 'UTC')::date),
            min((c.opens_at AT TIME ZONE 'UTC')::date),
            LEAST(max((COALESCE(c.extended_until, c.closes_at) AT TIME ZONE 'UTC')::date), CURRENT_DATE)
        ) AS fin
    FROM event.events e
    LEFT JOIN event.calls_for_proposals c ON c.event_id = e.id
    LEFT JOIN programme.proposals p
           ON p.event_id = e.id AND p.deleted_at IS NULL AND p.submitted_at IS NOT NULL
    GROUP BY e.id
),
calendrier AS (
    SELECT f.event_id, g::date AS jour
    FROM fenetres f
    CROSS JOIN LATERAL generate_series(
        -- Borne de sécurité : deux ans de série au plus par événement.
        GREATEST(f.debut, f.fin - 730)::timestamp,
        f.fin::timestamp,
        interval '1 day'
    ) AS g
    WHERE f.debut IS NOT NULL AND f.fin IS NOT NULL AND f.fin >= f.debut
),
depots AS (
    SELECT
        p.event_id,
        (p.submitted_at AT TIME ZONE 'UTC')::date AS jour,
        count(*)                          AS soumissions,
        count(DISTINCT p.organization_id)  AS organisations_distinctes
    FROM programme.proposals p
    WHERE p.deleted_at IS NULL AND p.submitted_at IS NOT NULL
    GROUP BY 1, 2
),
decisions AS (
    SELECT
        p.event_id,
        (p.decided_at AT TIME ZONE 'UTC')::date AS jour,
        count(*) FILTER (WHERE p.status = 'accepted') AS acceptations,
        count(*) FILTER (WHERE p.status = 'rejected') AS rejets
    FROM programme.proposals p
    WHERE p.deleted_at IS NULL AND p.decided_at IS NOT NULL
    GROUP BY 1, 2
)
SELECT
    cal.jour,
    cal.event_id,
    platform.t(e.title)                  AS evenement,
    e.edition_year,
    COALESCE(d.soumissions, 0)           AS soumissions,
    COALESCE(d.organisations_distinctes, 0) AS organisations_distinctes,
    COALESCE(dec.acceptations, 0)        AS acceptations,
    COALESCE(dec.rejets, 0)              AS rejets,
    sum(COALESCE(d.soumissions, 0)) OVER (PARTITION BY cal.event_id ORDER BY cal.jour
                                          ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS soumissions_cumulees,
    round(avg(COALESCE(d.soumissions, 0)) OVER (PARTITION BY cal.event_id ORDER BY cal.jour
                                                ROWS BETWEEN 6 PRECEDING AND CURRENT ROW), 2) AS moyenne_mobile_7j
FROM calendrier cal
JOIN event.events e   ON e.id = cal.event_id
LEFT JOIN depots d    ON d.event_id = cal.event_id AND d.jour = cal.jour
LEFT JOIN decisions dec ON dec.event_id = cal.event_id AND dec.jour = cal.jour;

CREATE UNIQUE INDEX ux_mv_daily_submissions ON analytics.mv_daily_submissions (jour, event_id);
CREATE INDEX ix_mv_daily_submissions_event  ON analytics.mv_daily_submissions (event_id, jour DESC);

COMMENT ON MATERIALIZED VIEW analytics.mv_daily_submissions IS
    'Dépôts de propositions par jour et par événement, série continue de l''ouverture de l''appel à son échéance (ou à aujourd''hui).';

-- -----------------------------------------------------------------------------
-- 4 bis. Inscriptions aux activités, par jour et par événement
--
-- POURQUOI CETTE PROJECTION EXISTE, alors que mv_daily_signups compte déjà des
-- « inscriptions par jour ». Les deux mots recouvrent deux faits sans rapport :
-- mv_daily_signups compte des CRÉATIONS DE COMPTE sur la plateforme entière,
-- mv_daily_registrations compte des INSCRIPTIONS À UNE ACTIVITÉ d'une édition
-- donnée. Le tableau de bord du back-office (écran A6) a besoin du second — la
-- question qu'on se pose dans les semaines qui précèdent une COP est « le public
-- s'inscrit-il aux activités ? », pas « combien de comptes se sont créés ».
--
-- ET SURTOUT : mv_daily_signups N'EST PAS VENTILABLE PAR ÉVÉNEMENT. Un
-- administrateur détaché sur une seule édition (règle du périmètre
-- d'administration, identity.administered_events) ne peut donc rien en lire qui
-- le concerne. Cette projection-ci porte event_id dans sa clé, ce qui la rend
-- filtrable comme tout le reste du back-office.
--
-- Série CONTINUE, pour la même raison que les deux séries précédentes : un jour
-- sans inscription est une information, pas une absence. La fenêtre court de la
-- première inscription (ou de l'ouverture de l'appel, faute d'inscription) à la
-- fin de l'édition, sans dépasser aujourd'hui — projeter une courbe dans le
-- futur donnerait une longue traîne de zéros qui écrase la lecture du passé.
-- -----------------------------------------------------------------------------
CREATE MATERIALIZED VIEW analytics.mv_daily_registrations AS
WITH inscriptions AS (
    SELECT
        s.event_id,
        (r.created_at AT TIME ZONE 'UTC')::date AS jour,
        r.status,
        r.joined_at,
        r.person_id
    FROM programme.registrations r
    JOIN programme.sessions s ON s.id = r.session_id
),
fenetres AS (
    SELECT
        e.id AS event_id,
        COALESCE(min(i.jour), (e.starts_at AT TIME ZONE 'UTC')::date) AS debut,
        LEAST(
            GREATEST(
                COALESCE(max(i.jour), (e.starts_at AT TIME ZONE 'UTC')::date),
                (e.ends_at AT TIME ZONE 'UTC')::date
            ),
            CURRENT_DATE
        ) AS fin
    FROM event.events e
    LEFT JOIN inscriptions i ON i.event_id = e.id
    GROUP BY e.id, e.starts_at, e.ends_at
),
calendrier AS (
    SELECT f.event_id, g::date AS jour
    FROM fenetres f
    CROSS JOIN LATERAL generate_series(
        -- Borne de sécurité : deux ans de série au plus par événement, comme
        -- pour les dépôts.
        GREATEST(f.debut, f.fin - 730)::timestamp,
        f.fin::timestamp,
        interval '1 day'
    ) AS g
    WHERE f.debut IS NOT NULL AND f.fin IS NOT NULL AND f.fin >= f.debut
),
par_jour AS (
    SELECT
        i.event_id,
        i.jour,
        count(*) FILTER (WHERE i.status <> 'cancelled')      AS inscriptions,
        count(*) FILTER (WHERE i.status = 'waitlisted')      AS liste_attente,
        count(*) FILTER (WHERE i.status = 'cancelled')       AS annulations,
        count(*) FILTER (WHERE i.joined_at IS NOT NULL)      AS presents,
        count(DISTINCT i.person_id) FILTER (WHERE i.status <> 'cancelled') AS personnes_distinctes
    FROM inscriptions i
    GROUP BY i.event_id, i.jour
)
SELECT
    cal.jour,
    cal.event_id,
    platform.t(e.title)                     AS evenement,
    e.edition_year,
    COALESCE(pj.inscriptions, 0)            AS inscriptions,
    COALESCE(pj.liste_attente, 0)           AS liste_attente,
    COALESCE(pj.annulations, 0)             AS annulations,
    COALESCE(pj.presents, 0)                AS presents,
    COALESCE(pj.personnes_distinctes, 0)    AS personnes_distinctes,
    sum(COALESCE(pj.inscriptions, 0)) OVER (PARTITION BY cal.event_id ORDER BY cal.jour
                                            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS inscriptions_cumulees,
    round(avg(COALESCE(pj.inscriptions, 0)) OVER (PARTITION BY cal.event_id ORDER BY cal.jour
                                                  ROWS BETWEEN 6 PRECEDING AND CURRENT ROW), 2) AS moyenne_mobile_7j
FROM calendrier cal
JOIN event.events e ON e.id = cal.event_id
LEFT JOIN par_jour pj ON pj.event_id = cal.event_id AND pj.jour = cal.jour;

CREATE UNIQUE INDEX ux_mv_daily_registrations ON analytics.mv_daily_registrations (jour, event_id);
CREATE INDEX ix_mv_daily_registrations_event  ON analytics.mv_daily_registrations (event_id, jour DESC);

COMMENT ON MATERIALIZED VIEW analytics.mv_daily_registrations IS
    'Inscriptions aux activités par jour et par événement, série continue. À ne pas confondre avec mv_daily_signups, qui compte des créations de compte sur toute la plateforme.';
COMMENT ON COLUMN analytics.mv_daily_registrations.inscriptions IS
    'Inscriptions non annulées créées ce jour-là, liste d''attente comprise. Les annulations sont comptées à part, jamais soustraites.';
COMMENT ON COLUMN analytics.mv_daily_registrations.personnes_distinctes IS
    'Personnes distinctes inscrites ce jour-là : une même personne inscrite à trois activités le même jour ne compte qu''une fois.';

-- -----------------------------------------------------------------------------
-- 5. Fiche de performance des organisations
--
-- C'EST l'écran « liste des organisations » du back-office : « leurs activités,
-- leurs membres, nombre d'activités validées, ratio ». En v1, cette page
-- déclenchait une requête agrégée par ligne affichée (le N+1 classique) ;
-- ici, la liste complète tient en un `SELECT ... ORDER BY ... LIMIT`.
--
-- Les fiches fusionnées et rejetées restent présentes, avec leur statut : le
-- back-office les filtre, mais l'historique d'une fiche absorbée doit rester
-- consultable après une fusion (org.merge_organizations conserve la fiche).
-- -----------------------------------------------------------------------------
CREATE MATERIALIZED VIEW analytics.mv_organization_scorecard AS
WITH membres AS (
    SELECT
        m.organization_id,
        count(*) FILTER (WHERE m.status = 'active')                          AS membres_actifs,
        count(*) FILTER (WHERE m.status = 'pending')                         AS membres_en_attente,
        count(*) FILTER (WHERE m.status = 'active' AND m.role = 'manager')   AS referents,
        max(m.created_at)                                                    AS derniere_adhesion
    FROM org.memberships m
    GROUP BY m.organization_id
),
propositions AS (
    SELECT
        p.organization_id,
        count(*) FILTER (WHERE p.submitted_at IS NOT NULL)      AS propositions_deposees,
        count(*) FILTER (WHERE p.status = 'draft')              AS propositions_en_brouillon,
        count(*) FILTER (WHERE p.status = 'accepted')           AS propositions_acceptees,
        count(*) FILTER (WHERE p.status = 'rejected')           AS propositions_rejetees,
        count(*) FILTER (WHERE p.status = 'withdrawn')          AS propositions_retirees,
        count(DISTINCT p.event_id) FILTER (WHERE p.submitted_at IS NOT NULL) AS evenements_couverts,
        round(avg(p.average_score), 2)                          AS note_moyenne_obtenue,
        max(p.created_at)                                       AS derniere_proposition
    FROM programme.proposals p
    WHERE p.deleted_at IS NULL
    GROUP BY p.organization_id
),
sessions AS (
    SELECT
        s.organization_id,
        count(*) FILTER (WHERE s.status IN ('scheduled', 'live', 'completed')) AS sessions_programmees,
        count(*) FILTER (WHERE s.status = 'completed')                          AS sessions_realisees,
        count(*) FILTER (WHERE s.status = 'cancelled')                          AS sessions_annulees,
        max(s.starts_at)                                                        AS derniere_session
    FROM programme.sessions s
    WHERE s.organization_id IS NOT NULL
    GROUP BY s.organization_id
),
participation AS (
    SELECT
        s.organization_id,
        count(r.id) FILTER (WHERE r.status <> 'cancelled')  AS inscrits_a_ses_sessions,
        count(r.id) FILTER (WHERE r.joined_at IS NOT NULL)  AS presents_a_ses_sessions
    FROM programme.sessions s
    JOIN programme.registrations r ON r.session_id = s.id
    WHERE s.organization_id IS NOT NULL
    GROUP BY s.organization_id
),
articles AS (
    SELECT
        a.organization_id,
        count(*) FILTER (WHERE a.status = 'published')                   AS articles_publies,
        count(*) FILTER (WHERE a.status IN ('submitted', 'in_review'))   AS articles_en_moderation,
        COALESCE(sum(a.storage_bytes), 0)                                AS octets_stockes,
        max(a.published_at)                                              AS dernier_article
    FROM publication.articles a
    WHERE a.deleted_at IS NULL
    GROUP BY a.organization_id
)
SELECT
    o.id                             AS organization_id,
    o.legal_name,
    o.acronym,
    o.slug,
    o.status::text                   AS statut,
    o.organization_type_code,
    o.country_id,
    c.iso3                           AS pays_iso3,
    platform.t(c.name)               AS pays_nom,
    COALESCE(c.oif_status::text, 'none') AS statut_oif,
    (o.verified_at IS NOT NULL)      AS est_verifiee,
    o.verified_at,
    o.trust_score                    AS score_confiance,
    o.merged_into_id,

    COALESCE(mb.membres_actifs, 0)          AS membres_actifs,
    COALESCE(mb.membres_en_attente, 0)      AS membres_en_attente,
    COALESCE(mb.referents, 0)               AS referents,

    COALESCE(pr.propositions_deposees, 0)   AS propositions_deposees,
    COALESCE(pr.propositions_en_brouillon, 0) AS propositions_en_brouillon,
    COALESCE(pr.propositions_acceptees, 0)  AS propositions_acceptees,
    COALESCE(pr.propositions_rejetees, 0)   AS propositions_rejetees,
    COALESCE(pr.propositions_retirees, 0)   AS propositions_retirees,
    COALESCE(pr.evenements_couverts, 0)     AS evenements_couverts,
    pr.note_moyenne_obtenue,
    -- LE ratio du cadrage : part des dossiers déposés qui aboutissent.
    round(pr.propositions_acceptees::numeric / NULLIF(pr.propositions_deposees, 0), 4) AS ratio_acceptation,

    COALESCE(se.sessions_programmees, 0)    AS sessions_programmees,
    COALESCE(se.sessions_realisees, 0)      AS sessions_realisees,
    COALESCE(se.sessions_annulees, 0)       AS sessions_annulees,
    COALESCE(pa.inscrits_a_ses_sessions, 0) AS inscrits_a_ses_sessions,
    COALESCE(pa.presents_a_ses_sessions, 0) AS presents_a_ses_sessions,

    COALESCE(ar.articles_publies, 0)        AS articles_publies,
    COALESCE(ar.articles_en_moderation, 0)  AS articles_en_moderation,
    COALESCE(ar.octets_stockes, 0)          AS octets_stockes,

    -- Dernier signe de vie, toutes natures confondues : c'est le tri qui permet
    -- de distinguer une organisation dormante d'une organisation active, sans
    -- lire six écrans.
    GREATEST(
        o.updated_at, mb.derniere_adhesion, pr.derniere_proposition,
        se.derniere_session, ar.dernier_article
    ) AS derniere_activite,
    o.created_at                     AS inscrite_le
FROM org.organizations o
LEFT JOIN reference.countries c ON c.id = o.country_id
LEFT JOIN membres       mb ON mb.organization_id = o.id
LEFT JOIN propositions  pr ON pr.organization_id = o.id
LEFT JOIN sessions      se ON se.organization_id = o.id
LEFT JOIN participation pa ON pa.organization_id = o.id
LEFT JOIN articles      ar ON ar.organization_id = o.id;

CREATE UNIQUE INDEX ux_mv_organization_scorecard ON analytics.mv_organization_scorecard (organization_id);
CREATE INDEX ix_mv_organization_scorecard_ratio  ON analytics.mv_organization_scorecard (ratio_acceptation DESC NULLS LAST);
CREATE INDEX ix_mv_organization_scorecard_statut ON analytics.mv_organization_scorecard (statut, derniere_activite DESC NULLS LAST);
CREATE INDEX ix_mv_organization_scorecard_pays   ON analytics.mv_organization_scorecard (country_id);

COMMENT ON MATERIALIZED VIEW analytics.mv_organization_scorecard IS
    'Fiche de performance par organisation : membres, dépôts, acceptations, ratio, sessions, publications, dernière activité et score de confiance. Alimente la liste des organisations du back-office.';
COMMENT ON COLUMN analytics.mv_organization_scorecard.ratio_acceptation IS
    'Propositions acceptées / propositions déposées. NULL si l''organisation n''a jamais rien déposé — un ratio de 0 serait un contresens.';
COMMENT ON COLUMN analytics.mv_organization_scorecard.derniere_activite IS
    'Dernier signe de vie toutes natures confondues (fiche, adhésion, dépôt, session, article). Tri de référence pour repérer les fiches dormantes.';

-- -----------------------------------------------------------------------------
-- 6. Participation par session
--
-- Le taux de participation RÉEL est l'apport majeur de la v2 sur ce terrain :
-- `programme.registrations.joined_at` est écrit une seule fois, au premier clic
-- sur « Rejoindre ». La v1 ne connaissait que le nombre d'inscrits, ce qui
-- surestimait systématiquement l'audience d'un facteur deux à trois et rendait
-- tout bilan de COP discutable.
--
-- La ventilation par canal d'acquisition est lue dans le document
-- `registrations.answers` (clé = code du champ de formulaire, ici
-- `referral_source`), indexé en GIN : aucune table annexe à maintenir, et une
-- nouvelle question posée aux inscrits est exploitable sans migration.
-- -----------------------------------------------------------------------------
CREATE MATERIALIZED VIEW analytics.mv_session_attendance AS
WITH inscriptions AS (
    SELECT
        r.session_id,
        count(*)                                                        AS inscriptions_total,
        count(*) FILTER (WHERE r.status <> 'cancelled')                 AS inscrits,
        count(*) FILTER (WHERE r.status = 'waitlisted')                 AS liste_attente,
        count(*) FILTER (WHERE r.status = 'cancelled')                  AS annulations,
        count(*) FILTER (WHERE r.joined_at IS NOT NULL)                 AS presents,
        count(*) FILTER (WHERE r.status = 'no_show')                    AS absents_declares,
        count(DISTINCT r.organization_id) FILTER (WHERE r.organization_id IS NOT NULL) AS organisations_representees,
        count(*) FILTER (WHERE r.source = 'web')                        AS inscriptions_web,
        count(*) FILTER (WHERE r.source = 'import')                     AS inscriptions_importees,
        count(*) FILTER (WHERE r.locale = 'fr')                         AS inscriptions_fr,
        count(*) FILTER (WHERE r.locale = 'en')                         AS inscriptions_en,
        round(avg(r.attendance_minutes) FILTER (WHERE r.joined_at IS NOT NULL), 1) AS duree_moyenne_minutes,
        round((percentile_cont(0.5) WITHIN GROUP (ORDER BY r.attendance_minutes)
               FILTER (WHERE r.joined_at IS NOT NULL))::numeric, 1)     AS duree_mediane_minutes,
        min(r.created_at)                                               AS premiere_inscription,
        max(r.created_at)                                               AS derniere_inscription
    FROM programme.registrations r
    GROUP BY r.session_id
),
canaux_bruts AS (
    SELECT
        r.session_id,
        -- Le champ peut être absent (formulaire sans la question), vide (réponse
        -- non obligatoire) ou renseigné : les trois cas sont ramenés à un code
        -- unique pour que la somme des canaux égale toujours le nombre d'inscrits.
        COALESCE(NULLIF(btrim(r.answers ->> 'referral_source'), ''), 'non_renseigne') AS canal,
        count(*) AS n
    FROM programme.registrations r
    WHERE r.status <> 'cancelled'
    GROUP BY 1, 2
),
canaux AS (
    SELECT
        cb.session_id,
        jsonb_object_agg(cb.canal, cb.n ORDER BY cb.n DESC, cb.canal)  AS canaux_acquisition,
        (array_agg(cb.canal ORDER BY cb.n DESC, cb.canal))[1]          AS canal_principal
    FROM canaux_bruts cb
    GROUP BY cb.session_id
),
visio AS (
    -- Contrôle croisé : la présence vue par le fournisseur de visioconférence.
    -- Un écart durable entre `presents` et `presents_visio` signale un défaut de
    -- traçage côté plateforme, pas une baisse d'audience.
    SELECT
        m.session_id,
        count(*) FILTER (WHERE mp.joined_at IS NOT NULL)          AS presents_visio,
        round(avg(mp.attendance_seconds) FILTER (WHERE mp.joined_at IS NOT NULL) / 60.0, 1) AS duree_moyenne_visio_minutes
    FROM live.meetings m
    JOIN live.meeting_participants mp ON mp.meeting_id = m.id
    WHERE m.session_id IS NOT NULL
    GROUP BY m.session_id
)
SELECT
    s.id                             AS session_id,
    s.event_id,
    platform.t(e.title)              AS evenement,
    e.edition_year,
    s.event_day_id,
    platform.t(s.title)              AS session,
    s.slug,
    s.status::text                   AS statut,
    s.format::text                   AS format,
    s.starts_at,
    s.ends_at,
    round(EXTRACT(EPOCH FROM (s.ends_at - s.starts_at))::numeric / 60, 0) AS duree_prevue_minutes,
    s.organization_id,
    o.legal_name                     AS organisation,
    s.capacity                       AS jauge,

    COALESCE(i.inscrits, 0)          AS inscrits,
    COALESCE(i.liste_attente, 0)     AS liste_attente,
    COALESCE(i.annulations, 0)       AS annulations,
    COALESCE(i.presents, 0)          AS presents,
    COALESCE(i.absents_declares, 0)  AS absents_declares,
    -- LE chiffre : présents / inscrits non annulés. NULL et non zéro quand
    -- personne ne s'est inscrit, pour ne pas polluer les moyennes d'événement.
    round(i.presents::numeric / NULLIF(i.inscrits, 0), 4) AS taux_participation,
    round(i.inscrits::numeric / NULLIF(s.capacity, 0), 4) AS taux_remplissage,
    i.duree_moyenne_minutes,
    i.duree_mediane_minutes,
    COALESCE(i.organisations_representees, 0) AS organisations_representees,
    COALESCE(i.inscriptions_web, 0)      AS inscriptions_web,
    COALESCE(i.inscriptions_importees, 0) AS inscriptions_importees,
    COALESCE(i.inscriptions_fr, 0)       AS inscriptions_fr,
    COALESCE(i.inscriptions_en, 0)       AS inscriptions_en,
    i.premiere_inscription,
    i.derniere_inscription,

    COALESCE(cn.canaux_acquisition, '{}'::jsonb) AS canaux_acquisition,
    cn.canal_principal,

    COALESCE(v.presents_visio, 0)    AS presents_visio,
    v.duree_moyenne_visio_minutes,
    s.attendee_count                 AS presents_declares_par_organisateur,
    s.view_count                     AS vues_page
FROM programme.sessions s
JOIN event.events e            ON e.id = s.event_id
LEFT JOIN org.organizations o  ON o.id = s.organization_id
LEFT JOIN inscriptions i       ON i.session_id = s.id
LEFT JOIN canaux cn            ON cn.session_id = s.id
LEFT JOIN visio v              ON v.session_id = s.id;

CREATE UNIQUE INDEX ux_mv_session_attendance ON analytics.mv_session_attendance (session_id);
CREATE INDEX ix_mv_session_attendance_event  ON analytics.mv_session_attendance (event_id, starts_at);
CREATE INDEX ix_mv_session_attendance_taux   ON analytics.mv_session_attendance (taux_participation DESC NULLS LAST);
CREATE INDEX ix_mv_session_attendance_canaux ON analytics.mv_session_attendance USING gin (canaux_acquisition jsonb_path_ops);

COMMENT ON MATERIALIZED VIEW analytics.mv_session_attendance IS
    'Participation par session : inscrits, présents réels (joined_at), taux de participation, durées et ventilation par canal d''acquisition.';
COMMENT ON COLUMN analytics.mv_session_attendance.taux_participation IS
    'Présents / inscrits non annulés. Fondé sur joined_at (premier clic sur « Rejoindre »), donc réel — la v1 ne connaissait que les inscrits.';
COMMENT ON COLUMN analytics.mv_session_attendance.canaux_acquisition IS
    'Ventilation {canal: effectif} lue dans registrations.answers->>''referral_source''. Le code `non_renseigne` couvre les réponses absentes ou vides.';
COMMENT ON COLUMN analytics.mv_session_attendance.presents_visio IS
    'Présence relevée chez le fournisseur de visioconférence. Sert de contrôle croisé du traçage plateforme, pas de source principale.';

-- -----------------------------------------------------------------------------
-- 7. Charge de travail du comité de sélection
--
-- Répond à deux questions de pilotage que la v1 ne savait pas poser :
-- qui prend du retard, et qui note plus sévèrement que ses pairs. La seconde
-- est structurante pour l'équité de la sélection : si un révisionniste note en
-- moyenne trois points sous la moyenne de l'événement, les propositions qui lui
-- ont été confiées sont désavantagées par le tirage, pas par leur qualité.
--
-- Grain (révisionniste, événement) : la charge se pilote COP par COP, et
-- comparer la sévérité n'a de sens qu'à grille de notation comparable.
-- -----------------------------------------------------------------------------
CREATE MATERIALIZED VIEW analytics.mv_reviewer_workload AS
WITH cles AS (
    SELECT ra.reviewer_id, p.event_id
    FROM programme.review_assignments ra
    JOIN programme.proposals p ON p.id = ra.proposal_id AND p.deleted_at IS NULL
    UNION
    SELECT r.reviewer_id, p.event_id
    FROM programme.reviews r
    JOIN programme.proposals p ON p.id = r.proposal_id AND p.deleted_at IS NULL
),
affectations AS (
    SELECT
        ra.reviewer_id,
        p.event_id,
        count(*) FILTER (WHERE ra.recused_at IS NULL)   AS propositions_assignees,
        count(*) FILTER (WHERE ra.recused_at IS NOT NULL) AS deports,
        -- En retard = échéance dépassée sans revue soumise, déports exclus.
        count(*) FILTER (
            WHERE ra.recused_at IS NULL
              AND ra.due_at IS NOT NULL
              AND ra.due_at < now()
              AND NOT EXISTS (
                  SELECT 1 FROM programme.reviews rv
                  WHERE rv.proposal_id = ra.proposal_id
                    AND rv.reviewer_id = ra.reviewer_id
                    AND rv.submitted_at IS NOT NULL
              )
        ) AS revues_en_retard,
        min(ra.due_at) FILTER (WHERE ra.recused_at IS NULL) AS prochaine_echeance,
        max(ra.assigned_at)                                 AS derniere_affectation
    FROM programme.review_assignments ra
    JOIN programme.proposals p ON p.id = ra.proposal_id AND p.deleted_at IS NULL
    GROUP BY ra.reviewer_id, p.event_id
),
revues AS (
    SELECT
        r.reviewer_id,
        p.event_id,
        count(*) FILTER (WHERE r.submitted_at IS NOT NULL)     AS revues_soumises,
        count(*) FILTER (WHERE r.submitted_at IS NULL)         AS revues_en_cours,
        count(*) FILTER (WHERE r.recommendation = 'accept')    AS avis_favorables,
        count(*) FILTER (WHERE r.recommendation = 'reject')    AS avis_defavorables,
        round(avg(r.score_out_of_20) FILTER (WHERE r.submitted_at IS NOT NULL), 2) AS note_moyenne_attribuee,
        round(stddev_samp(r.score_out_of_20) FILTER (WHERE r.submitted_at IS NOT NULL), 2) AS ecart_type_notes,
        avg(r.submitted_at - r.created_at) FILTER (WHERE r.submitted_at IS NOT NULL) AS delai_moyen_revue,
        max(r.submitted_at)                                    AS derniere_revue_le
    FROM programme.reviews r
    JOIN programme.proposals p ON p.id = r.proposal_id AND p.deleted_at IS NULL
    GROUP BY r.reviewer_id, p.event_id
),
reference_evenement AS (
    -- Moyenne de toutes les revues soumises de l'événement : l'étalon auquel se
    -- compare chaque membre du comité.
    SELECT
        p.event_id,
        round(avg(r.score_out_of_20), 2) AS note_moyenne_evenement
    FROM programme.reviews r
    JOIN programme.proposals p ON p.id = r.proposal_id AND p.deleted_at IS NULL
    WHERE r.submitted_at IS NOT NULL
    GROUP BY p.event_id
)
SELECT
    k.reviewer_id,
    k.event_id,
    pe.display_name                  AS revisionniste,
    platform.t(e.title)              AS evenement,
    e.edition_year,
    COALESCE(af.propositions_assignees, 0) AS propositions_assignees,
    COALESCE(af.deports, 0)                AS deports,
    COALESCE(rv.revues_soumises, 0)        AS revues_soumises,
    COALESCE(rv.revues_en_cours, 0)        AS revues_en_cours,
    COALESCE(af.revues_en_retard, 0)       AS revues_en_retard,
    GREATEST(COALESCE(af.propositions_assignees, 0) - COALESCE(rv.revues_soumises, 0), 0) AS revues_restantes,
    round(rv.revues_soumises::numeric / NULLIF(af.propositions_assignees, 0), 4) AS taux_completion,
    COALESCE(rv.avis_favorables, 0)        AS avis_favorables,
    COALESCE(rv.avis_defavorables, 0)      AS avis_defavorables,
    rv.note_moyenne_attribuee,
    re.note_moyenne_evenement,
    round(rv.note_moyenne_attribuee - re.note_moyenne_evenement, 2) AS ecart_a_la_moyenne,
    -- Étiquette de lecture : un point d'écart sur 20 est un biais réel de
    -- notation, en deçà c'est du bruit d'échantillonnage.
    CASE
        WHEN rv.note_moyenne_attribuee IS NULL OR re.note_moyenne_evenement IS NULL THEN 'indetermine'
        WHEN rv.note_moyenne_attribuee - re.note_moyenne_evenement <= -1 THEN 'severe'
        WHEN rv.note_moyenne_attribuee - re.note_moyenne_evenement >=  1 THEN 'genereux'
        ELSE 'dans_la_moyenne'
    END                                    AS profil_notation,
    rv.ecart_type_notes,
    rv.delai_moyen_revue,
    af.prochaine_echeance,
    af.derniere_affectation,
    rv.derniere_revue_le
FROM cles k
JOIN identity.people pe ON pe.id = k.reviewer_id
JOIN event.events e     ON e.id = k.event_id
LEFT JOIN affectations af ON af.reviewer_id = k.reviewer_id AND af.event_id = k.event_id
LEFT JOIN revues rv       ON rv.reviewer_id = k.reviewer_id AND rv.event_id = k.event_id
LEFT JOIN reference_evenement re ON re.event_id = k.event_id;

CREATE UNIQUE INDEX ux_mv_reviewer_workload ON analytics.mv_reviewer_workload (reviewer_id, event_id);
CREATE INDEX ix_mv_reviewer_workload_retard ON analytics.mv_reviewer_workload (event_id, revues_en_retard DESC);

COMMENT ON MATERIALIZED VIEW analytics.mv_reviewer_workload IS
    'Charge et comportement de notation du comité, par révisionniste et par événement : affectations, revues soumises, retards, note moyenne et écart à la moyenne de l''événement.';
COMMENT ON COLUMN analytics.mv_reviewer_workload.profil_notation IS
    'Sévérité relative : écart d''au moins un point sur 20 par rapport à la moyenne de l''événement. En deçà, l''écart n''est pas significatif.';
COMMENT ON COLUMN analytics.mv_reviewer_workload.revues_en_retard IS
    'Affectations dont l''échéance est dépassée sans revue soumise, déports exclus.';

-- -----------------------------------------------------------------------------
-- 8. Popularité des contenus
--
-- Agrégat de analytics.page_views. Le titre est résolu ici, une fois par
-- rafraîchissement, plutôt qu'à chaque affichage de la liste : le back-office
-- lit un tableau prêt, sans jointure vers quatre modules.
-- -----------------------------------------------------------------------------
CREATE MATERIALIZED VIEW analytics.mv_content_popularity AS
WITH vues AS (
    SELECT
        v.content_schema,
        v.content_table,
        COALESCE(v.content_id, '00000000-0000-0000-0000-000000000000'::uuid) AS cle_contenu,
        v.content_id,
        count(*)                                                          AS vues_total,
        count(*) FILTER (WHERE v.viewed_at >= now() - interval '24 hours') AS vues_24h,
        count(*) FILTER (WHERE v.viewed_at >= now() - interval '7 days')   AS vues_7j,
        count(*) FILTER (WHERE v.viewed_at >= now() - interval '30 days')  AS vues_30j,
        -- Visiteurs uniques sur 24 h seulement : le sel de l'empreinte tournant
        -- chaque jour, un « unique » calculé sur 30 jours compterait la même
        -- personne autant de fois qu'elle est revenue de jours différents. Mieux
        -- vaut un chiffre exact sur une journée qu'un chiffre faux sur un mois.
        count(DISTINCT v.visitor_hash) FILTER (WHERE v.viewed_at >= now() - interval '24 hours') AS visiteurs_uniques_24h,
        count(*) FILTER (WHERE v.is_authenticated)                         AS vues_authentifiees,
        count(*) FILTER (WHERE v.device_kind = 'mobile')                   AS vues_mobile,
        round(avg(v.duration_ms) FILTER (WHERE v.duration_ms IS NOT NULL) / 1000.0, 1) AS duree_moyenne_secondes,
        mode() WITHIN GROUP (ORDER BY v.locale)                            AS locale_dominante,
        mode() WITHIN GROUP (ORDER BY v.path)                              AS chemin_principal,
        min(v.viewed_at)                                                   AS premiere_vue,
        max(v.viewed_at)                                                   AS derniere_vue
    FROM analytics.page_views v
    GROUP BY v.content_schema, v.content_table, v.content_id
)
SELECT
    vu.content_schema,
    vu.content_table,
    vu.cle_contenu,
    vu.content_id,
    COALESCE(
        platform.t(s.title),
        platform.t(a.title),
        platform.t(ev.title),
        platform.t(pp.title),
        vu.chemin_principal
    ) AS titre,
    vu.vues_total,
    vu.vues_24h,
    vu.vues_7j,
    vu.vues_30j,
    vu.visiteurs_uniques_24h,
    vu.vues_authentifiees,
    vu.vues_mobile,
    vu.duree_moyenne_secondes,
    vu.locale_dominante,
    vu.chemin_principal,
    vu.premiere_vue,
    vu.derniere_vue,
    rank() OVER (ORDER BY vu.vues_30j DESC, vu.vues_total DESC) AS rang_30j
FROM vues vu
LEFT JOIN programme.sessions s   ON vu.content_schema = 'programme'   AND vu.content_table = 'sessions'  AND s.id  = vu.content_id
LEFT JOIN programme.proposals pp ON vu.content_schema = 'programme'   AND vu.content_table = 'proposals' AND pp.id = vu.content_id
LEFT JOIN publication.articles a ON vu.content_schema = 'publication' AND vu.content_table = 'articles'  AND a.id  = vu.content_id
LEFT JOIN event.events ev        ON vu.content_schema = 'event'       AND vu.content_table = 'events'    AND ev.id = vu.content_id;

CREATE UNIQUE INDEX ux_mv_content_popularity
    ON analytics.mv_content_popularity (content_schema, content_table, cle_contenu);
CREATE INDEX ix_mv_content_popularity_rang ON analytics.mv_content_popularity (rang_30j);

COMMENT ON MATERIALIZED VIEW analytics.mv_content_popularity IS
    'Popularité des contenus agrégée depuis analytics.page_views, titre résolu au rafraîchissement. Classement sur 30 jours glissants.';
COMMENT ON COLUMN analytics.mv_content_popularity.visiteurs_uniques_24h IS
    'Uniques sur 24 h seulement : le sel de l''empreinte tourne chaque jour, un unique multi-jours serait faux par construction.';

-- -----------------------------------------------------------------------------
-- 9. Journal des rafraîchissements
--
-- Sans ce journal, « le tableau de bord affiche des chiffres d'hier » est un
-- signalement invérifiable. Avec lui, l'âge de chaque projection et la durée de
-- son calcul sont des données — et v_operational_health peut alerter.
-- -----------------------------------------------------------------------------
CREATE TABLE analytics.refresh_log (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    view_name     text        NOT NULL,
    started_at    timestamptz NOT NULL DEFAULT now(),
    finished_at   timestamptz,
    duration_ms   integer,
    row_count     bigint,
    was_concurrent boolean    NOT NULL DEFAULT true,
    succeeded     boolean     NOT NULL DEFAULT false,
    error_code    text,
    error_message text,
    triggered_by  text        NOT NULL DEFAULT current_user
);

CREATE INDEX ix_refresh_log_view   ON analytics.refresh_log (view_name, started_at DESC);
CREATE INDEX ix_refresh_log_recent ON analytics.refresh_log (started_at DESC);
CREATE INDEX ix_refresh_log_echecs ON analytics.refresh_log (started_at DESC) WHERE NOT succeeded;

COMMENT ON TABLE analytics.refresh_log IS
    'Journal des rafraîchissements de vues matérialisées : durée, lignes produites, succès ou erreur. Base de l''alerte « tableaux de bord périmés ».';

-- Rafraîchissement de l'ensemble des projections.
--
-- L'ordre est explicite et non alphabétique : il suit la dépendance des données
-- (les projections nourries par les tables de faits en dernier), afin qu'une
-- future vue construite sur une autre soit rafraîchie après sa source.
--
-- Chaque vue est traitée dans son propre sous-bloc : l'échec de l'une n'annule
-- pas le rafraîchissement des autres, il est journalisé et remonté dans le
-- résultat. Un tableau de bord partiellement à jour vaut mieux qu'un tableau de
-- bord entièrement périmé parce qu'une seule agrégation a fauté.
CREATE OR REPLACE FUNCTION analytics.refresh_all(p_concurrently boolean DEFAULT true)
RETURNS TABLE (
    vue      text,
    duree_ms integer,
    lignes   bigint,
    succes   boolean,
    erreur   text
)
LANGUAGE plpgsql
AS $$
DECLARE
    v_vues   text[] := ARRAY[
        'mv_daily_signups',
        'mv_proposal_funnel',
        'mv_daily_submissions',
        'mv_daily_registrations',
        'mv_organization_scorecard',
        'mv_session_attendance',
        'mv_reviewer_workload',
        'mv_content_popularity'
    ];
    v_vue    text;
    v_debut  timestamptz;
    v_log_id uuid;
    v_lignes bigint;
    v_ms     integer;
    v_ok     boolean;
    v_err    text;
    v_code   text;
BEGIN
    FOREACH v_vue IN ARRAY v_vues LOOP
        v_debut  := clock_timestamp();
        v_lignes := NULL;
        v_ok     := true;
        v_err    := NULL;
        v_code   := NULL;

        INSERT INTO analytics.refresh_log (view_name, was_concurrent)
        VALUES (v_vue, p_concurrently)
        RETURNING id INTO v_log_id;

        BEGIN
            EXECUTE format(
                'REFRESH MATERIALIZED VIEW %s analytics.%I',
                CASE WHEN p_concurrently THEN 'CONCURRENTLY' ELSE '' END,
                v_vue
            );
            EXECUTE format('SELECT count(*) FROM analytics.%I', v_vue) INTO v_lignes;
        EXCEPTION WHEN OTHERS THEN
            v_ok   := false;
            GET STACKED DIAGNOSTICS v_err = MESSAGE_TEXT, v_code = RETURNED_SQLSTATE;
        END;

        v_ms := (EXTRACT(EPOCH FROM (clock_timestamp() - v_debut)) * 1000)::integer;

        UPDATE analytics.refresh_log
        SET finished_at   = clock_timestamp(),
            duration_ms   = v_ms,
            row_count     = v_lignes,
            succeeded     = v_ok,
            error_code    = v_code,
            error_message = v_err
        WHERE id = v_log_id;

        vue      := v_vue;
        duree_ms := v_ms;
        lignes   := v_lignes;
        succes   := v_ok;
        erreur   := v_err;
        RETURN NEXT;
    END LOOP;
END;
$$;

COMMENT ON FUNCTION analytics.refresh_all(boolean) IS
    'Rafraîchit toutes les vues matérialisées du module dans l''ordre, journalise durée et lignes, et isole les échecs vue par vue. p_concurrently = false uniquement pour le premier peuplement ou la maintenance hors ligne.';

-- Dépôt d'une demande de rafraîchissement dans la file de travaux. Appelée par
-- un abonné de l'outbox (après une décision de comité, une fin de session...)
-- ou par la planification périodique.
--
-- La clé d'idempotence contient une tranche de temps : cent événements de
-- domaine arrivant dans la même minute ne produisent qu'UN travail de
-- rafraîchissement. C'est le mécanisme qui évite qu'une clôture d'appel — donc
-- une rafale de décisions — ne déclenche cent recalculs complets.
CREATE OR REPLACE FUNCTION analytics.enqueue_refresh(
    p_concurrently      boolean DEFAULT true,
    p_delay             interval DEFAULT interval '0',
    p_debounce_seconds  integer  DEFAULT 300
)
RETURNS uuid
LANGUAGE plpgsql
AS $$
DECLARE
    v_tranche text;
    v_id      uuid;
BEGIN
    v_tranche := to_char(
        to_timestamp(floor(EXTRACT(EPOCH FROM clock_timestamp()) / GREATEST(p_debounce_seconds, 1))
                     * GREATEST(p_debounce_seconds, 1)) AT TIME ZONE 'UTC',
        'YYYYMMDD"T"HH24MISS'
    );

    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, priority, run_at, max_attempts)
    VALUES (
        'analytics',
        'analytics.refresh_all',
        jsonb_build_object('concurrently', p_concurrently, 'tranche', v_tranche),
        'refresh_all:' || v_tranche,
        -- Priorité basse (la valeur par défaut est 100, le plus urgent est le
        -- plus petit) : un rafraîchissement analytique ne doit jamais passer
        -- devant un envoi de confirmation d'inscription.
        200,
        now() + p_delay,
        3
    )
    ON CONFLICT (task, idempotency_key) WHERE idempotency_key IS NOT NULL AND status <> 'cancelled'
    DO NOTHING
    RETURNING id INTO v_id;

    -- NULL signifie « une demande est déjà en file pour cette tranche » : ce
    -- n'est pas une erreur, c'est le résultat attendu de l'anti-rebond.
    RETURN v_id;
END;
$$;

COMMENT ON FUNCTION analytics.enqueue_refresh(boolean, interval, integer) IS
    'Dépose un travail `analytics.refresh_all` dans platform.jobs, avec anti-rebond par tranche de temps. Renvoie NULL si une demande est déjà en file.';

-- -----------------------------------------------------------------------------
-- 10. Vue d'ensemble de la plateforme
--
-- Compteurs de la page d'accueil du back-office. Vue NON matérialisée,
-- volontairement : ces chiffres sont lus par une poignée d'administrateurs, ils
-- doivent être exacts à la seconde (« j'ai validé cette proposition, je veux la
-- voir dans le compteur »), et ils reposent sur des count() servis par des
-- index partiels. Matérialiser ici coûterait de la fraîcheur sans rien gagner.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW analytics.v_platform_overview AS
SELECT
    -- Personnes
    (SELECT count(*) FROM identity.people)                                          AS personnes_total,
    (SELECT count(*) FROM identity.people WHERE status = 'active')                  AS personnes_actives,
    (SELECT count(*) FROM identity.people WHERE status = 'anonymized')              AS personnes_anonymisees,
    (SELECT count(*) FROM identity.people WHERE email_verified_at IS NOT NULL)      AS personnes_verifiees,
    (SELECT count(DISTINCT person_id) FROM identity.accounts)                       AS personnes_avec_compte,
    (SELECT count(*) FROM identity.people WHERE created_at >= date_trunc('day', now())) AS inscriptions_aujourdhui,
    (SELECT count(*) FROM identity.people WHERE created_at >= now() - interval '7 days')  AS inscriptions_7j,
    (SELECT count(*) FROM identity.people WHERE created_at >= now() - interval '30 days') AS inscriptions_30j,

    -- Organisations
    (SELECT count(*) FROM org.organizations)                                        AS organisations_total,
    (SELECT count(*) FROM org.organizations WHERE status = 'active')                AS organisations_actives,
    (SELECT count(*) FROM org.organizations WHERE status = 'candidate')             AS organisations_a_valider,
    (SELECT count(*) FROM org.organizations WHERE verified_at IS NOT NULL)          AS organisations_verifiees,
    (SELECT count(*) FROM org.organizations WHERE status = 'merged')                AS organisations_fusionnees,
    (SELECT count(*) FROM org.duplicate_candidates WHERE reviewed_at IS NULL)       AS doublons_a_arbitrer,

    -- Événements et appels
    (SELECT count(*) FROM event.events)                                             AS evenements_total,
    (SELECT count(*) FROM event.events WHERE status = 'ongoing')                    AS evenements_en_cours,
    (SELECT count(*) FROM event.events WHERE status = 'announced' AND starts_at > now()) AS evenements_a_venir,
    (SELECT count(*) FROM event.calls_for_proposals
      WHERE status = 'open' AND now() BETWEEN opens_at AND COALESCE(extended_until, closes_at)) AS appels_ouverts,

    -- Propositions
    (SELECT count(*) FROM programme.proposals WHERE deleted_at IS NULL)             AS propositions_total,
    (SELECT count(*) FROM programme.proposals
      WHERE deleted_at IS NULL AND status IN ('submitted', 'under_review'))         AS propositions_a_traiter,
    (SELECT count(*) FROM programme.proposals WHERE deleted_at IS NULL AND status = 'accepted') AS propositions_acceptees,
    (SELECT count(*) FROM programme.proposals WHERE deleted_at IS NULL AND status = 'rejected') AS propositions_rejetees,
    (SELECT count(*) FROM programme.proposals
      WHERE deleted_at IS NULL AND submitted_at >= now() - interval '7 days')       AS propositions_deposees_7j,
    (SELECT count(*) FROM programme.reviews WHERE submitted_at IS NULL)             AS revues_en_cours,

    -- Programmation et inscriptions
    (SELECT count(*) FROM programme.sessions WHERE published_at IS NOT NULL AND status <> 'cancelled') AS sessions_publiees,
    (SELECT count(*) FROM programme.sessions WHERE status = 'live')                 AS sessions_en_direct,
    (SELECT count(*) FROM programme.sessions
      WHERE status IN ('planned', 'scheduled') AND starts_at BETWEEN now() AND now() + interval '7 days') AS sessions_7_prochains_jours,
    (SELECT count(*) FROM programme.registrations WHERE status <> 'cancelled')      AS inscriptions_sessions_total,
    (SELECT count(*) FROM programme.registrations
      WHERE status <> 'cancelled' AND created_at >= now() - interval '7 days')      AS inscriptions_sessions_7j,
    (SELECT count(*) FROM programme.registrations WHERE joined_at IS NOT NULL)      AS participations_effectives,

    -- Publications
    (SELECT count(*) FROM publication.articles WHERE deleted_at IS NULL AND status = 'published') AS articles_publies,
    (SELECT count(*) FROM publication.articles
      WHERE deleted_at IS NULL AND status IN ('submitted', 'in_review'))            AS articles_en_moderation,

    now() AS calcule_le;

COMMENT ON VIEW analytics.v_platform_overview IS
    'Compteurs de la page d''accueil du back-office. Vue temps réel (non matérialisée) : ces chiffres doivent refléter l''instant présent.';

-- -----------------------------------------------------------------------------
-- 11. Santé opérationnelle
--
-- LE tableau à regarder chaque matin. Il ne mesure pas l'activité métier mais la
-- bonne marche des mécanismes asynchrones sur lesquels tout le reste repose :
-- si l'outbox n'est plus relayé, les confirmations d'inscription ne partent
-- plus — et personne ne s'en aperçoit avant les réclamations des participants.
--
-- Chaque ligne porte sa valeur ET ses deux seuils : la règle d'alerte vit à
-- côté de la mesure, pas dans un script de supervision séparé qui dériverait.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW analytics.v_operational_health AS
WITH indicateurs AS (
    -- Outbox : profondeur de la file non publiée.
    SELECT
        'outbox_non_publie'::text                                   AS code,
        'Événements de domaine non publiés'::text                   AS libelle,
        'platform'::text                                            AS domaine,
        count(*)::bigint                                            AS valeur,
        100::bigint                                                 AS seuil_attention,
        1000::bigint                                                AS seuil_critique,
        jsonb_build_object(
            'plus_ancien', min(o.occurred_at),
            'tentatives_max', max(o.attempts)
        )                                                           AS detail
    FROM platform.outbox_events o
    WHERE o.published_at IS NULL

    UNION ALL

    -- Outbox bloqué : des événements qui échouent en boucle. Un seul suffit à
    -- justifier un examen, c'est le symptôme d'un consommateur cassé.
    SELECT
        'outbox_en_echec', 'Événements d''outbox en échec répété', 'platform',
        count(*)::bigint, 1::bigint, 10::bigint,
        jsonb_build_object('derniere_erreur', max(o.last_error))
    FROM platform.outbox_events o
    WHERE o.published_at IS NULL AND o.attempts >= 3

    UNION ALL

    -- File morte : travaux abandonnés après épuisement des tentatives.
    SELECT
        'travaux_file_morte', 'Travaux en file morte', 'platform',
        count(*)::bigint, 1::bigint, 25::bigint,
        jsonb_build_object(
            'taches', COALESCE(jsonb_object_agg(j.task, j.n), '{}'::jsonb)
        )
    FROM (SELECT task, count(*) AS n FROM platform.jobs WHERE status = 'dead' GROUP BY task) j

    UNION ALL

    -- Travaux en retard d'exécution : la file se creuse plus vite qu'elle ne se vide.
    SELECT
        'travaux_en_retard', 'Travaux échus non pris en charge', 'platform',
        count(*)::bigint, 50::bigint, 500::bigint,
        jsonb_build_object('plus_ancien', min(j.run_at))
    FROM platform.jobs j
    WHERE j.status = 'queued' AND j.run_at <= now() - interval '5 minutes'

    UNION ALL

    -- Travaux réservés puis jamais relâchés : un worker tué en cours de route.
    SELECT
        'travaux_bloques', 'Travaux verrouillés depuis plus de 15 minutes', 'platform',
        count(*)::bigint, 1::bigint, 10::bigint,
        jsonb_build_object('plus_ancien', min(j.locked_at), 'workers', array_agg(DISTINCT j.locked_by))
    FROM platform.jobs j
    WHERE j.status = 'running' AND j.locked_at < now() - interval '15 minutes'

    UNION ALL

    -- Rappels en retard : un rappel « J-1 » envoyé après l'activité est pire
    -- qu'un rappel non envoyé.
    SELECT
        'rappels_en_retard', 'Rappels programmés non envoyés à l''heure', 'engagement',
        count(*)::bigint, 1::bigint, 50::bigint,
        jsonb_build_object('plus_ancien', min(sr.scheduled_for))
    FROM engagement.scheduled_reminders sr
    WHERE sr.status = 'pending' AND sr.scheduled_for < now() - interval '5 minutes'

    UNION ALL

    -- Délivrabilité : rebonds et plaintes sur sept jours. Au-delà, la réputation
    -- du domaine expéditeur se dégrade pour TOUS les envois, confirmations
    -- d'inscription comprises.
    SELECT
        'emails_rebond_7j', 'Courriels en rebond ou signalés (7 jours)', 'engagement',
        count(*)::bigint, 20::bigint, 100::bigint,
        jsonb_build_object(
            'rebonds_durs', count(*) FILTER (WHERE em.bounce_kind = 'hard'),
            'plaintes',     count(*) FILTER (WHERE em.status = 'complained')
        )
    FROM engagement.email_messages em
    WHERE em.created_at >= now() - interval '7 days'
      AND em.status IN ('bounced', 'complained')

    UNION ALL

    SELECT
        'emails_en_echec', 'Courriels en échec technique (7 jours)', 'engagement',
        count(*)::bigint, 5::bigint, 50::bigint,
        jsonb_build_object('derniere_erreur', max(em.last_error))
    FROM engagement.email_messages em
    WHERE em.created_at >= now() - interval '7 days' AND em.status = 'failed'

    UNION ALL

    SELECT
        'emails_en_attente', 'Courriels en file depuis plus de 15 minutes', 'engagement',
        count(*)::bigint, 20::bigint, 200::bigint,
        jsonb_build_object('plus_ancien', min(em.created_at))
    FROM engagement.email_messages em
    WHERE em.status = 'queued' AND em.created_at < now() - interval '15 minutes'

    UNION ALL

    -- Visioconférence : réunions non poussées chez le fournisseur.
    SELECT
        'visio_reunions_desynchronisees', 'Réunions non synchronisées chez le fournisseur', 'live',
        count(*)::bigint, 1::bigint, 5::bigint,
        jsonb_build_object('derniere_erreur', max(m.last_sync_error))
    FROM live.meetings m
    WHERE m.sync_status IN ('failed', 'abandoned')

    UNION ALL

    -- Inscriptions non enregistrées chez le fournisseur : l'inscrit est acquis
    -- côté plateforme mais n'a pas reçu son lien personnel.
    SELECT
        'visio_inscriptions_desynchronisees', 'Inscriptions visio à rattraper', 'live',
        count(*)::bigint, 5::bigint, 50::bigint,
        jsonb_build_object(
            'abandonnees', count(*) FILTER (WHERE mp.sync_status = 'abandoned'),
            'derniere_erreur', max(mp.last_error)
        )
    FROM live.meeting_participants mp
    WHERE mp.sync_status IN ('failed', 'abandoned') AND mp.recovered_at IS NULL

    UNION ALL

    SELECT
        'visio_webhooks_en_echec', 'Webhooks fournisseur non traités', 'live',
        count(*)::bigint, 5::bigint, 50::bigint,
        jsonb_build_object('plus_ancien', min(w.received_at))
    FROM live.provider_webhook_events w
    WHERE w.status = 'failed'

    UNION ALL

    -- Partitions du mois prochain : oubli silencieux jusqu'au 1er du mois, où
    -- toutes les écritures tombent dans la partition par défaut et la font
    -- gonfler. Une seule partition manquante justifie une intervention.
    SELECT
        'partitions_manquantes', 'Partitions du mois prochain non créées', 'platform',
        count(*)::bigint, 1::bigint, 1::bigint,
        jsonb_build_object('tables', COALESCE(array_agg(t.schema_name || '.' || t.table_name), '{}'))
    FROM (VALUES
            ('platform',    'audit_log'),
            ('engagement',  'email_messages'),
            ('negotiation', 'channel_messages'),
            ('analytics',   'page_views')
         ) AS t(schema_name, table_name)
    WHERE NOT EXISTS (
        SELECT 1
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = t.schema_name
          AND c.relname = t.table_name || '_' || to_char(now() + interval '1 month', 'YYYYMM')
    )

    UNION ALL

    -- Fraîcheur des tableaux de bord : âge, en minutes, du dernier
    -- rafraîchissement complet réussi.
    SELECT
        'analytique_perimee', 'Minutes depuis le dernier rafraîchissement analytique', 'analytics',
        COALESCE(
            (EXTRACT(EPOCH FROM (now() - max(rl.finished_at))) / 60)::bigint,
            99999::bigint
        ),
        120::bigint, 1440::bigint,
        jsonb_build_object('dernier_succes', max(rl.finished_at))
    FROM analytics.refresh_log rl
    WHERE rl.succeeded
)
SELECT
    i.code,
    i.libelle,
    i.domaine,
    i.valeur,
    i.seuil_attention,
    i.seuil_critique,
    CASE
        WHEN i.valeur >= i.seuil_critique  THEN 'critique'
        WHEN i.valeur >= i.seuil_attention THEN 'attention'
        ELSE 'ok'
    END AS gravite,
    i.detail,
    now() AS mesure_le
FROM indicateurs i
ORDER BY
    CASE
        WHEN i.valeur >= i.seuil_critique  THEN 0
        WHEN i.valeur >= i.seuil_attention THEN 1
        ELSE 2
    END,
    i.code;

COMMENT ON VIEW analytics.v_operational_health IS
    'Santé opérationnelle temps réel : outbox, file de travaux, rappels, synchronisation visio, délivrabilité, partitions et fraîcheur analytique. Une ligne par indicateur, avec ses seuils.';
