-- Lot 10 : référentiels.
--
-- Pays : non repris, rapprochés par code ISO de la v2 semée (carte du cœur § 3).
-- Vocabulaires : les valeurs des ENUM v1 utilisées par les lots suivants
-- deviennent des termes de reference.taxonomy_terms (données, aucun schéma) :
--   - activity_theme : quatre thématiques sans terme v2, `finance` = `climate_finance` (D6) ;
--   - activity_type  : vocabulaire nouveau (`side_event`, `country_day`, `other`),
--     rattaché aux propositions par entity_terms comme les thématiques ;
--   - document_type  : `additional_resource` (texte libre de activity_documents.types,
--     sans ENUM : pas de ligne dans id_map, le lot 50 lit le code).
-- id_map : public.countries, et un ENUM sous son nom qualifié (public.activity_theme,
-- public.activity_type), legacy_id = la valeur.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-10', true) \gset lot10_

-- Pays -----------------------------------------------------------------------

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'public.countries', c.id::text, 'reference', 'countries', rc.id,
       CASE WHEN rc.id IS NULL THEN 'skipped' ELSE 'merged' END,
       CASE WHEN rc.id IS NULL
            THEN 'code ISO ' || c.code || ' absent du référentiel v2'
            ELSE 'pays semé en v2, rapproché par code ISO' END
FROM v1.countries c
LEFT JOIN reference.countries rc ON rc.iso2 = upper(btrim(c.code));

-- Un pays écarté devient un pays nul dans les lots suivants : on le dit ici.
DO $$
DECLARE
    col record;
    n bigint;
BEGIN
    FOR col IN
        SELECT table_name, column_name FROM information_schema.columns
        WHERE table_schema = 'v1' AND column_name LIKE '%country_id'
          AND data_type = 'uuid'
    LOOP
        EXECUTE format(
            'SELECT count(*) FROM v1.%I t JOIN legacy.id_map m
               ON m.legacy_table = ''public.countries'' AND m.legacy_id = t.%I::text
              AND m.disposition = ''skipped''', col.table_name, col.column_name)
        INTO n;
        IF n > 0 THEN
            RAISE NOTICE 'lot 10 : % ligne(s) de %.% désignent un pays écarté', n, col.table_name, col.column_name;
        END IF;
    END LOOP;
END $$;

-- Thématiques ----------------------------------------------------------------

-- Libellés de la v1 (epavillonvue, locales fr et en). Couleurs prises dans les
-- teintes que les dix-sept autres laissent libres, à l'écart du rouge (le direct).
CREATE TEMP TABLE lot10_nouveaux (taxonomy_code text, code text) ON COMMIT DROP;

WITH ins AS (
    INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, color_hex, sort_order)
    VALUES
        ('activity_theme', 'gender_youth_and_education',
         '{"fr":"Genre, jeunesse et éducation","en":"Gender, youth and education"}', '#B04FC8', 180),
        ('activity_theme', 'forests_nature_based_solutions',
         '{"fr":"Forêts et solutions fondées sur la nature","en":"Forests and nature-based solutions"}', '#2D6A4F', 190),
        ('activity_theme', 'just_energy_transition',
         '{"fr":"Transition énergétique juste","en":"Just energy transition"}', '#1F4E9C', 200),
        ('activity_theme', 'other',
         '{"fr":"Autre","en":"Other"}', '#B0B7C0', 990)
    RETURNING taxonomy_code, code
)
INSERT INTO lot10_nouveaux SELECT * FROM ins;

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'public.activity_theme', e.enumlabel, 'reference', 'taxonomy_terms', t.id,
       CASE WHEN n.code IS NOT NULL THEN 'migrated' ELSE 'merged' END,
       CASE WHEN n.code IS NOT NULL THEN NULL
            WHEN e.enumlabel = 'finance' THEN 'finance v1 = climate_finance v2 (D6)'
            ELSE 'terme semé en v2, même code' END
FROM pg_enum e
JOIN pg_type ty ON ty.oid = e.enumtypid
JOIN pg_namespace ns ON ns.oid = ty.typnamespace AND ns.nspname = 'v1'
LEFT JOIN reference.taxonomy_terms t
       ON t.taxonomy_code = 'activity_theme'
      AND t.code = CASE e.enumlabel WHEN 'finance' THEN 'climate_finance' ELSE e.enumlabel END
LEFT JOIN lot10_nouveaux n ON n.taxonomy_code = 'activity_theme' AND n.code = e.enumlabel
WHERE ty.typname = 'activity_theme';

-- Type d'activité ------------------------------------------------------------

INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system)
VALUES ('activity_type', '{"fr":"Types d''activité","en":"Activity types"}',
        '{"fr":"Place de l''activité au Pavillon : événement parallèle, journée pays","en":"Place of the activity at the Pavilion: side event, country day"}',
        false, false, false);

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('activity_type', 'side_event',  '{"fr":"Événement parallèle","en":"Side event"}', 10),
    ('activity_type', 'country_day', '{"fr":"Journée pays","en":"Country day"}', 20),
    ('activity_type', 'other',       '{"fr":"Autre","en":"Other"}', 90);

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'public.activity_type', e.enumlabel, 'reference', 'taxonomy_terms', t.id, 'migrated', NULL
FROM pg_enum e
JOIN pg_type ty ON ty.oid = e.enumtypid
JOIN pg_namespace ns ON ns.oid = ty.typnamespace AND ns.nspname = 'v1'
LEFT JOIN reference.taxonomy_terms t ON t.taxonomy_code = 'activity_type' AND t.code = e.enumlabel
WHERE ty.typname = 'activity_type';

-- Type de document -----------------------------------------------------------

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order)
VALUES ('document_type', 'additional_resource',
        '{"fr":"Ressource additionnelle","en":"Additional resource"}', 60);

-- Compte rendu ---------------------------------------------------------------

DO $$
DECLARE
    r record;
BEGIN
    IF EXISTS (SELECT 1 FROM legacy.id_map
               WHERE legacy_table IN ('public.activity_theme', 'public.activity_type')
                 AND target_id IS NULL) THEN
        RAISE EXCEPTION 'lot 10 : une valeur d''ENUM v1 n''a pas de terme v2';
    END IF;

    FOR r IN
        SELECT s.t, s.extraites,
               count(m.*) FILTER (WHERE m.disposition = 'migrated') AS migrated,
               count(m.*) FILTER (WHERE m.disposition = 'merged')   AS merged,
               count(m.*) FILTER (WHERE m.disposition = 'skipped')  AS skipped
        FROM (VALUES
            ('public.countries',      (SELECT count(*) FROM v1.countries)),
            ('public.activity_theme', (SELECT count(*) FROM pg_enum e JOIN pg_type ty ON ty.oid = e.enumtypid
                                        JOIN pg_namespace ns ON ns.oid = ty.typnamespace
                                        WHERE ns.nspname = 'v1' AND ty.typname = 'activity_theme')),
            ('public.activity_type',  (SELECT count(*) FROM pg_enum e JOIN pg_type ty ON ty.oid = e.enumtypid
                                        JOIN pg_namespace ns ON ns.oid = ty.typnamespace
                                        WHERE ns.nspname = 'v1' AND ty.typname = 'activity_type'))
        ) AS s(t, extraites)
        LEFT JOIN legacy.id_map m ON m.legacy_table = s.t
        GROUP BY s.t, s.extraites
        ORDER BY s.t
    LOOP
        RAISE NOTICE 'lot 10 : % extraites %, migrated %, merged %, skipped %',
            r.t, r.extraites, r.migrated, r.merged, r.skipped;
        IF r.extraites <> r.migrated + r.merged + r.skipped THEN
            RAISE EXCEPTION 'lot 10 : % ne rend pas compte de toutes ses lignes', r.t;
        END IF;
    END LOOP;
END $$;

COMMIT;
