-- =============================================================================
-- ePavillon v2 — 000_bootstrap.sql
-- Socle technique : extensions, schémas (bounded contexts), rôles,
-- domaines réutilisables et fonctions utilitaires.
--
-- Ce fichier DOIT être exécuté en premier. Tous les autres en dépendent.
-- Cible : PostgreSQL 17+ (compatible 18 ; voir note sur uuidv7()).
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Extensions
-- -----------------------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS pgcrypto;      -- gen_random_uuid(), gen_random_bytes(), digest()
CREATE EXTENSION IF NOT EXISTS citext;        -- emails et identifiants insensibles à la casse
CREATE EXTENSION IF NOT EXISTS pg_trgm;       -- recherche floue (dédoublonnage des organisations)
CREATE EXTENSION IF NOT EXISTS unaccent;      -- normalisation des noms accentués
CREATE EXTENSION IF NOT EXISTS btree_gist;    -- contraintes d'exclusion sur créneaux horaires
CREATE EXTENSION IF NOT EXISTS vector;        -- embeddings (RAG négociateurs / chatbot)
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- -----------------------------------------------------------------------------
-- 2. Schémas = bounded contexts
--
-- RÈGLE D'OR « monolith first, microservice ready » :
--   un schéma = un module = une future frontière de service.
--   Toute clé étrangère traversant deux schémas métier DOIT être nommée
--   `xmod_fk_*` afin d'être identifiable et supprimable en une requête le jour
--   où le module est extrait (cf. platform.cross_module_fk_report()).
--   Les schémas `platform` et `reference` forment le noyau partagé
--   (shared kernel) : les FK vers eux sont libres et ne portent pas ce préfixe.
-- -----------------------------------------------------------------------------
CREATE SCHEMA IF NOT EXISTS platform;     -- noyau technique : audit, outbox, jobs, réglages
CREATE SCHEMA IF NOT EXISTS reference;    -- référentiels stables : pays, locales, taxonomies
CREATE SCHEMA IF NOT EXISTS identity;     -- personnes, comptes, authentification, RBAC
CREATE SCHEMA IF NOT EXISTS org;          -- organisations, alias, adhésions, fusion
CREATE SCHEMA IF NOT EXISTS event;        -- événements (COP, séries, journées), appels à propositions
CREATE SCHEMA IF NOT EXISTS programme;    -- propositions, révision, sessions programmées, inscriptions
CREATE SCHEMA IF NOT EXISTS live;         -- visioconférence, diffusion, incidents temps réel
CREATE SCHEMA IF NOT EXISTS publication;  -- articles des organisations et quotas éditoriaux
CREATE SCHEMA IF NOT EXISTS negotiation;  -- espace négociateurs : sessions, documents, canaux
CREATE SCHEMA IF NOT EXISTS engagement;   -- notifications, emails, rappels, commentaires
CREATE SCHEMA IF NOT EXISTS media;        -- objets stockés (Garage/S3), variantes, rattachements
CREATE SCHEMA IF NOT EXISTS tool;         -- outils détachables : sondages, assistants IA
CREATE SCHEMA IF NOT EXISTS analytics;    -- projections de lecture et tableaux de bord

COMMENT ON SCHEMA platform    IS 'Noyau technique partagé : audit, outbox, files de travaux, réglages.';
COMMENT ON SCHEMA reference   IS 'Référentiels partagés en lecture seule applicative : pays, locales, taxonomies.';
COMMENT ON SCHEMA identity    IS 'Module Identité : personnes, comptes, sessions, rôles et permissions scopés.';
COMMENT ON SCHEMA org         IS 'Module Organisations : référentiel unique, alias, domaines, fusion des doublons.';
COMMENT ON SCHEMA event       IS 'Module Événements : séries (COP climat/biodiversité/désertification), éditions, appels à propositions.';
COMMENT ON SCHEMA programme   IS 'Module Programmation : propositions d''activités, revue, sessions publiées, inscriptions.';
COMMENT ON SCHEMA live        IS 'Module Direct : réunions Zoom/Teams, diffusions YouTube, messages d''incident.';
COMMENT ON SCHEMA publication IS 'Module Publications : articles des organisations soumis à quota.';
COMMENT ON SCHEMA negotiation IS 'Module Négociations : sessions, documents d''aide, réunions francophones, canaux d''échange.';
COMMENT ON SCHEMA engagement  IS 'Module Engagement : notifications, emails transactionnels, rappels, commentaires.';
COMMENT ON SCHEMA media       IS 'Module Média : objets S3 (Garage), variantes d''images, rattachements polymorphes contrôlés.';
COMMENT ON SCHEMA tool        IS 'Module Outils : sondages et assistants IA, conçus pour un déploiement séparé.';
COMMENT ON SCHEMA analytics   IS 'Module Analytique : vues matérialisées et agrégats du back-office.';

-- -----------------------------------------------------------------------------
-- 3. Rôles applicatifs
--
-- L'application Actix Web se connecte avec `epavillon_app`. Les tâches de fond
-- (relais d'outbox, jobs) utilisent `epavillon_worker`. Le reporting utilise un
-- rôle strictement en lecture. Aucun de ces rôles n'est propriétaire du schéma :
-- les migrations passent par `epavillon_migrator`.
-- -----------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'epavillon_migrator') THEN
        CREATE ROLE epavillon_migrator NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'epavillon_app') THEN
        CREATE ROLE epavillon_app NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'epavillon_worker') THEN
        CREATE ROLE epavillon_worker NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'epavillon_readonly') THEN
        CREATE ROLE epavillon_readonly NOLOGIN;
    END IF;
END
$$;

-- -----------------------------------------------------------------------------
-- 4. Identifiants : UUID v7
--
-- Choix : UUID v7 (préfixe temporel) plutôt que v4. Les clés primaires restent
-- opaques et non devinables, mais deviennent quasi séquentielles : l'insertion
-- ne fragmente plus les index B-tree et le tri par clé équivaut à un tri
-- chronologique. Sur PostgreSQL 18+, remplacer le corps par `SELECT uuidv7()`.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION platform.uuid_v7()
RETURNS uuid
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    -- Remplace les 48 premiers bits d'un UUID v4 par l'horodatage Unix en
    -- millisecondes, puis force le numéro de version à 7.
    RETURN encode(
        set_bit(
            set_bit(
                overlay(
                    uuid_send(gen_random_uuid())
                    PLACING substring(
                        int8send(floor(extract(EPOCH FROM clock_timestamp()) * 1000)::bigint)
                        FROM 3
                    )
                    FROM 1 FOR 6
                ),
                52, 1
            ),
            53, 1
        ),
        'hex'
    )::uuid;
END;
$$;

COMMENT ON FUNCTION platform.uuid_v7() IS
    'Génère un UUID v7 (ordonné dans le temps). Valeur par défaut de toutes les clés primaires.';

-- Extraction de l'horodatage d'un UUID v7 : utile pour le partitionnement et
-- l'archivage sans colonne supplémentaire.
CREATE OR REPLACE FUNCTION platform.uuid_v7_timestamp(p_uuid uuid)
RETURNS timestamptz
LANGUAGE sql
IMMUTABLE
STRICT
AS $$
    SELECT to_timestamp(
        (('x' || substring(replace(p_uuid::text, '-', '') FROM 1 FOR 12))::bit(48)::bigint) / 1000.0
    );
$$;

-- -----------------------------------------------------------------------------
-- 5. Domaines réutilisables
-- -----------------------------------------------------------------------------

-- 5.1 Texte internationalisé.
-- La v1 dupliquait les colonnes (`message_fr`, `message_en`) : chaque nouvelle
-- langue imposait une migration. Ici, un champ traduisible est un objet JSON
-- `{"fr": "...", "en": "..."}`. Le français est obligatoire (langue pivot de la
-- Francophonie), les autres locales sont facultatives et ajoutables sans DDL.
CREATE OR REPLACE FUNCTION platform.is_i18n_text(p_value jsonb)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT jsonb_typeof(p_value) = 'object'
       AND p_value ? 'fr'
       AND length(trim(p_value ->> 'fr')) > 0
       AND NOT EXISTS (
           SELECT 1 FROM jsonb_each(p_value) AS e(k, v)
           WHERE jsonb_typeof(e.v) <> 'string'
              OR e.k !~ '^[a-z]{2}(-[A-Z]{2})?$'
       );
$$;

CREATE DOMAIN platform.i18n_text AS jsonb
    CHECK (VALUE IS NULL OR platform.is_i18n_text(VALUE));

COMMENT ON DOMAIN platform.i18n_text IS
    'Texte multilingue {"fr": "...", "en": "..."}. Le français est requis, les autres locales sont optionnelles.';

-- Résolution d'un texte i18n avec repli sur le français puis sur la première
-- valeur disponible : jamais de chaîne vide affichée côté Nuxt.
CREATE OR REPLACE FUNCTION platform.t(p_value platform.i18n_text, p_locale text DEFAULT 'fr')
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT COALESCE(
        p_value ->> p_locale,
        p_value ->> split_part(p_locale, '-', 1),
        p_value ->> 'fr',
        (SELECT e.value #>> '{}' FROM jsonb_each(p_value) AS e LIMIT 1)
    );
$$;

-- 5.2 Adresse email : citext + validation syntaxique unique pour toute la base.
CREATE DOMAIN platform.email AS citext
    CHECK (VALUE IS NULL OR VALUE ~ '^[^@\s]+@[^@\s]+\.[^@\s]{2,}$');

-- 5.3 Slug : segment d'URL stable, socle du référencement Nuxt.
CREATE DOMAIN platform.slug AS text
    CHECK (VALUE ~ '^[a-z0-9]+(-[a-z0-9]+)*$' AND length(VALUE) BETWEEN 2 AND 160);

-- 5.4 Identifiant de fuseau horaire IANA, validé à l'écriture.
CREATE OR REPLACE FUNCTION platform.is_valid_timezone(p_tz text)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = p_tz);
$$;

CREATE DOMAIN platform.timezone_name AS text
    CHECK (VALUE IS NULL OR platform.is_valid_timezone(VALUE));

-- 5.5 URL absolue (liens externes, YouTube, sites d'organisations).
CREATE DOMAIN platform.url AS text
    CHECK (VALUE ~ '^https?://[^\s]+$' AND length(VALUE) <= 2048);

-- -----------------------------------------------------------------------------
-- 6. Normalisation textuelle
--
-- Fondation du dédoublonnage des organisations : `unaccent` n'est pas IMMUTABLE
-- par défaut (il dépend d'un dictionnaire), on l'encapsule dans un wrapper
-- explicitement immuable afin de pouvoir l'utiliser dans des colonnes générées
-- et des index.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION platform.immutable_unaccent(p_text text)
RETURNS text
LANGUAGE sql
IMMUTABLE
STRICT
PARALLEL SAFE
AS $$
    SELECT public.unaccent('public.unaccent'::regdictionary, p_text);
$$;

-- Forme canonique d'un libellé : minuscules, sans accents, sans ponctuation,
-- espaces réduits. « Institut de la Francophonie pour le Développement Durable »
-- et « INSTITUT DE LA FRANCOPHONIE POUR LE DEVELOPPEMENT DURABLE. » convergent.
CREATE OR REPLACE FUNCTION platform.normalize_label(p_text text)
RETURNS text
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT NULLIF(
        btrim(regexp_replace(
            regexp_replace(lower(platform.immutable_unaccent(COALESCE(p_text, ''))), '[^a-z0-9]+', ' ', 'g'),
            '\s+', ' ', 'g'
        )),
        ''
    );
$$;

COMMENT ON FUNCTION platform.normalize_label(text) IS
    'Forme canonique d''un libellé (minuscules, sans accents ni ponctuation). Base du rapprochement des organisations.';

-- Génération de slug depuis un libellé quelconque.
CREATE OR REPLACE FUNCTION platform.slugify(p_text text)
RETURNS text
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT NULLIF(
        left(regexp_replace(COALESCE(platform.normalize_label(p_text), ''), ' ', '-', 'g'), 160),
        ''
    );
$$;

-- Extraction du domaine d'une adresse email ou d'une URL : signal fort de
-- rattachement à une organisation (cf. org.organization_domains).
CREATE OR REPLACE FUNCTION platform.extract_domain(p_value text)
RETURNS text
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT NULLIF(
        lower(regexp_replace(
            regexp_replace(COALESCE(p_value, ''), '^\s*(https?://)?(www\.)?', '', 'i'),
            '^[^@]*@|[/?#].*$', '', 'g'
        )),
        ''
    );
$$;

-- -----------------------------------------------------------------------------
-- 7. Contexte de requête
--
-- L'application positionne `app.actor_id` et `app.request_id` en début de
-- transaction (SET LOCAL). Les triggers d'audit et les valeurs par défaut s'en
-- servent : l'auteur d'une modification n'a plus à être passé colonne par
-- colonne, et une trace reste exploitable même pour les écritures indirectes.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION platform.current_actor_id()
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('app.actor_id', true), '')::uuid;
$$;

CREATE OR REPLACE FUNCTION platform.current_request_id()
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('app.request_id', true), '');
$$;

-- -----------------------------------------------------------------------------
-- 8. Triggers génériques
-- -----------------------------------------------------------------------------

-- 8.1 Horodatage de mise à jour.
CREATE OR REPLACE FUNCTION platform.tg_set_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

-- 8.2 Interdiction de modifier une colonne immuable (clé métier, référence
-- externe...). S'utilise avec un argument : le nom de la colonne protégée.
CREATE OR REPLACE FUNCTION platform.tg_forbid_column_update()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_column text := TG_ARGV[0];
    v_old jsonb := to_jsonb(OLD);
    v_new jsonb := to_jsonb(NEW);
BEGIN
    IF v_old -> v_column IS DISTINCT FROM v_new -> v_column THEN
        RAISE EXCEPTION 'La colonne %.% est immuable (valeur % -> %)',
            TG_TABLE_NAME, v_column, v_old ->> v_column, v_new ->> v_column
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN NEW;
END;
$$;

-- -----------------------------------------------------------------------------
-- 9. Gouvernance des frontières de modules
--
-- Rapport de conformité : toute FK reliant deux schémas métier doit s'appeler
-- `xmod_fk_*`. Le jour où un module part en microservice, la liste des liens à
-- couper est produite mécaniquement, sans relecture manuelle du schéma.
-- À brancher en test d'intégration (échec si le rapport contient une ligne
-- `is_compliant = false`).
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW platform.cross_module_fk_report AS
WITH shared_kernel AS (
    SELECT unnest(ARRAY['platform', 'reference']) AS schema_name
),
fk AS (
    SELECT
        c.conname                          AS constraint_name,
        src_ns.nspname                     AS source_schema,
        src.relname                        AS source_table,
        tgt_ns.nspname                     AS target_schema,
        tgt.relname                        AS target_table
    FROM pg_constraint c
    JOIN pg_class src        ON src.oid = c.conrelid
    JOIN pg_namespace src_ns ON src_ns.oid = src.relnamespace
    JOIN pg_class tgt        ON tgt.oid = c.confrelid
    JOIN pg_namespace tgt_ns ON tgt_ns.oid = tgt.relnamespace
    WHERE c.contype = 'f'
)
SELECT
    fk.*,
    (fk.source_schema <> fk.target_schema
        AND fk.target_schema NOT IN (SELECT schema_name FROM shared_kernel)) AS is_cross_module,
    CASE
        WHEN fk.source_schema = fk.target_schema THEN true
        WHEN fk.target_schema IN (SELECT schema_name FROM shared_kernel) THEN true
        ELSE fk.constraint_name LIKE 'xmod\_fk\_%'
    END AS is_compliant
FROM fk;

COMMENT ON VIEW platform.cross_module_fk_report IS
    'Audit des frontières de modules : liste des FK inter-schémas et conformité de nommage (xmod_fk_*).';

-- Script de découplage : génère les ordres SQL de suppression des FK d'un
-- module donné, préalable à son extraction en service autonome.
CREATE OR REPLACE FUNCTION platform.generate_module_decoupling_script(p_schema text)
RETURNS TABLE (statement text)
LANGUAGE sql
STABLE
AS $$
    SELECT format('ALTER TABLE %I.%I DROP CONSTRAINT %I;', source_schema, source_table, constraint_name)
    FROM platform.cross_module_fk_report
    WHERE is_cross_module
      AND (source_schema = p_schema OR target_schema = p_schema);
$$;

COMMENT ON FUNCTION platform.generate_module_decoupling_script(text) IS
    'Produit les instructions de suppression des FK inter-modules pour extraire un schéma en microservice.';
