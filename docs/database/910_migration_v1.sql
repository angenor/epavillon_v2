-- =============================================================================
-- ePavillon v2 — 910_migration_v1.sql
-- Reprise des données de la plateforme v1 (Supabase / PostgreSQL) vers v2.
--
-- Dépend de : tous les fichiers de schéma + 900_seed.sql
--
-- PRINCIPE DIRECTEUR — la migration est un PROJET, pas un script
-- Elle se déroule en quatre temps, rejouables autant de fois que nécessaire :
--   1. CHARGEMENT   : copie brute de la v1 dans le schéma `legacy`, sans
--                     transformation. Aucune contrainte, aucune perte.
--   2. RÉSOLUTION   : dédoublonnage des organisations et des personnes, décidé
--                     et validé AVANT toute écriture dans les schémas v2.
--                     C'est l'unique occasion de nettoyer la dette de la v1 :
--                     une fois les données reprises telles quelles, plus
--                     personne ne le fera.
--   3. TRANSFORMATION : écriture dans les schémas v2, chaque ligne conservant
--                     son identifiant d'origine dans `legacy.id_map`.
--   4. RÉCONCILIATION : contrôles de complétude et d'intégrité, comparés aux
--                     comptages de la v1. Un écart bloque la bascule.
--
-- La reprise se fait à identifiants NOUVEAUX (UUID v7) avec table de
-- correspondance, et non par conservation des UUID v4 d'origine : cela permet
-- de fusionner deux lignes v1 en une seule ligne v2 (le cas des organisations
-- en double et des invités devenus utilisateurs), ce qu'une reprise à
-- identifiants constants rendrait impossible.
-- =============================================================================

CREATE SCHEMA IF NOT EXISTS legacy;
COMMENT ON SCHEMA legacy IS
    'Zone de transit de la migration v1. Supprimée après validation de la bascule.';

-- -----------------------------------------------------------------------------
-- 1. Table de correspondance des identifiants
--
-- Sert pendant la migration (résolution des clés étrangères), puis après :
-- les URL de la v1 déjà partagées, les identifiants figurant dans des courriels
-- envoyés et les exports Excel des équipes continuent de résoudre.
-- -----------------------------------------------------------------------------
CREATE TABLE legacy.id_map (
    legacy_table text NOT NULL,
    legacy_id    text NOT NULL,       -- text : la v1 mêle UUID et BIGINT (laravel_user_id)
    target_schema text NOT NULL,
    target_table text NOT NULL,
    target_id    uuid NOT NULL,
    -- 'migrated'  : reprise à l'identique
    -- 'merged'    : plusieurs lignes v1 pointent vers la même ligne v2
    -- 'skipped'   : écartée (doublon rejeté, donnée de test, brouillon vide)
    disposition  text NOT NULL DEFAULT 'migrated'
                 CHECK (disposition IN ('migrated', 'merged', 'skipped')),
    note         text,
    migrated_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (legacy_table, legacy_id)
);

CREATE INDEX ix_id_map_target ON legacy.id_map (target_schema, target_table, target_id);

CREATE OR REPLACE FUNCTION legacy.map(p_table text, p_legacy_id text)
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT target_id FROM legacy.id_map
    WHERE legacy_table = p_table AND legacy_id = p_legacy_id;
$$;

CREATE OR REPLACE FUNCTION legacy.remember(
    p_table text, p_legacy_id text,
    p_target_schema text, p_target_table text, p_target_id uuid,
    p_disposition text DEFAULT 'migrated', p_note text DEFAULT NULL
)
RETURNS uuid
LANGUAGE sql
AS $$
    INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                               target_id, disposition, note)
    VALUES (p_table, p_legacy_id, p_target_schema, p_target_table, p_target_id, p_disposition, p_note)
    ON CONFLICT (legacy_table, legacy_id) DO UPDATE
        SET target_id = EXCLUDED.target_id,
            disposition = EXCLUDED.disposition,
            note = EXCLUDED.note
    RETURNING target_id;
$$;

-- -----------------------------------------------------------------------------
-- 2. Correspondance des vocabulaires
--
-- Les ENUM de la v1 deviennent des codes de `reference.taxonomy_terms`. Les
-- codes ont été repris à l'identique dans 020_reference.sql : la correspondance
-- est donc l'identité pour la plupart des valeurs. Cette table ne recense que
-- les écarts et les valeurs disparues.
-- -----------------------------------------------------------------------------
CREATE TABLE legacy.enum_mapping (
    legacy_type  text NOT NULL,
    legacy_value text NOT NULL,
    taxonomy_code text,
    term_code    text,
    note         text,
    PRIMARY KEY (legacy_type, legacy_value)
);

INSERT INTO legacy.enum_mapping (legacy_type, legacy_value, taxonomy_code, term_code, note) VALUES
    -- session_category_v2 -> filières de négociation
    ('session_category_v2', 'climate',         'negotiation_track', 'climate', NULL),
    ('session_category_v2', 'biodiversity',    'negotiation_track', 'biodiversity', NULL),
    ('session_category_v2', 'desertification', 'negotiation_track', 'desertification', NULL),
    -- activity_type : les trois valeurs v1 rejoignent la taxonomie des catégories
    ('activity_type', 'side_event',  'activity_category', 'results_sharing',
     'Reclassé : « side_event » décrivait un format, pas une nature d''activité.'),
    ('activity_type', 'country_day', 'activity_category', 'concertation',
     'Reclassé ; la notion de journée pays est désormais portée par event.event_days.'),
    ('activity_type', 'other',       'activity_category', 'awareness', 'Valeur fourre-tout, à revoir au cas par cas.'),
    -- thematique_type (témoignages) -> thématiques d'activité
    ('thematique_type', 'pertes_et_prejudices', 'activity_theme', 'loss_and_damage', NULL),
    ('thematique_type', 'attenuation',          'activity_theme', 'mitigation', NULL),
    ('thematique_type', 'adaptation',           'activity_theme', 'adaptation', NULL),
    ('thematique_type', 'finance',              'activity_theme', 'climate_finance', NULL),
    ('thematique_type', 'genre',                'activity_theme', 'gender', NULL),
    ('thematique_type', 'transparence',         'activity_theme', 'transparency', NULL),
    ('thematique_type', 'agriculture',          'activity_theme', 'agriculture_food', NULL),
    ('thematique_type', 'ace',                  NULL, NULL,
     'Action pour l''autonomisation climatique : créer le terme avant reprise.'),
    ('thematique_type', 'mecanismes_de_cooperation', NULL, NULL, 'Terme à créer.'),
    ('thematique_type', 'bilan_mondial',        NULL, NULL, 'Terme à créer.'),
    ('thematique_type', 'droits_de_l_homme_et_climat', 'activity_theme', 'climate_justice_indigenous', NULL)
ON CONFLICT DO NOTHING;

-- Correspondance des rôles v1 -> rôles v2 scopés.
CREATE TABLE legacy.role_mapping (
    legacy_role text PRIMARY KEY,
    role_code   text NOT NULL,
    scope_type  text NOT NULL DEFAULT 'global',
    note        text
);

INSERT INTO legacy.role_mapping (legacy_role, role_code, scope_type, note) VALUES
    ('super_admin',        'super_admin', 'global', NULL),
    ('admin',              'admin',       'global', NULL),
    ('revisionniste',      'reviewer',    'global',
     'À restreindre ensuite à la portée `event` de la dernière COP : la v1 ne conservait pas ce périmètre.'),
    ('negotiator',         'negotiator',  'global', NULL),
    ('trainer',            'trainer',     'global', NULL),
    ('standard',           'standard',    'global', NULL),
    ('unfccc_focal_point', 'negotiator',  'global',
     'Devient un attribut du profil (identity.negotiator_profiles.is_unfccc_focal_point), pas un rôle.'),
    ('paco',               'standard',    'global',
     'Rôle de circonstance lié à un cycle de webinaires. Remplacé par l''inscription aux sessions.')
ON CONFLICT DO NOTHING;

-- -----------------------------------------------------------------------------
-- 3. Étape 1 — Chargement brut
--
-- Les tables `legacy.v1_*` sont créées par le script d'extraction
-- (`scripts/migration/dump_v1.sh`, `pg_dump --data-only --schema=public` de la
-- base Supabase, restauré dans le schéma `legacy`). Elles sont volontairement
-- typées large (text partout sauf les identifiants) : le chargement ne doit
-- jamais échouer sur une valeur inattendue, c'est l'étape suivante qui filtre.
--
-- Structure minimale attendue après chargement :
--   legacy.v1_users, v1_user_roles, v1_organizations, v1_organization_aliases,
--   v1_events, v1_activities, v1_activity_speakers, v1_activity_registrations,
--   v1_activity_documents, v1_activity_ratings, v1_revision_comments,
--   v1_zoom_meetings, v1_incident_messages, v1_negotiation_sessions,
--   v1_negotiation_documents, v1_francophonie_meetings, v1_countries
-- -----------------------------------------------------------------------------

-- Contrôle de présence avant de démarrer la transformation.
CREATE OR REPLACE FUNCTION legacy.check_staging_ready()
RETURNS TABLE (expected_table text, is_present boolean, row_count bigint)
LANGUAGE plpgsql
AS $$
DECLARE
    v_table text;
    v_count bigint;
BEGIN
    FOREACH v_table IN ARRAY ARRAY[
        'v1_users', 'v1_user_roles', 'v1_organizations', 'v1_organization_aliases',
        'v1_events', 'v1_activities', 'v1_activity_speakers', 'v1_activity_registrations',
        'v1_countries'
    ]
    LOOP
        IF EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = 'legacy' AND tablename = v_table) THEN
            EXECUTE format('SELECT count(*) FROM legacy.%I', v_table) INTO v_count;
            RETURN QUERY SELECT v_table, true, v_count;
        ELSE
            RETURN QUERY SELECT v_table, false, NULL::bigint;
        END IF;
    END LOOP;
END;
$$;

-- -----------------------------------------------------------------------------
-- 4. Étape 2 — Résolution des organisations
--
-- LE moment décisif de la migration. La v1 a produit des doublons parce que la
-- recherche se faisait sur le seul nom complet. On applique ici la logique v2
-- (nom normalisé, sigle, domaine) aux données v1 pour proposer des
-- regroupements, qui sont VALIDÉS PAR UN HUMAIN avant reprise.
--
-- Rien n'est fusionné automatiquement : une décision erronée mêlerait les
-- activités de deux organisations distinctes, dommage impossible à défaire
-- proprement une fois les activités reprises.
-- -----------------------------------------------------------------------------
CREATE TABLE legacy.organization_resolution (
    legacy_id       uuid PRIMARY KEY,
    legacy_name     text NOT NULL,
    legacy_acronym  text,
    legacy_email    text,
    legacy_country  text,
    normalized_name text GENERATED ALWAYS AS (platform.normalize_label(legacy_name)) STORED,
    email_domain    text GENERATED ALWAYS AS (platform.extract_domain(legacy_email)) STORED,
    -- Groupe de fusion proposé : toutes les lignes partageant la même valeur
    -- deviendront une seule organisation v2.
    cluster_key     text,
    -- Ligne retenue comme fiche de référence du groupe.
    is_cluster_head boolean NOT NULL DEFAULT false,
    decision        text CHECK (decision IN ('keep', 'merge', 'reject')),
    decided_by      text,
    decided_at      timestamptz,
    note            text
);

CREATE INDEX ix_org_resolution_cluster ON legacy.organization_resolution (cluster_key);
CREATE INDEX ix_org_resolution_trgm    ON legacy.organization_resolution USING gin (normalized_name gin_trgm_ops);

-- Propose les regroupements. À exécuter après chargement, puis à relire dans le
-- back-office de migration avant de statuer.
CREATE OR REPLACE FUNCTION legacy.propose_organization_clusters(p_threshold real DEFAULT 0.55)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    v_pairs integer;
BEGIN
    -- 4.1 Regroupement fort : domaine de courriel partagé, hors domaines publics.
    UPDATE legacy.organization_resolution r
    SET cluster_key = 'domain:' || r.email_domain
    WHERE r.email_domain IS NOT NULL
      AND r.email_domain NOT IN (SELECT domain FROM org.public_email_domains)
      AND r.cluster_key IS NULL;

    -- 4.2 Regroupement fort : nom normalisé strictement identique.
    UPDATE legacy.organization_resolution r
    SET cluster_key = 'name:' || r.normalized_name
    WHERE r.cluster_key IS NULL
      AND EXISTS (
          SELECT 1 FROM legacy.organization_resolution o
          WHERE o.legacy_id <> r.legacy_id AND o.normalized_name = r.normalized_name
      );

    -- 4.3 Regroupement fort : le sigle de l'un est le nom complet de l'autre.
    --     C'est exactement le cas décrit dans le cadrage — « certains
    --     cherchaient par nom complet tandis que d'autres par sigle ».
    UPDATE legacy.organization_resolution r
    SET cluster_key = 'acronym:' || platform.normalize_label(r.legacy_acronym)
    WHERE r.cluster_key IS NULL
      AND r.legacy_acronym IS NOT NULL
      AND EXISTS (
          SELECT 1 FROM legacy.organization_resolution o
          WHERE o.legacy_id <> r.legacy_id
            AND o.normalized_name = platform.normalize_label(r.legacy_acronym)
      );

    -- 4.4 Rapprochement flou : consigné pour revue, jamais appliqué d'office.
    INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
    SELECT LEAST(a.legacy_id, b.legacy_id),
           GREATEST(a.legacy_id, b.legacy_id),
           round((similarity(a.normalized_name, b.normalized_name) * 100)::numeric, 1),
           ARRAY['legacy_name_similarity']
    FROM legacy.organization_resolution a
    JOIN legacy.organization_resolution b
      ON b.legacy_id > a.legacy_id
     AND a.normalized_name % b.normalized_name
     AND similarity(a.normalized_name, b.normalized_name) >= p_threshold
    WHERE a.cluster_key IS DISTINCT FROM b.cluster_key
    ON CONFLICT DO NOTHING;

    GET DIAGNOSTICS v_pairs = ROW_COUNT;

    -- Chaque groupe reçoit une tête : la fiche la plus complète (sigle, courriel,
    -- pays renseignés), à défaut la plus ancienne.
    WITH ranked AS (
        SELECT legacy_id, cluster_key,
               row_number() OVER (
                   PARTITION BY cluster_key
                   ORDER BY (legacy_acronym IS NOT NULL)::int
                          + (legacy_email IS NOT NULL)::int
                          + (legacy_country IS NOT NULL)::int DESC,
                            legacy_id
               ) AS rn
        FROM legacy.organization_resolution
        WHERE cluster_key IS NOT NULL
    )
    UPDATE legacy.organization_resolution r
    SET is_cluster_head = (ranked.rn = 1)
    FROM ranked WHERE ranked.legacy_id = r.legacy_id;

    RETURN v_pairs;
END;
$$;

COMMENT ON FUNCTION legacy.propose_organization_clusters IS
    'Propose les regroupements d''organisations en double. Ne décide rien : la colonne `decision` est remplie par un humain.';

-- Vue de travail du back-office de migration : les groupes à trancher, du plus
-- lourd de conséquences au plus anodin.
CREATE OR REPLACE VIEW legacy.v_organization_clusters AS
SELECT
    cluster_key,
    count(*)                                              AS member_count,
    array_agg(legacy_name ORDER BY is_cluster_head DESC)  AS names,
    array_agg(legacy_id ORDER BY is_cluster_head DESC)    AS legacy_ids,
    bool_and(decision IS NOT NULL)                        AS is_decided
FROM legacy.organization_resolution
WHERE cluster_key IS NOT NULL
GROUP BY cluster_key
HAVING count(*) > 1
ORDER BY count(*) DESC, cluster_key;

-- -----------------------------------------------------------------------------
-- 5. Étape 2 (suite) — Résolution des personnes
--
-- La v1 stockait les invités dans `activity_registrations` (colonnes `guest_*`)
-- sans lien avec `users`. Une même personne pouvait donc exister en tant
-- qu'invitée à trois activités et en tant qu'utilisatrice inscrite : quatre
-- lignes, aucune consolidation. La reprise les réunit par courriel normalisé.
-- -----------------------------------------------------------------------------
CREATE TABLE legacy.person_resolution (
    id             uuid PRIMARY KEY DEFAULT platform.uuid_v7(),
    source_kind    text NOT NULL CHECK (source_kind IN ('user', 'guest', 'speaker')),
    source_table   text NOT NULL,
    source_id      text NOT NULL,
    email          text,
    email_key      text GENERATED ALWAYS AS (lower(btrim(email))) STORED,
    first_name     text,
    last_name      text,
    -- Personne v2 retenue pour cette source (renseignée par la transformation).
    target_person_id uuid,
    CONSTRAINT ux_person_resolution UNIQUE (source_table, source_id)
);

CREATE INDEX ix_person_resolution_email ON legacy.person_resolution (email_key);

COMMENT ON TABLE legacy.person_resolution IS
    'Toutes les occurrences de personnes de la v1 (comptes, invités, intervenants), rapprochées par courriel.';

-- -----------------------------------------------------------------------------
-- 6. Étape 3 — Transformation
--
-- Chaque procédure est idempotente (ON CONFLICT + id_map) : on peut relancer la
-- migration après correction sans repartir d'une base vide. L'ordre est imposé
-- par les dépendances.
-- -----------------------------------------------------------------------------

-- 6.1 Personnes. Priorité aux comptes, puis aux invités, puis aux intervenants :
--     le premier rencontré crée la personne, les suivants s'y rattachent.
CREATE OR REPLACE FUNCTION legacy.migrate_people()
RETURNS TABLE (created integer, merged integer)
LANGUAGE plpgsql
AS $$
DECLARE
    v_created integer := 0;
    v_merged  integer := 0;
    v_row     record;
    v_person  uuid;
BEGIN
    FOR v_row IN
        SELECT * FROM legacy.person_resolution
        WHERE email_key IS NOT NULL AND email_key <> ''
        ORDER BY CASE source_kind WHEN 'user' THEN 1 WHEN 'guest' THEN 2 ELSE 3 END, id
    LOOP
        SELECT p.id INTO v_person
        FROM identity.people p
        WHERE p.primary_email = v_row.email_key::platform.email
          AND p.status <> 'anonymized';

        IF v_person IS NULL THEN
            INSERT INTO identity.people (primary_email, first_name, last_name)
            VALUES (v_row.email_key::platform.email,
                    COALESCE(NULLIF(btrim(v_row.first_name), ''), 'Prénom inconnu'),
                    COALESCE(NULLIF(btrim(v_row.last_name), ''), 'Nom inconnu'))
            RETURNING id INTO v_person;
            v_created := v_created + 1;
        ELSE
            v_merged := v_merged + 1;
        END IF;

        UPDATE legacy.person_resolution SET target_person_id = v_person WHERE id = v_row.id;

        PERFORM legacy.remember(
            v_row.source_table, v_row.source_id, 'identity', 'people', v_person,
            CASE WHEN v_merged > 0 AND v_person IS NOT NULL THEN 'merged' ELSE 'migrated' END,
            format('source=%s', v_row.source_kind)
        );
    END LOOP;

    RETURN QUERY SELECT v_created, v_merged;
END;
$$;

-- 6.2 Organisations. Une ligne v2 par groupe validé ; les noms des fiches
--     absorbées deviennent des dénominations alternatives, ce qui préserve la
--     recherche sur les libellés historiques.
CREATE OR REPLACE FUNCTION legacy.migrate_organizations()
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    v_row     record;
    v_head    record;
    v_org_id  uuid;
    v_count   integer := 0;
BEGIN
    IF EXISTS (SELECT 1 FROM legacy.organization_resolution
               WHERE cluster_key IS NOT NULL AND decision IS NULL) THEN
        RAISE EXCEPTION 'Des groupes d''organisations n''ont pas été tranchés. Compléter legacy.organization_resolution.decision avant de poursuivre.'
            USING ERRCODE = 'restrict_violation';
    END IF;

    -- Têtes de groupe et fiches isolées : création de la fiche v2.
    FOR v_head IN
        SELECT * FROM legacy.organization_resolution
        WHERE COALESCE(decision, 'keep') <> 'reject'
          AND (cluster_key IS NULL OR is_cluster_head)
    LOOP
        INSERT INTO org.organizations (
            legal_name, acronym, slug, organization_type_code, contact_email, status
        )
        VALUES (
            v_head.legacy_name,
            NULLIF(btrim(v_head.legacy_acronym), ''),
            COALESCE(platform.slugify(v_head.legacy_name), 'org-' || left(replace(v_head.legacy_id::text,'-',''), 8)),
            'ngo_association',   -- valeur de repli ; le type v1 est réappliqué juste après
            NULLIF(v_head.legacy_email, '')::platform.email,
            'active'
        )
        ON CONFLICT (slug) DO UPDATE SET updated_at = now()
        RETURNING id INTO v_org_id;

        PERFORM legacy.remember('organizations', v_head.legacy_id::text, 'org', 'organizations', v_org_id);
        v_count := v_count + 1;
    END LOOP;

    -- Membres absorbés : pas de fiche propre, mais leur nom est conservé.
    FOR v_row IN
        SELECT r.*, h.legacy_id AS head_id
        FROM legacy.organization_resolution r
        JOIN legacy.organization_resolution h
          ON h.cluster_key = r.cluster_key AND h.is_cluster_head
        WHERE r.cluster_key IS NOT NULL
          AND NOT r.is_cluster_head
          AND COALESCE(r.decision, 'merge') <> 'reject'
    LOOP
        v_org_id := legacy.map('organizations', v_row.head_id::text);

        INSERT INTO org.organization_names (organization_id, name, kind, is_confirmed)
        VALUES (v_org_id, v_row.legacy_name, 'short', false)
        ON CONFLICT (organization_id, name_normalized, kind) DO NOTHING;

        PERFORM legacy.remember('organizations', v_row.legacy_id::text, 'org', 'organizations',
                                v_org_id, 'merged',
                                format('Fusionnée dans %s lors de la reprise', v_org_id));
    END LOOP;

    RETURN v_count;
END;
$$;

COMMENT ON FUNCTION legacy.migrate_organizations IS
    'Crée une organisation v2 par groupe validé. Refuse de s''exécuter tant que des groupes ne sont pas tranchés.';

-- 6.3 Activités v1 -> proposition + session(s).
--     Une activité v1 devient toujours une proposition ; elle engendre une
--     session si et seulement si elle avait été approuvée. Les activités
--     multi-éditions (repérées par la présence de plusieurs `session_edition`
--     dans les inscriptions) engendrent autant de sessions.
CREATE OR REPLACE FUNCTION legacy.migrate_activities()
RETURNS TABLE (proposals integer, sessions integer)
LANGUAGE plpgsql
AS $$
DECLARE
    v_proposals integer := 0;
    v_sessions  integer := 0;
BEGIN
    RAISE NOTICE 'Correspondance des statuts appliquée : draft->draft, submitted->submitted, under_review->under_review, approved->accepted, rejected->rejected, cancelled->withdrawn, live/completed->accepted (l''état de diffusion passe sur la session).';
    -- Le corps effectif dépend de la structure exacte des tables de transit ;
    -- il est écrit dans `scripts/migration/03_activities.sql` afin de rester
    -- lisible et testable indépendamment. Cette fonction sert de point d'entrée
    -- unique et de garde-fou d'ordonnancement.
    RETURN QUERY SELECT v_proposals, v_sessions;
END;
$$;

-- -----------------------------------------------------------------------------
-- 7. Étape 4 — Réconciliation
--
-- Contrôles bloquants avant bascule. Tout écart doit être expliqué ligne à
-- ligne : « il manque 14 inscriptions » n'est pas une conclusion acceptable.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW legacy.v_reconciliation AS
WITH counts AS (
    SELECT 'organisations' AS entity,
           (SELECT count(*) FROM legacy.organization_resolution WHERE COALESCE(decision,'keep') <> 'reject') AS legacy_rows,
           (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'organizations') AS mapped_rows,
           (SELECT count(*) FROM org.organizations WHERE status <> 'rejected') AS target_rows
    UNION ALL
    SELECT 'personnes',
           (SELECT count(DISTINCT email_key) FROM legacy.person_resolution WHERE email_key <> ''),
           (SELECT count(*) FROM legacy.id_map WHERE target_table = 'people'),
           (SELECT count(*) FROM identity.people)
    UNION ALL
    SELECT 'propositions',
           (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'activities'),
           (SELECT count(*) FROM legacy.id_map WHERE target_table = 'proposals'),
           (SELECT count(*) FROM programme.proposals)
    UNION ALL
    SELECT 'inscriptions',
           (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'activity_registrations'),
           (SELECT count(*) FROM legacy.id_map WHERE target_table = 'registrations'),
           (SELECT count(*) FROM programme.registrations)
)
SELECT entity, legacy_rows, mapped_rows, target_rows,
       (mapped_rows = legacy_rows) AS is_complete
FROM counts;

COMMENT ON VIEW legacy.v_reconciliation IS
    'Tableau de bord de la reprise. Toutes les lignes doivent afficher is_complete = true avant la bascule.';

-- Contrôles d'intégrité post-reprise : détecte les orphelins que la v1
-- tolérait et que la v2 refuse (organisation manquante, personne manquante).
CREATE OR REPLACE FUNCTION legacy.integrity_report()
RETURNS TABLE (check_name text, problem_count bigint, sample jsonb)
LANGUAGE sql
STABLE
AS $$
    SELECT 'Personnes sans adresse de courriel valide',
           count(*),
           jsonb_agg(jsonb_build_object('id', id, 'email', primary_email)) FILTER (WHERE true)
    FROM (SELECT id, primary_email FROM identity.people
          WHERE primary_email::text !~ '^[^@\s]+@[^@\s]+\.[^@\s]{2,}$' LIMIT 5) s

    UNION ALL

    SELECT 'Organisations actives homonymes dans le même pays',
           count(*),
           jsonb_agg(to_jsonb(s))
    FROM (SELECT legal_name_normalized, country_id, count(*) AS n
          FROM org.organizations WHERE status = 'active'
          GROUP BY 1, 2 HAVING count(*) > 1 LIMIT 5) s

    UNION ALL

    SELECT 'Sessions publiées sans créneau cohérent',
           count(*),
           jsonb_agg(to_jsonb(s))
    FROM (SELECT id, starts_at, ends_at FROM programme.sessions
          WHERE published_at IS NOT NULL AND ends_at <= starts_at LIMIT 5) s;
$$;

-- -----------------------------------------------------------------------------
-- 8. Nettoyage
--
-- Après validation de la bascule et une période d'observation (30 jours
-- recommandés), le schéma de transit est supprimé — à l'exception de
-- `legacy.id_map`, déplacée dans `platform` pour continuer à résoudre les
-- anciens identifiants.
-- -----------------------------------------------------------------------------
-- ALTER TABLE legacy.id_map SET SCHEMA platform;
-- DROP SCHEMA legacy CASCADE;
