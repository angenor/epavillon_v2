-- =============================================================================
-- ePavillon v2 — 010_platform.sql
-- Noyau technique : journal d'audit, outbox transactionnel, file de travaux,
-- réglages, drapeaux de fonctionnalités, registre des modules.
--
-- Dépend de : 000_bootstrap.sql
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Registre des modules
--
-- Rend explicite, en base, la cartographie « module -> schéma -> état ».
-- Sert de source de vérité au démarrage de l'API : un module désactivé n'est pas
-- monté dans le routeur Actix, et un module marqué `external` bascule vers un
-- client HTTP au lieu de l'accès direct au schéma.
-- -----------------------------------------------------------------------------
CREATE TYPE platform.module_deployment AS ENUM (
    'embedded',   -- servi par le monolithe (défaut)
    'external',   -- extrait : appelé via HTTP/gRPC, schéma en lecture seule
    'disabled'    -- non monté
);

CREATE TABLE platform.modules (
    code              text PRIMARY KEY,
    schema_name       text        NOT NULL UNIQUE,
    display_name      platform.i18n_text NOT NULL,
    deployment        platform.module_deployment NOT NULL DEFAULT 'embedded',
    base_url          platform.url,
    depends_on        text[]      NOT NULL DEFAULT '{}',
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_modules_external_needs_url
        CHECK (deployment <> 'external' OR base_url IS NOT NULL)
);

CREATE TRIGGER tg_modules_updated_at
    BEFORE UPDATE ON platform.modules
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE platform.modules IS
    'Cartographie des modules et de leur mode de déploiement. Lue au démarrage de l''API pour monter les routes.';

-- -----------------------------------------------------------------------------
-- 2. Journal d'audit
--
-- Partitionné par mois : la table la plus volumineuse de la base ne doit jamais
-- pénaliser les écritures métier, et la purge réglementaire se fait par DROP
-- PARTITION plutôt que par DELETE massif.
--
-- L'audit ne porte aucune FK : il doit survivre à la suppression des entités
-- auditées et à l'extraction d'un module.
-- -----------------------------------------------------------------------------
CREATE TABLE platform.audit_log (
    id             uuid        NOT NULL DEFAULT platform.uuid_v7(),
    occurred_at    timestamptz NOT NULL DEFAULT now(),
    actor_id       uuid,                       -- identity.people.id (sans FK, volontairement)
    actor_label    text,                       -- dénormalisé : reste lisible après anonymisation RGPD
    request_id     text,
    entity_schema  text        NOT NULL,
    entity_table   text        NOT NULL,
    entity_id      uuid,
    action         text        NOT NULL CHECK (action IN ('insert', 'update', 'delete')),
    changed_fields text[],
    old_data       jsonb,
    new_data       jsonb,
    PRIMARY KEY (occurred_at, id)
) PARTITION BY RANGE (occurred_at);

CREATE INDEX ix_audit_log_entity ON platform.audit_log (entity_schema, entity_table, entity_id, occurred_at DESC);
CREATE INDEX ix_audit_log_actor  ON platform.audit_log (actor_id, occurred_at DESC) WHERE actor_id IS NOT NULL;

COMMENT ON TABLE platform.audit_log IS
    'Journal d''audit partitionné par mois. Alimenté par platform.tg_audit(), sans clé étrangère pour rester indépendant.';

-- Partition « fourre-tout » : garantit qu'aucune écriture ne peut échouer si le
-- worker de maintenance a pris du retard.
CREATE TABLE platform.audit_log_default PARTITION OF platform.audit_log DEFAULT;

-- Création idempotente d'une partition mensuelle (appelée par le worker).
CREATE OR REPLACE FUNCTION platform.ensure_month_partition(
    p_parent_schema text,
    p_parent_table  text,
    p_month         date
)
RETURNS text
LANGUAGE plpgsql
AS $$
DECLARE
    v_start   date := date_trunc('month', p_month)::date;
    v_end     date := (date_trunc('month', p_month) + interval '1 month')::date;
    v_name    text := format('%s_%s', p_parent_table, to_char(v_start, 'YYYYMM'));
    v_default text;
    v_keycol  text;
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = p_parent_schema AND c.relname = v_name
    ) THEN
        RETURN v_name;
    END IF;

    -- La partition « fourre-tout » peut déjà contenir des lignes du mois visé :
    -- PostgreSQL refuse alors la création de la partition définitive. On la
    -- détache le temps de créer la nouvelle partition, on y déplace les lignes
    -- concernées, puis on la rattache.
    SELECT c.relname INTO v_default
    FROM pg_class c
    JOIN pg_inherits i ON i.inhrelid = c.oid
    JOIN pg_class p    ON p.oid = i.inhparent
    JOIN pg_namespace n ON n.oid = p.relnamespace
    WHERE n.nspname = p_parent_schema
      AND p.relname = p_parent_table
      AND pg_get_expr(c.relpartbound, c.oid) = 'DEFAULT';

    IF v_default IS NULL THEN
        EXECUTE format(
            'CREATE TABLE %I.%I PARTITION OF %I.%I FOR VALUES FROM (%L) TO (%L)',
            p_parent_schema, v_name, p_parent_schema, p_parent_table, v_start, v_end
        );
        RETURN v_name;
    END IF;

    SELECT a.attname INTO v_keycol
    FROM pg_partitioned_table pt
    JOIN pg_class p     ON p.oid = pt.partrelid
    JOIN pg_namespace n ON n.oid = p.relnamespace
    JOIN pg_attribute a ON a.attrelid = p.oid AND a.attnum = pt.partattrs[0]
    WHERE n.nspname = p_parent_schema AND p.relname = p_parent_table;

    EXECUTE format('ALTER TABLE %I.%I DETACH PARTITION %I.%I',
                   p_parent_schema, p_parent_table, p_parent_schema, v_default);

    EXECUTE format(
        'CREATE TABLE %I.%I PARTITION OF %I.%I FOR VALUES FROM (%L) TO (%L)',
        p_parent_schema, v_name, p_parent_schema, p_parent_table, v_start, v_end
    );

    EXECUTE format(
        'WITH deplacees AS (DELETE FROM %I.%I WHERE %I >= %L AND %I < %L RETURNING *)
         INSERT INTO %I.%I SELECT * FROM deplacees',
        p_parent_schema, v_default, v_keycol, v_start, v_keycol, v_end,
        p_parent_schema, p_parent_table
    );

    EXECUTE format('ALTER TABLE %I.%I ATTACH PARTITION %I.%I DEFAULT',
                   p_parent_schema, p_parent_table, p_parent_schema, v_default);

    RETURN v_name;
END;
$$;

COMMENT ON FUNCTION platform.ensure_month_partition(text, text, date) IS
    'Crée si nécessaire la partition mensuelle d''une table partitionnée par RANGE sur un timestamptz.';

-- Trigger d'audit générique. À attacher explicitement, table par table, aux
-- seules entités dont la traçabilité a une valeur métier ou réglementaire :
--   CREATE TRIGGER tg_audit AFTER INSERT OR UPDATE OR DELETE ON <table>
--     FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE OR REPLACE FUNCTION platform.tg_audit()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = platform, pg_temp
AS $$
DECLARE
    v_old      jsonb;
    v_new      jsonb;
    v_changed  text[];
    v_entity   uuid;
BEGIN
    IF TG_OP = 'INSERT' THEN
        v_new := to_jsonb(NEW);
    ELSIF TG_OP = 'UPDATE' THEN
        v_old := to_jsonb(OLD);
        v_new := to_jsonb(NEW);
        SELECT array_agg(key ORDER BY key) INTO v_changed
        FROM jsonb_each(v_new)
        WHERE v_new -> key IS DISTINCT FROM v_old -> key;
        -- Aucune modification effective : ne pas polluer le journal.
        IF v_changed IS NULL THEN
            RETURN NULL;
        END IF;
    ELSE
        v_old := to_jsonb(OLD);
    END IF;

    v_entity := COALESCE(v_new ->> 'id', v_old ->> 'id')::uuid;

    INSERT INTO platform.audit_log (
        actor_id, request_id, entity_schema, entity_table, entity_id,
        action, changed_fields, old_data, new_data
    )
    VALUES (
        platform.current_actor_id(),
        platform.current_request_id(),
        TG_TABLE_SCHEMA,
        TG_TABLE_NAME,
        v_entity,
        lower(TG_OP),
        v_changed,
        v_old,
        v_new
    );

    RETURN NULL; -- trigger AFTER : la valeur de retour est ignorée
END;
$$;

-- Historique lisible d'une entité : dépile `changed_fields` en une ligne par
-- champ modifié, avec ancienne et nouvelle valeur.
--
-- La v1 maintenait une table dédiée `activity_modifications` alimentée à la
-- main par le code applicatif : elle ne couvrait que les activités, et
-- seulement les écritures qui passaient par le bon chemin. Ici, l'historique
-- est un SOUS-PRODUIT de l'audit : toute table portant le trigger
-- `platform.tg_audit()` obtient gratuitement son historique complet, y compris
-- pour les corrections faites en console.
--
-- Le filtre par entité s'appuie sur ix_audit_log_entity ; la fenêtre temporelle
-- permet en plus l'élagage des partitions.
CREATE OR REPLACE FUNCTION platform.entity_history(
    p_schema         text,
    p_table          text,
    p_entity_id      uuid,
    p_exclude_fields text[] DEFAULT ARRAY['updated_at', 'search_vector', 'view_count'],
    p_since          timestamptz DEFAULT NULL
)
RETURNS TABLE (
    occurred_at timestamptz,
    actor_id    uuid,
    actor_label text,
    action      text,
    field       text,
    old_value   jsonb,
    new_value   jsonb
)
LANGUAGE sql
STABLE
AS $$
    SELECT a.occurred_at,
           a.actor_id,
           a.actor_label,
           a.action,
           f.field,
           a.old_data -> f.field,
           a.new_data -> f.field
    FROM platform.audit_log a
    LEFT JOIN LATERAL unnest(COALESCE(a.changed_fields, '{}')) AS f(field) ON true
    WHERE a.entity_schema = p_schema
      AND a.entity_table = p_table
      AND a.entity_id = p_entity_id
      AND (p_since IS NULL OR a.occurred_at >= p_since)
      AND (f.field IS NULL OR NOT (f.field = ANY (p_exclude_fields)))
    ORDER BY a.occurred_at DESC, f.field;
$$;

COMMENT ON FUNCTION platform.entity_history IS
    'Historique des modifications d''une entité, champ par champ. Remplace la table activity_modifications de la v1.';

-- -----------------------------------------------------------------------------
-- 3. Outbox transactionnel
--
-- Pièce maîtresse du « microservice ready ». Un changement d'état métier et
-- l'événement qui l'annonce sont écrits dans la MÊME transaction : impossible
-- d'avoir un email envoyé pour une activité non validée, ou une validation sans
-- notification. Un relais lit les lignes non publiées (FOR UPDATE SKIP LOCKED)
-- et les remet aux consommateurs — aujourd'hui des handlers in-process, demain
-- un bus (NATS/Kafka) sans changer une ligne du code métier.
-- -----------------------------------------------------------------------------
CREATE TABLE platform.outbox_events (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    aggregate_schema  text        NOT NULL,
    aggregate_type    text        NOT NULL,
    aggregate_id      uuid        NOT NULL,
    event_type        text        NOT NULL,     -- ex. 'programme.proposal.approved'
    event_version     smallint    NOT NULL DEFAULT 1,
    payload           jsonb       NOT NULL,
    metadata          jsonb       NOT NULL DEFAULT '{}'::jsonb,
    correlation_id    text,
    occurred_at       timestamptz NOT NULL DEFAULT now(),
    available_at      timestamptz NOT NULL DEFAULT now(),
    published_at      timestamptz,
    attempts          smallint    NOT NULL DEFAULT 0,
    last_error        text,
    CONSTRAINT ck_outbox_event_type_format CHECK (event_type ~ '^[a-z_]+\.[a-z_]+\.[a-z_]+$')
);

-- Index partiel : la file « à publier » reste minuscule même avec des millions
-- d'événements historisés.
CREATE INDEX ix_outbox_pending
    ON platform.outbox_events (available_at, id)
    WHERE published_at IS NULL;

CREATE INDEX ix_outbox_aggregate
    ON platform.outbox_events (aggregate_schema, aggregate_type, aggregate_id, occurred_at DESC);

COMMENT ON TABLE platform.outbox_events IS
    'Outbox transactionnel : événements de domaine écrits avec le changement d''état. Relayés par le worker.';

-- Publication d'un événement depuis une fonction ou un trigger métier.
CREATE OR REPLACE FUNCTION platform.emit_event(
    p_aggregate_schema text,
    p_aggregate_type   text,
    p_aggregate_id     uuid,
    p_event_type       text,
    p_payload          jsonb DEFAULT '{}'::jsonb,
    p_metadata         jsonb DEFAULT '{}'::jsonb
)
RETURNS uuid
LANGUAGE plpgsql
AS $$
DECLARE
    v_id uuid;
BEGIN
    INSERT INTO platform.outbox_events (
        aggregate_schema, aggregate_type, aggregate_id, event_type,
        payload, metadata, correlation_id
    )
    VALUES (
        p_aggregate_schema, p_aggregate_type, p_aggregate_id, p_event_type,
        p_payload,
        p_metadata || jsonb_build_object('actor_id', platform.current_actor_id()),
        platform.current_request_id()
    )
    RETURNING id INTO v_id;

    -- Réveil immédiat du relais : évite d'attendre le prochain cycle de scrutation.
    PERFORM pg_notify('platform_outbox', v_id::text);
    RETURN v_id;
END;
$$;

-- Idempotence côté consommateur : un même événement rejoué (redémarrage du
-- worker, redistribution du bus) ne doit produire ses effets qu'une fois.
CREATE TABLE platform.inbox_events (
    consumer     text        NOT NULL,
    event_id     uuid        NOT NULL,
    processed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (consumer, event_id)
);

COMMENT ON TABLE platform.inbox_events IS
    'Garde-fou d''idempotence : trace (consommateur, événement) déjà traité.';

-- -----------------------------------------------------------------------------
-- 4. File de travaux
--
-- PostgreSQL sert de courtier en phase 1 (SKIP LOCKED) : pas de Redis, pas de
-- RabbitMQ à exploiter tant que la charge ne le justifie pas. La table porte
-- déjà tout ce qu'il faut pour migrer vers un vrai broker (clé d'unicité,
-- planification, backoff, file morte).
-- -----------------------------------------------------------------------------
CREATE TYPE platform.job_status AS ENUM ('queued', 'running', 'succeeded', 'failed', 'dead', 'cancelled');

CREATE TABLE platform.jobs (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    queue           text        NOT NULL DEFAULT 'default',
    task            text        NOT NULL,          -- ex. 'engagement.send_reminder'
    payload         jsonb       NOT NULL DEFAULT '{}'::jsonb,
    -- Clé d'unicité métier : empêche structurellement le double envoi d'un même
    -- rappel ou la double création d'une réunion Zoom.
    idempotency_key text,
    status          platform.job_status NOT NULL DEFAULT 'queued',
    priority        smallint    NOT NULL DEFAULT 100,
    run_at          timestamptz NOT NULL DEFAULT now(),
    attempts        smallint    NOT NULL DEFAULT 0,
    max_attempts    smallint    NOT NULL DEFAULT 5,
    locked_at       timestamptz,
    locked_by       text,
    last_error      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    completed_at    timestamptz
);

CREATE UNIQUE INDEX ux_jobs_idempotency
    ON platform.jobs (task, idempotency_key)
    WHERE idempotency_key IS NOT NULL AND status <> 'cancelled';

CREATE INDEX ix_jobs_ready
    ON platform.jobs (queue, priority, run_at, id)
    WHERE status = 'queued';

CREATE INDEX ix_jobs_stuck
    ON platform.jobs (locked_at)
    WHERE status = 'running';

COMMENT ON TABLE platform.jobs IS
    'File de travaux différés (rappels email, synchronisation Zoom, variantes d''images, rafraîchissement analytique).';

-- Réservation atomique d'un lot de travaux par un worker.
CREATE OR REPLACE FUNCTION platform.claim_jobs(
    p_queue     text,
    p_worker_id text,
    p_limit     int DEFAULT 10
)
RETURNS SETOF platform.jobs
LANGUAGE sql
AS $$
    WITH claimed AS (
        SELECT id
        FROM platform.jobs
        WHERE queue = p_queue
          AND status = 'queued'
          AND run_at <= now()
        ORDER BY priority, run_at, id
        LIMIT p_limit
        FOR UPDATE SKIP LOCKED
    )
    UPDATE platform.jobs j
    SET status    = 'running',
        locked_at = now(),
        locked_by = p_worker_id,
        attempts  = j.attempts + 1
    FROM claimed
    WHERE j.id = claimed.id
    RETURNING j.*;
$$;

-- Échec d'un travail : replanification avec backoff exponentiel plafonné,
-- puis passage en file morte au-delà du nombre d'essais autorisé.
CREATE OR REPLACE FUNCTION platform.fail_job(p_job_id uuid, p_error text)
RETURNS void
LANGUAGE sql
AS $$
    UPDATE platform.jobs
    SET status     = CASE WHEN attempts >= max_attempts THEN 'dead'::platform.job_status
                          ELSE 'queued'::platform.job_status END,
        run_at     = now() + (least(power(2, attempts), 3600) || ' seconds')::interval,
        last_error = p_error,
        locked_at  = NULL,
        locked_by  = NULL
    WHERE id = p_job_id;
$$;

-- -----------------------------------------------------------------------------
-- 5. Réglages et drapeaux de fonctionnalités
--
-- La v1 codait en dur les journées thématiques et les campagnes dans le routeur
-- Vue (`/programmations/2025/journee-jeunesse`). Ici, tout ce qui varie d'une
-- édition à l'autre se pilote en base, sans redéploiement.
-- -----------------------------------------------------------------------------
CREATE TABLE platform.settings (
    key         text        PRIMARY KEY,
    value       jsonb       NOT NULL,
    description text,
    is_secret   boolean     NOT NULL DEFAULT false,
    updated_by  uuid,
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_settings_updated_at
    BEFORE UPDATE ON platform.settings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON COLUMN platform.settings.is_secret IS
    'Si vrai, la valeur n''est jamais exposée par l''API ; elle référence un secret du coffre (ex. identifiants Zoom).';

CREATE TABLE platform.feature_flags (
    key             text        PRIMARY KEY,
    description     text        NOT NULL,
    is_enabled      boolean     NOT NULL DEFAULT false,
    -- Déploiement progressif : pourcentage d'utilisateurs, ou liste explicite.
    rollout_percent smallint    NOT NULL DEFAULT 0 CHECK (rollout_percent BETWEEN 0 AND 100),
    enabled_for     uuid[]      NOT NULL DEFAULT '{}',
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_feature_flags_updated_at
    BEFORE UPDATE ON platform.feature_flags
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE OR REPLACE FUNCTION platform.is_feature_enabled(p_key text, p_person_id uuid DEFAULT NULL)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(
        (
            SELECT f.is_enabled
               AND (
                   f.rollout_percent = 100
                   OR (p_person_id IS NOT NULL AND p_person_id = ANY (f.enabled_for))
                   OR (p_person_id IS NOT NULL
                       AND ('x' || substr(md5(f.key || p_person_id::text), 1, 4))::bit(16)::int % 100 < f.rollout_percent)
               )
            FROM platform.feature_flags f
            WHERE f.key = p_key
        ),
        false
    );
$$;

-- -----------------------------------------------------------------------------
-- 6. Suivi des migrations de schéma
--
-- La v1 accumulait des fichiers SQL appliqués à la main, sans savoir lesquels
-- l'avaient été. Le migrateur (sqlx migrate / refinery côté Rust) écrit ici.
-- -----------------------------------------------------------------------------
CREATE TABLE platform.schema_migrations (
    version     text        PRIMARY KEY,
    description text        NOT NULL,
    checksum    text        NOT NULL,
    applied_at  timestamptz NOT NULL DEFAULT now(),
    applied_by  text        NOT NULL DEFAULT current_user,
    duration_ms integer
);

-- -----------------------------------------------------------------------------
-- 7. Amorçage du registre des modules
-- -----------------------------------------------------------------------------
INSERT INTO platform.modules (code, schema_name, display_name, depends_on) VALUES
    ('identity',    'identity',    '{"fr":"Identité","en":"Identity"}',            '{}'),
    ('org',         'org',         '{"fr":"Organisations","en":"Organizations"}',  '{identity}'),
    ('event',       'event',       '{"fr":"Événements","en":"Events"}',            '{org,identity}'),
    ('programme',   'programme',   '{"fr":"Programmation","en":"Programme"}',      '{event,org,identity}'),
    ('live',        'live',        '{"fr":"Direct","en":"Live"}',                  '{programme}'),
    ('publication', 'publication', '{"fr":"Publications","en":"Publications"}',    '{org,identity}'),
    ('negotiation', 'negotiation', '{"fr":"Négociations","en":"Negotiations"}',    '{identity}'),
    ('engagement',  'engagement',  '{"fr":"Engagement","en":"Engagement"}',        '{identity}'),
    ('media',       'media',       '{"fr":"Médiathèque","en":"Media"}',            '{identity}'),
    ('tool',        'tool',        '{"fr":"Outils","en":"Tools"}',                 '{identity}'),
    ('analytics',   'analytics',   '{"fr":"Analytique","en":"Analytics"}',         '{}')
ON CONFLICT (code) DO NOTHING;
